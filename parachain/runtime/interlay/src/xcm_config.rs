use super::*;
use crate::CurrencyId::ForeignAsset;
use codec::{Decode, Encode};
use cumulus_primitives_core::ParaId;
use frame_support::{
    parameter_types,
    traits::{Everything, Get, Nothing},
};
use orml_asset_registry::{AssetRegistryTrader, FixedRateAssetRegistryTrader};
use orml_traits::{
    location::AbsoluteReserveProvider, parameter_type_with_key, FixedConversionRateProvider, MultiCurrency,
};
use orml_xcm_support::{DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::WeakBoundedVec;
use xcm::latest::{prelude::*, Weight};
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
    EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, LocationInverter, NativeAsset, ParentIsPreset,
    RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit,
};
pub use xcm_executor;
use xcm_executor::{Config, XcmExecutor};

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
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    AllowKnownQueryResponses<PolkadotXcm>,
    AllowSubscriptionsFrom<Everything>,
); // required for others to keep track of our xcm version

parameter_types! {
    // One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer.
    pub UnitWeightCost: Weight = 200_000_000;
    pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;

// the dot cost to to execute a no-op extrinsic
fn base_tx_in_dot() -> Balance {
    DOT.one() / 5706 // same worth as on kintsugi - we use 1/50_000 KSM there, and currently KSM:DOT = 1:8.763822.
}

pub fn dot_per_second() -> u128 {
    let base_weight = Balance::from(ExtrinsicBaseWeight::get().ref_time());
    let base_tx_per_second = (WEIGHT_PER_SECOND.ref_time() as u128) / base_weight;
    base_tx_per_second * base_tx_in_dot()
}

parameter_types! {
    pub DotPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), dot_per_second());
    pub CanonicalizedIntrPerSecond: (AssetId, u128) = (
        canonical_currency_location(Token(INTR)).into(),
        // INTR:DOT = 4:3
        (dot_per_second() * 4) / 3
    );
    pub CanonicalizedIbtcPerSecond: (AssetId, u128) = (
        canonical_currency_location(Token(IBTC)).into(),
        // (I)BTC:DOT = 1:2266 & Satoshi:Planck = 1:100
        dot_per_second() / 226_600
    );
    pub IntrPerSecond: (AssetId, u128) = ( // can be removed once we no longer need to support polkadot < 0.9.16
        non_canonical_currency_location(Token(INTR)).into(),
        // KINT:KSM = 4:3
        (dot_per_second() * 4) / 3
    );
    pub IbtcPerSecond: (AssetId, u128) = (
        non_canonical_currency_location(Token(IBTC)).into(),
        // (I)BTC:DOT = 1:2266 & Satoshi:Planck = 1:100
        dot_per_second() / 226_600
    );
}

pub struct ToAuthor;
impl TakeRevenue for ToAuthor {
    fn take_revenue(revenue: MultiAsset) {
        if let MultiAsset {
            id: Concrete(location),
            fun: Fungible(amount),
        } = revenue
        {
            if let Some(currency_id) = CurrencyIdConvert::convert(location) {
                if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
                    // Note: will need rethinking once we have existential deposits. Ignore the result.
                    let _ = Tokens::deposit(currency_id, &author, amount);
                } else {
                    // should only happen in tests. In the tests it helps us ensure that the fee are
                    // dealt with.
                    let _ = Tokens::deposit(currency_id, &TreasuryAccount::get(), amount);
                }
            }
        }
    }
}

pub type Trader = (
    FixedRateOfFungible<DotPerSecond, ToAuthor>,
    FixedRateOfFungible<CanonicalizedIntrPerSecond, ToAuthor>,
    FixedRateOfFungible<CanonicalizedIbtcPerSecond, ToAuthor>,
    // The xcm-executor ensures no non-canonicalized versions will be used in transfers to our chain
    // when execute_xcm_in_credit is used. However, there are still cases where it can appear, e.g.
    // when reclaiming trapped tokens.
    FixedRateOfFungible<IntrPerSecond, ToAuthor>,
    FixedRateOfFungible<IbtcPerSecond, ToAuthor>,
    AssetRegistryTrader<FixedRateAssetRegistryTrader<MyFixedConversionRateProvider>, ToAuthor>,
);

pub struct MyFixedConversionRateProvider;
impl FixedConversionRateProvider for MyFixedConversionRateProvider {
    fn get_fee_per_second(location: &MultiLocation) -> Option<u128> {
        let metadata = AssetRegistry::fetch_metadata_by_location(location)?;
        Some(metadata.additional.fee_per_second)
    }
}

impl Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
    type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = Trader;
    type ResponseHandler = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (SignedToAccountId32<RuntimeOrigin, AccountId, ParentNetwork>,);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

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
    type LocationInverter = LocationInverter<Ancestry>;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
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
    type WeightInfo = ();
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

pub fn canonical_currency_location(id: CurrencyId) -> MultiLocation {
    MultiLocation::new(
        0,
        X1(GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
            id.encode(),
            None,
        ))),
    )
}

pub fn non_canonical_currency_location(id: CurrencyId) -> MultiLocation {
    MultiLocation::new(
        1,
        X2(
            Parachain(ParachainInfo::get().into()),
            GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(id.encode(), None)),
        ),
    )
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
            fn decode_currency_id(key: Vec<u8>) -> Option<CurrencyId> {
                // decode the general key
                if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
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
                    interior: X2(Parachain(id), GeneralKey(key)),
                } if ParaId::from(id) == ParachainInfo::get() => decode_currency_id(key.into_inner()),
                MultiLocation {
                    // adapt for reanchor canonical location: https://github.com/paritytech/polkadot/pull/4470
                    parents: 0,
                    interior: X1(GeneralKey(key)),
                } => decode_currency_id(key.into_inner()),
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
    pub const MaxAssetsForTransfer: usize = 2; // set to 2 so we can transfer intr and ibtc at once
}

parameter_type_with_key! {
    // Only used for transferring parachain tokens to other parachains using DOT as fee currency. Currently we do not support this, hence return MAX.
    // See: https://github.com/open-web3-stack/open-runtime-module-library/blob/cadcc9fb10b8212f92668138fc8f83dc0c53acf5/xtokens/README.md#transfer-multiple-currencies
    pub ParachainMinFee: |location: MultiLocation| -> Option<u128> {
        #[allow(clippy::match_ref_pats)] // false positive
        match (location.parents, location.first_interior()) {
            _ => None,
        }
    };
}

pub struct AccountIdToMultiLocation;

impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: NetworkId::Any,
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
    type LocationInverter = <XcmConfig as Config>::LocationInverter;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    type MinXcmFee = ParachainMinFee;
    type MultiLocationsFilter = Everything;
    type ReserveProvider = AbsoluteReserveProvider;
}
