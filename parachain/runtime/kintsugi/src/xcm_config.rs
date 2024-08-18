use super::*;
use codec::{Decode, Encode};
use cumulus_primitives_core::ParaId;
use frame_support::{
    parameter_types,
    traits::{Everything, Get, Nothing, OriginTrait},
};
use orml_asset_registry::{AssetRegistryTrader, FixedRateAssetRegistryTrader};
use orml_traits::{
    location::AbsoluteReserveProvider, parameter_type_with_key, FixedConversionRateProvider, MultiCurrency,
};
use orml_xcm_support::{DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use runtime_common::Transactless;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::TrailingZeroInput;
use sp_std::{borrow::Borrow, marker::PhantomData};
use xcm::latest::{prelude::*, Weight};
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
    EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, ParentIsPreset, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit, WithComputedOrigin,
};
use xcm_executor::{
    traits::{Convert as XcmConvert, ConvertOrigin, WithOriginFilter},
    XcmExecutor,
};
use CurrencyId::ForeignAsset;
parameter_types! {
    pub const ParentLocation: MultiLocation = MultiLocation::parent();
    pub const ParentNetwork: NetworkId = NetworkId::Kusama;
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Means for transacting assets on this chain.
type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<ParentNetwork, AccountId>,
    // Mapping Tinkernet multisig to the correctly derived AccountId.
    TinkernetMultisigAsAccountId<AccountId>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<ParentNetwork, RuntimeOrigin>,
    // Derives signed AccountId origins for Tinkernet multisigs.
    TinkernetMultisigAsNativeOrigin<RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);

pub type Barrier = (
    Transactless<(
        TakeWeightCredit,
        AllowTopLevelPaidExecutionFrom<Everything>,
        AllowKnownQueryResponses<PolkadotXcm>,
        AllowSubscriptionsFrom<Everything>,
    )>, // required for others to keep track of our xcm version
    // XCM barrier that allows Tinkernet Multisigs to transact if paying for execution.
    WithComputedOrigin<AllowTopLevelPaidExecutionFrom<TinkernetMultisigMultiLocation>, UniversalLocation, ConstU32<8>>,
);

parameter_types! {
    // One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer.
    pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0u64);
    pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;

// the ksm cost to to execute a no-op extrinsic
fn base_tx_in_ksm() -> Balance {
    KSM.one() / 50_000
}

pub fn ksm_per_second() -> u128 {
    let base_weight = Balance::from(ExtrinsicBaseWeight::get().ref_time());
    let base_tx_per_second = (WEIGHT_REF_TIME_PER_SECOND as u128) / base_weight;
    base_tx_per_second * base_tx_in_ksm()
}

pub fn kint_per_second() -> u128 {
    (ksm_per_second() * 4) / 3
}

parameter_types! {
    pub KsmPerSecond: (AssetId, u128, u128) = (MultiLocation::parent().into(), ksm_per_second(),
    0, // todo: determine how much to charge per mb of proof
);
    pub KintPerSecond: (AssetId, u128, u128) = ( // can be removed once we no longer need to support polkadot < 0.9.16
        non_canonical_currency_location(Token(KINT)).into(),
        // KINT:KSM = 4:3
        kint_per_second(),
        0, // todo: determine how much to charge per mb of proof
    );
    pub KbtcPerSecond: (AssetId, u128, u128) = ( // can be removed once we no longer need to support polkadot < 0.9.16
        non_canonical_currency_location(Token(KBTC)).into(),
        // KBTC:KSM = 1:150 & Satoshi:Planck = 1:10_000
        ksm_per_second() / 1_500_000,
        0, // todo: determine how much to charge per mb of proof
    );
    pub CanonicalizedKintPerSecond: (AssetId, u128, u128) = (
        canonical_currency_location(Token(KINT)).into(),
        // KINT:KSM = 4:3
        kint_per_second(),
        0, // todo: determine how much to charge per mb of proof
    );
    pub CanonicalizedKbtcPerSecond: (AssetId, u128, u128) = (
        canonical_currency_location(Token(KBTC)).into(),
        // KBTC:KSM = 1:150 & Satoshi:Planck = 1:10_000
        ksm_per_second() / 1_500_000,
        0, // todo: determine how much to charge per mb of proof
    );
    pub const RelayNetwork: NetworkId = NetworkId::Kusama;
    pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
    pub const MaxAssetsIntoHolding: u32 = 8;
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
    fn take_revenue(revenue: MultiAsset) {
        if let MultiAsset {
            id: Concrete(location),
            fun: Fungible(amount),
        } = revenue
        {
            if let Some(currency_id) = CurrencyIdConvert::convert(location) {
                // Note: we should ensure that treasury account has existential deposit for all of the cross-chain
                // asset. Ignore the result.
                let _ = Tokens::deposit(currency_id, &TreasuryAccount::get(), amount);
            }
        }
    }
}

pub type Trader = (
    FixedRateOfFungible<KsmPerSecond, ToTreasury>,
    FixedRateOfFungible<KintPerSecond, ToTreasury>,
    FixedRateOfFungible<KbtcPerSecond, ToTreasury>,
    FixedRateOfFungible<CanonicalizedKintPerSecond, ToTreasury>,
    FixedRateOfFungible<CanonicalizedKbtcPerSecond, ToTreasury>,
    AssetRegistryTrader<FixedRateAssetRegistryTrader<MyFixedConversionRateProvider>, ToTreasury>,
);

pub struct MyFixedConversionRateProvider;
impl FixedConversionRateProvider for MyFixedConversionRateProvider {
    fn get_fee_per_second(location: &MultiLocation) -> Option<u128> {
        let metadata = AssetRegistry::fetch_metadata_by_location(location)?;
        Some(metadata.additional.fee_per_second)
    }
}

/// A call filter for the XCM Transact instruction. This is a temporary measure until we properly
/// account for proof size weights.
///
/// Calls that are allowed through this filter must:
/// 1. Have a fixed weight;
/// 2. Cannot lead to another call being made;
/// 3. Have a defined proof size weight, e.g. no unbounded vecs in call parameters.
pub struct SafeCallFilter;
impl Contains<RuntimeCall> for SafeCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        // we need to filter all calls that can recurse. We're being a bit overly conservative here
        // by completly blocking the pallets below rather than filter per specific call.
        match call {
            RuntimeCall::Sudo(..) | RuntimeCall::Proxy(..) | RuntimeCall::Multisig(..) | RuntimeCall::Utility(..) => {
                // these calls can recurse - disallow
                false
            }
            RuntimeCall::Issue(..) | RuntimeCall::Replace(..) | RuntimeCall::Redeem(..) | RuntimeCall::BTCRelay(..) => {
                // disallow anything to do with btc transactions since btc tx may be unbounded
                false
            }
            _ => true,
        }
    }
}

impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    #[cfg(feature = "runtime-benchmarks")]
    type AssetTransactor = BenchmarkingLocalAssetTransactor;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
    type IsTeleporter = Nothing; // no teleportation allowed
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = Trader;
    type ResponseHandler = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type SafeCallFilter = SafeCallFilter;
    type CallDispatcher = WithOriginFilter<SafeCallFilter>;
    type UniversalLocation = UniversalLocation;
    type Aliasers = Nothing;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (SignedToAccountId32<RuntimeOrigin, AccountId, ParentNetwork>,);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>, /* note: sets PriceForParentDelivery
                                                                                * to 0 */
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub const ReachableDest: MultiLocation = MultiLocation::parent();
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type RuntimeOrigin = RuntimeOrigin;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type Currency = NativeCurrency; // note: not used due to the empty CurrencyMatcher
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<8>;
    type UniversalLocation = UniversalLocation;
    type WeightInfo = pallet_xcm::TestWeightInfo; // todo: use actual weight
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
    type AdminOrigin = EnsureRoot<AccountId>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type PriceForSiblingDelivery = ();
    type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

pub type LocalAssetTransactor = MultiCurrencyAdapter<
    Tokens,
    UnknownTokens,
    IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
    AccountId,
    LocationToAccountId,
    CurrencyId,
    CurrencyIdConvert,
    DepositToAlternative<TreasuryAccount, Tokens, CurrencyId, AccountId, Balance>,
>;

fn general_key_of(id: CurrencyId) -> Junction {
    let encoded = id.encode();
    let mut data = [0u8; 32];
    if encoded.len() > 32 {
        // we are not returning result, so panic is inevitable. Let's make it explicit.
        panic!("Currency ID was too long to be encoded");
    }
    data[..encoded.len()].copy_from_slice(&encoded[..]);
    GeneralKey {
        length: encoded.len() as u8,
        data,
    }
}

pub fn canonical_currency_location(id: CurrencyId) -> MultiLocation {
    MultiLocation::new(0, X1(general_key_of(id)))
}

pub fn non_canonical_currency_location(id: CurrencyId) -> MultiLocation {
    MultiLocation::new(1, X2(Parachain(ParachainInfo::get().into()), general_key_of(id)))
}

pub use currency_id_convert::CurrencyIdConvert;

mod currency_id_convert {
    use super::*;

    pub struct CurrencyIdConvert;

    impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
        fn convert(id: CurrencyId) -> Option<MultiLocation> {
            match id {
                PARENT_CURRENCY_ID => Some(MultiLocation::parent()),
                WRAPPED_CURRENCY_ID => Some(non_canonical_currency_location(id)),
                NATIVE_CURRENCY_ID => Some(non_canonical_currency_location(id)),
                ForeignAsset(id) => AssetRegistry::multilocation(&id).unwrap_or_default(),
                _ => None,
            }
        }
    }

    impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
        fn convert(location: MultiLocation) -> Option<CurrencyId> {
            fn decode_currency_id(length: u8, data: [u8; 32]) -> Option<CurrencyId> {
                let length = length as usize;
                if length > data.len() {
                    return None;
                }
                // decode the general key
                if let Ok(currency_id) = CurrencyId::decode(&mut &data[..length]) {
                    // check `currency_id` is cross-chain asset
                    match currency_id {
                        WRAPPED_CURRENCY_ID => Some(currency_id),
                        NATIVE_CURRENCY_ID => Some(currency_id),
                        _ => None,
                    }
                } else {
                    None
                }
            }

            match location.clone() {
                x if x == MultiLocation::parent() => Some(PARENT_CURRENCY_ID),
                MultiLocation {
                    parents: 1,
                    interior: X2(Parachain(id), GeneralKey { length, data }),
                } if ParaId::from(id) == ParachainInfo::get() => decode_currency_id(length, data),
                MultiLocation {
                    // adapt for reanchor canonical location: https://github.com/paritytech/polkadot/pull/4470
                    parents: 0,
                    interior: X1(GeneralKey { length, data }),
                } => decode_currency_id(length, data),
                _ => None,
            }
            .or_else(|| AssetRegistry::location_to_asset_id(&location).map(|id| CurrencyId::ForeignAsset(id)))
        }
    }

    impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
        fn convert(asset: MultiAsset) -> Option<CurrencyId> {
            if let MultiAsset {
                id: Concrete(location), ..
            } = asset
            {
                Self::convert(location)
            } else {
                None
            }
        }
    }
}

parameter_types! {
    pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
    pub const MaxAssetsForTransfer: usize = 2; // potentially useful to send both kint and kbtc at once
}

const STATEMINE_PARA_ID: u32 = 1000;
const STATEMINE_XCM_FEE: u128 = 500_000_000; // statemine fee was 16_000_000 on dec 15 2022: https://statemine.stg.subscan.io/xcm_message/kusama-bec543e48e201aa2b4f6efded509626d14091014
parameter_type_with_key! {
    // Used to determine KSM fee when transferring to statemine. https://github.com/open-web3-stack/open-runtime-module-library/blob/cadcc9fb10b8212f92668138fc8f83dc0c53acf5/xtokens/README.md#transfer-multiple-currencies
    pub ParachainMinFee: |location: MultiLocation| -> Option<u128> {
        #[allow(clippy::match_ref_pats)] // false positive
        match (location.parents, location.first_interior()) {
            (1, Some(Parachain(id))) if *id == STATEMINE_PARA_ID => Some(STATEMINE_XCM_FEE),
            _ => None,
        }
    };
}

pub struct AccountIdToMultiLocation;

impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: None,
            id: account.into(),
        })
        .into()
    }
}

impl orml_xtokens::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type CurrencyId = CurrencyId;
    type CurrencyIdConvert = CurrencyIdConvert;
    type AccountIdToMultiLocation = AccountIdToMultiLocation;
    type SelfLocation = SelfLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type BaseXcmWeight = UnitWeightCost;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    type MinXcmFee = ParachainMinFee;
    type MultiLocationsFilter = Everything;
    type ReserveProvider = AbsoluteReserveProvider;
    type UniversalLocation = UniversalLocation;
}

/// Tinkernet Multisig Multilocation for XCM barriers.
pub struct TinkernetMultisigMultiLocation;
impl Contains<MultiLocation> for TinkernetMultisigMultiLocation {
    fn contains(t: &MultiLocation) -> bool {
        matches!(
            t,
            MultiLocation {
                parents: 1,
                interior: Junctions::X3(
                    Junction::Parachain(2125),
                    Junction::PalletInstance(71),
                    Junction::GeneralIndex(_)
                )
            }
        )
    }
}

/// Constant derivation function for Tinkernet Multisigs.
/// Uses the Tinkernet genesis hash as a salt.
pub fn derive_tinkernet_multisig<AccountId: Decode>(id: u128) -> Result<AccountId, ()> {
    AccountId::decode(&mut TrailingZeroInput::new(
        &(
            // The constant salt used to derive Tinkernet Multisigs, this is Tinkernet's genesis hash.
            H256([
                212, 46, 150, 6, 169, 149, 223, 228, 51, 220, 121, 85, 220, 42, 112, 244, 149, 243, 80, 243, 115, 218,
                162, 0, 9, 138, 232, 68, 55, 129, 106, 210,
            ]),
            // The actual multisig integer id.
            u32::try_from(id).map_err(|_| ())?,
        )
            .using_encoded(blake2_256),
    ))
    .map_err(|_| ())
}

/// Convert a Tinkernet Multisig `MultiLocation` value into a local `AccountId`.
pub struct TinkernetMultisigAsAccountId<AccountId>(PhantomData<AccountId>);
impl<AccountId: Decode + Clone> XcmConvert<MultiLocation, AccountId> for TinkernetMultisigAsAccountId<AccountId> {
    fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AccountId, ()> {
        match location.borrow() {
            MultiLocation {
                parents: 1,
                interior:
                    X3(
                        Parachain(2125),
                        PalletInstance(71),
                        // Index from which the multisig account is derived.
                        GeneralIndex(id),
                    ),
            } => derive_tinkernet_multisig(*id),
            _ => Err(()),
        }
    }
}

/// Convert a Tinkernet Multisig `MultiLocation` value into a `Signed` origin.
pub struct TinkernetMultisigAsNativeOrigin<RuntimeOrigin>(PhantomData<RuntimeOrigin>);
impl<RuntimeOrigin: OriginTrait> ConvertOrigin<RuntimeOrigin> for TinkernetMultisigAsNativeOrigin<RuntimeOrigin>
where
    RuntimeOrigin::AccountId: Decode,
{
    fn convert_origin(origin: impl Into<MultiLocation>, kind: OriginKind) -> Result<RuntimeOrigin, MultiLocation> {
        let origin = origin.into();
        match (kind, origin) {
            (
                OriginKind::Native,
                MultiLocation {
                    parents: 1,
                    interior:
                        X3(
                            Junction::Parachain(2125),
                            Junction::PalletInstance(71),
                            // Index from which the multisig account is derived.
                            Junction::GeneralIndex(id),
                        ),
                },
            ) => Ok(RuntimeOrigin::signed(
                derive_tinkernet_multisig(id).map_err(|_| origin)?,
            )),
            (_, origin) => Err(origin),
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
use benchmark_impls::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmark_impls {
    use super::*;
    use frame_benchmarking::BenchmarkError;

    /// wrapper around LocalAssetTransactor that fakes teleports for bencharking purposes.
    /// Without this wrapper, benchmarks just fail. Since we don't support teleports,
    /// faking this is acceptable.
    pub struct BenchmarkingLocalAssetTransactor;

    #[cfg(feature = "runtime-benchmarks")]
    impl xcm_executor::traits::TransactAsset for BenchmarkingLocalAssetTransactor {
        fn can_check_in(_origin: &MultiLocation, _what: &MultiAsset, _context: &XcmContext) -> XcmResult {
            Ok(())
        }
        fn check_in(_origin: &MultiLocation, _what: &MultiAsset, _context: &XcmContext) {}

        fn can_check_out(_dest: &MultiLocation, _what: &MultiAsset, _context: &XcmContext) -> XcmResult {
            Ok(())
        }
        fn check_out(_dest: &MultiLocation, _what: &MultiAsset, _context: &XcmContext) {}

        fn deposit_asset(what: &MultiAsset, who: &MultiLocation, context: &XcmContext) -> XcmResult {
            LocalAssetTransactor::deposit_asset(what, who, context)
        }
        fn withdraw_asset(
            what: &MultiAsset,
            who: &MultiLocation,
            maybe_context: Option<&XcmContext>,
        ) -> Result<xcm_executor::Assets, XcmError> {
            LocalAssetTransactor::withdraw_asset(what, who, maybe_context)
        }

        fn internal_transfer_asset(
            asset: &MultiAsset,
            from: &MultiLocation,
            to: &MultiLocation,
            context: &XcmContext,
        ) -> Result<xcm_executor::Assets, XcmError> {
            LocalAssetTransactor::internal_transfer_asset(asset, from, to, context)
        }

        fn transfer_asset(
            asset: &MultiAsset,
            from: &MultiLocation,
            to: &MultiLocation,
            context: &XcmContext,
        ) -> Result<xcm_executor::Assets, XcmError> {
            LocalAssetTransactor::transfer_asset(asset, from, to, context)
        }
    }

    impl pallet_xcm_benchmarks::Config for Runtime {
        type XcmConfig = XcmConfig;
        type AccountIdConverter = xcm_config::LocationToAccountId;
        fn valid_destination() -> Result<MultiLocation, BenchmarkError> {
            Ok(MultiLocation::parent())
        }
        fn worst_case_holding(_depositable_count: u32) -> MultiAssets {
            // 8 fungibles
            const HOLDING_FUNGIBLES: u32 = 9;
            let fungibles_amount: u128 = 100;
            let assets = (0..HOLDING_FUNGIBLES)
                .map(|i| {
                    let location: MultiLocation = GeneralIndex(i as u128).into();
                    MultiAsset {
                        id: Concrete(location),
                        fun: Fungible(fungibles_amount * i as u128),
                    }
                    .into()
                })
                .chain(core::iter::once(MultiAsset {
                    id: Concrete(MultiLocation::parent()),
                    fun: Fungible(u128::MAX),
                }))
                .collect::<Vec<_>>();

            assets.into()
        }
    }

    parameter_types! {
        pub TrustedTeleporter: Option<(MultiLocation, MultiAsset)> = None;
        pub CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
    }

    impl pallet_xcm_benchmarks::fungible::Config for Runtime {
        type TransactAsset = orml_tokens::CurrencyAdapter<Runtime, GetNativeCurrencyId>;

        type CheckedAccount = CheckedAccount;
        type TrustedTeleporter = TrustedTeleporter;

        fn get_multi_asset() -> MultiAsset {
            MultiAsset {
                id: Concrete(canonical_currency_location(Token(KINT))),
                fun: Fungible(100000000000),
            }
        }
    }

    impl pallet_xcm_benchmarks::generic::Config for Runtime {
        type RuntimeCall = RuntimeCall;

        fn worst_case_response() -> (u64, Response) {
            (0u64, Response::Version(Default::default()))
        }

        fn worst_case_asset_exchange() -> Result<(MultiAssets, MultiAssets), BenchmarkError> {
            // not supported atm
            Err(BenchmarkError::Skip)
        }

        fn universal_alias() -> Result<(MultiLocation, Junction), BenchmarkError> {
            // The XCM executor doesn't have a configured `UniversalAliases`
            Err(BenchmarkError::Skip)
        }

        fn transact_origin_and_runtime_call() -> Result<(MultiLocation, RuntimeCall), BenchmarkError> {
            let origin = MultiLocation::parent();
            let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
            // transact disallowed, but necessary for unsubscribe_version
            Ok((origin, call))
        }

        fn subscribe_origin() -> Result<MultiLocation, BenchmarkError> {
            Ok(MultiLocation::parent())
        }

        fn claimable_asset() -> Result<(MultiLocation, MultiLocation, MultiAssets), BenchmarkError> {
            let origin = MultiLocation::parent();
            let assets: MultiAssets = (Concrete(MultiLocation::parent()), 1_000u128).into();
            let ticket = MultiLocation {
                parents: 0,
                interior: Here,
            };
            Ok((origin, ticket, assets))
        }

        fn unlockable_asset() -> Result<(MultiLocation, MultiLocation, MultiAsset), BenchmarkError> {
            // we don't support asset locking
            Err(BenchmarkError::Skip)
        }

        fn export_message_origin_and_destination(
        ) -> Result<(MultiLocation, NetworkId, InteriorMultiLocation), BenchmarkError> {
            // We don't support exporting messages
            Err(BenchmarkError::Skip)
        }

        fn alias_origin() -> Result<(MultiLocation, MultiLocation), BenchmarkError> {
            // The XCM executor of Polkadot doesn't have a configured `Aliasers`
            Err(BenchmarkError::Skip)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type FungiblesWeight = crate::weights::pallet_xcm_benchmarks_fungible::WeightInfo<Runtime>;
    type GenericsWeight = crate::weights::pallet_xcm_benchmarks_generic::WeightInfo<Runtime>;

    fn check_assets_weight(weight: Weight) {
        let unit_weight_cost = UnitWeightCost::get();
        let holding_items: u64 = MaxAssetsIntoHolding::get().into();
        // multiply by holding_items * 2 because these instructions iterate over assets
        // in the holding, and documentation states: "In the worse case, the Holding
        // Register may contain up to twice as many assets as this"
        assert!(weight.ref_time() * holding_items * 2 <= unit_weight_cost.ref_time());
    }

    #[test]
    #[ignore] // disabled for now since it requires weights measured on production machines
    fn test_weights() {
        let unit_weight_cost = UnitWeightCost::get();

        // the following don't have benchmarks..
        // check_assets_weight(FungiblesWeight::initiate_reserve_withdraw());
        // check_assets_weight(FungiblesWeight::reserve_asset_deposited());

        // check instructions that iterate over assets
        check_assets_weight(FungiblesWeight::withdraw_asset());
        check_assets_weight(FungiblesWeight::transfer_asset());
        check_assets_weight(FungiblesWeight::transfer_reserve_asset());
        check_assets_weight(FungiblesWeight::deposit_asset());
        check_assets_weight(FungiblesWeight::deposit_reserve_asset());
        check_assets_weight(GenericsWeight::burn_asset());
        check_assets_weight(GenericsWeight::expect_asset());

        assert!(GenericsWeight::clear_origin().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::descend_origin().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::report_error().ref_time() <= unit_weight_cost.ref_time());

        assert!(GenericsWeight::report_holding().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::buy_execution().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::refund_surplus().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::set_error_handler().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::set_appendix().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::clear_error().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::claim_asset().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::trap().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::subscribe_version().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::unsubscribe_version().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::expect_origin().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::expect_error().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::expect_transact_status().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::query_response().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::query_pallet().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::expect_pallet().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::report_transact_status().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::clear_transact_status().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::set_fees_mode().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::set_topic().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::clear_topic().ref_time() <= unit_weight_cost.ref_time());
        assert!(GenericsWeight::unpaid_execution().ref_time() <= unit_weight_cost.ref_time());
    }
}
