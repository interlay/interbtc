use super::*;
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
use xcm_executor::{traits::WeightBounds, Config, XcmExecutor};
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

// the ksm cost to to execute a no-op extrinsic
fn base_tx_in_ksm() -> Balance {
    KSM.one() / 50_000
}

pub fn ksm_per_second() -> u128 {
    let base_weight = Balance::from(ExtrinsicBaseWeight::get().ref_time());
    let base_tx_per_second = (WEIGHT_PER_SECOND.ref_time() as u128) / base_weight;
    base_tx_per_second * base_tx_in_ksm()
}

pub fn kint_per_second() -> u128 {
    (ksm_per_second() * 4) / 3
}

parameter_types! {
    pub KsmPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), ksm_per_second());
    pub KintPerSecond: (AssetId, u128) = ( // can be removed once we no longer need to support polkadot < 0.9.16
        non_canonical_currency_location(Token(KINT)).into(),
        // KINT:KSM = 4:3
        kint_per_second()
    );
    pub KbtcPerSecond: (AssetId, u128) = ( // can be removed once we no longer need to support polkadot < 0.9.16
        non_canonical_currency_location(Token(KBTC)).into(),
        // KBTC:KSM = 1:150 & Satoshi:Planck = 1:10_000
        ksm_per_second() / 1_500_000
    );
    pub CanonicalizedKintPerSecond: (AssetId, u128) = (
        canonical_currency_location(Token(KINT)).into(),
        // KINT:KSM = 4:3
        kint_per_second()
    );
    pub CanonicalizedKbtcPerSecond: (AssetId, u128) = (
        canonical_currency_location(Token(KBTC)).into(),
        // KBTC:KSM = 1:150 & Satoshi:Planck = 1:10_000
        ksm_per_second() / 1_500_000
    );
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

trait Tax {
    fn tax(&self) -> (Self, Self)
    where
        Self: Sized;
}
impl Tax for MultiAssets {
    fn tax(&self) -> (Self, Self) {
        let mut taxed = vec![];
        let mut tax = vec![];

        for asset in self.inner().iter() {
            match asset {
                &MultiAsset {
                    ref id,
                    fun: Fungibility::Fungible(amount),
                } => {
                    tax.push(MultiAsset {
                        id: id.clone(),
                        fun: Fungibility::Fungible(amount / 10),
                    });
                    taxed.push(MultiAsset {
                        id: id.clone(),
                        fun: Fungibility::Fungible((amount * 9) / 10),
                    });
                }
                x => taxed.push(x.clone()),
            }
        }
        (taxed.into(), tax.into())
    }
}

fn limit_buy_execution(mut xcm: Xcm<()>, holding: MultiAssets, at: &MultiLocation) -> Xcm<()> {
    use xcm_executor::traits::InvertLocation;

    let mut iter = xcm.0.iter_mut();
    match iter.next() {
        Some(BuyExecution {
            ref mut fees,
            weight_limit,
        }) => {
            if let Some(asset) = holding.inner().iter().find(|x| {
                let ancestry = LocationInverter::<Ancestry>::ancestry();
                let mut tmp = (*x).clone();
                match tmp.reanchored(at, &ancestry) {
                    Ok(reanchored) => reanchored.id == fees.id,
                    _ => false,
                }
            }) {
                match (&mut fees.fun, &asset.fun) {
                    (Fungibility::Fungible(ref mut a), Fungibility::Fungible(b)) => {
                        *a = *b;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    xcm
}

fn transform_outbound(message: Xcm<RuntimeCall>, original_weight: Weight) -> Result<(Xcm<RuntimeCall>, Weight), ()> {
    let transformed_message = match &message.0.as_slice() {
        &[TransferReserveAsset { assets, dest, xcm }] => {
            // transfer_self_reserve_asset
            let (taxed, tax) = assets.tax();
            let transfer_reserve_asset = TransferReserveAsset {
                assets: taxed.clone(),
                dest: dest.clone(),
                xcm: limit_buy_execution(xcm.clone(), taxed.clone(), &dest),
            };
            if tax.is_none() {
                Some(Xcm(vec![transfer_reserve_asset]))
            } else {
                let transfer_to_dao = TransferAsset {
                    assets: tax,
                    beneficiary: Junction::AccountId32 {
                        network: NetworkId::Any,
                        id: TreasuryAccount::get().into(),
                    }
                    .into(),
                };
                Some(Xcm(vec![transfer_reserve_asset, transfer_to_dao]))
            }
        }
        &[WithdrawAsset(assets), InitiateReserveWithdraw {
            assets: reserve_assets,
            reserve,
            xcm,
        }] => {
            // transfer_self_reserve_asset
            let (taxed, tax) = assets.tax();
            let withdraw = WithdrawAsset(taxed.clone());

            let mut reserve_withdraw = InitiateReserveWithdraw {
                assets: reserve_assets.clone(),
                reserve: reserve.clone(),
                xcm: limit_buy_execution(xcm.clone(), taxed.clone(), &reserve),
            };

            if tax.is_none() {
                Some(Xcm(vec![withdraw, reserve_withdraw.clone()]))
            } else {
                let transfer_to_dao = TransferAsset {
                    assets: tax,
                    beneficiary: Junction::AccountId32 {
                        network: NetworkId::Any,
                        id: TreasuryAccount::get().into(),
                    }
                    .into(),
                };
                Some(Xcm(vec![withdraw, transfer_to_dao, reserve_withdraw.clone()]))
            }
        }
        _ => None,
    };
    match transformed_message {
        Some(mut message) => {
            let new_weight = <Runtime as pallet_xcm::Config>::Weigher::weight(&mut message)?;
            Ok((message, new_weight))
        }
        None => Ok((message, original_weight)),
    }
}

fn transform_inbound(mut message: Xcm<RuntimeCall>) -> Result<Xcm<RuntimeCall>, ()> {
    // WithdrawAsset | ReserveAssetDeposited | ClaimAsset]
    // ClearOrigin*
    // BuyExecution
    // <to-insert> transfer
    // tail

    match message.0.get(0) {
        Some(WithdrawAsset(assets) | ReserveAssetDeposited(assets) | ClaimAsset { assets, .. }) => {
            let (_taxed, tax) = assets.tax();
            let buy_execution = message
                .0
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_idx, instruction)| !matches!(instruction, ClearOrigin));
            match buy_execution {
                Some((idx, BuyExecution { .. })) => {
                    let transfer_to_dao = DepositAsset {
                        assets: MultiAssetFilter::Definite(tax.clone()),
                        max_assets: tax.len() as u32,
                        beneficiary: Junction::AccountId32 {
                            network: NetworkId::Any,
                            id: TreasuryAccount::get().into(),
                        }
                        .into(),
                    };
                    message.0.insert(idx + 1, transfer_to_dao);
                    Ok((message))
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

pub struct TaxingExecutor;

use xcm::latest::ExecuteXcm;
impl ExecuteXcm<RuntimeCall> for TaxingExecutor {
    fn execute_xcm_in_credit(
        origin: impl Into<MultiLocation>,
        mut message: Xcm<RuntimeCall>,
        weight_limit: Weight,
        weight_credit: Weight,
    ) -> Outcome {
        use xcm::latest::Instruction::*;

        if weight_credit > 0 {
            let xcm_weight = match <Runtime as pallet_xcm::Config>::Weigher::weight(&mut message) {
                Ok(x) => x,
                Err(()) => return Outcome::Error(XcmError::WeightNotComputable),
            };

            if xcm_weight <= weight_credit {
                return match transform_outbound(message, xcm_weight) {
                    Ok((transformed_message, new_weight)) => {
                        <XcmExecutor<XcmConfig> as ExecuteXcm<RuntimeCall>>::execute_xcm_in_credit(
                            origin,
                            transformed_message,
                            new_weight,
                            new_weight,
                        )
                    }
                    Err(()) => Outcome::Error(XcmError::WeightNotComputable),
                };
            }
        } else {
            return match transform_inbound(message) {
                Ok(transformed_message) => {
                    <XcmExecutor<XcmConfig> as ExecuteXcm<RuntimeCall>>::execute_xcm_in_credit(
                        origin,
                        transformed_message,
                        weight_limit,
                        weight_credit,
                    )
                }
                Err(()) => Outcome::Error(XcmError::WeightNotComputable),
            }
        }

        todo!()
    }
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type RuntimeOrigin = RuntimeOrigin;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = TaxingExecutor;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = TaxingExecutor;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = TaxingExecutor;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = TaxingExecutor;
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
    pub const MaxAssetsForTransfer: usize = 2; // potentially useful to send both kint and kbtc at once
}

parameter_type_with_key! {
    // Only used for transferring parachain tokens to other parachains using KSM as fee currency. Currently we do not support this, hence return MAX.
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
    type XcmExecutor = TaxingExecutor;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type BaseXcmWeight = UnitWeightCost;
    type LocationInverter = <XcmConfig as Config>::LocationInverter;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    type MinXcmFee = ParachainMinFee;
    type MultiLocationsFilter = Everything;
    type ReserveProvider = AbsoluteReserveProvider;
}
