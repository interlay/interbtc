// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! Test utilities

use crate as multi_transaction_payment;
use core::marker::PhantomData;
use dex_general::GenerateLpAssetId;
use frame_support::{
    parameter_types,
    traits::{Contains, Currency as CurrencyTrait, Imbalance, OnUnbalanced},
    weights::{ConstantMultiplier, IdentityFee},
    PalletId,
};
use orml_tokens::{CurrencyAdapter, NegativeImbalance};
use orml_traits::parameter_type_with_key;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use primitives::{CurrencyId, CurrencyId::*, TokenSymbol::*};
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128, Perquintill};
use sp_core::{ConstU32, Get, H256};
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, Bounded, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type NativeCurrency = CurrencyAdapter<Test, GetNativeCurrencyId>;
pub type SlowAdjustingFeeUpdate<R> =
    TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier, MaximumMultiplier>;
pub type AccountId = u128;
pub type Balance = u128;
pub type SignedFixedPoint = FixedI128;
pub type SignedInner = i128;
pub type UnsignedFixedPoint = FixedU128;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        DexGeneral: dex_general::{Pallet, Call, Storage, Event<T>},
        Currency: currency::{Pallet},
        Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
        MultiTransactionPayment: multi_transaction_payment::{Call, Pallet, Storage},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
        Testing: testing_helpers::{Call},
    }
);

#[frame_support::pallet]
pub mod testing_helpers {
    use frame_support::{
        dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
        pallet_prelude::*,
    };
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::error]
    pub enum Error<T> {
        Fail,
    }
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({
            *_expected
		})]
        #[frame_support::transactional]
        pub fn weighted(
            _origin: OriginFor<T>,
            _expected: Weight,
            actual_weight: Option<Weight>,
            err: bool,
            pays_fee: Pays,
        ) -> DispatchResultWithPostInfo {
            let post_info = PostDispatchInfo {
                actual_weight,
                pays_fee,
            };
            if err {
                Err(DispatchErrorWithPostInfo {
                    error: Error::<T>::Fail.into(),
                    post_info,
                })
            } else {
                Ok(post_info)
            }
        }
    }
}

pub struct DealWithFees<T, GetCurrencyId, GetAccountId>(PhantomData<(T, GetCurrencyId, GetAccountId)>);

impl<T, GetCurrencyId, GetAccountId> OnUnbalanced<NegativeImbalance<T, GetCurrencyId>>
    for DealWithFees<T, GetCurrencyId, GetAccountId>
where
    T: orml_tokens::Config,
    GetCurrencyId: Get<T::CurrencyId>,
    GetAccountId: Get<T::AccountId>,
{
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<T, GetCurrencyId>>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            orml_tokens::CurrencyAdapter::<T, GetCurrencyId>::resolve_creating(&GetAccountId::get(), fees);
        }
    }
}

pub struct PairLpIdentity;
impl GenerateLpAssetId<CurrencyId> for PairLpIdentity {
    fn generate_lp_asset_id(asset_0: CurrencyId, asset_1: CurrencyId) -> Option<CurrencyId> {
        CurrencyId::join_lp_token(asset_0, asset_1)
    }
}

pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(
        _amount: &currency::Amount<Test>,
        _to: CurrencyId,
    ) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
        unimplemented!()
    }
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const BlockHashCount: u64 = 250;
    pub const DexGeneralPalletId: PalletId = PalletId(*b"dex/genr");
    pub const MaxReserves: u32 = 50;
    pub const MaxLocks:u32 = 50;
    pub const TransactionByteFee: Balance = 1;
    pub OperationalFeeMultiplier: u8 = 5;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1u128, 1_000_000u128);
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
    pub const GetNativeCurrencyId: CurrencyId = CurrencyId::ForeignAsset(0);
    pub const GetForeignCurrencyId: CurrencyId = CurrencyId::ForeignAsset(2323);
    pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::ForeignAsset(2);
    pub const GetWrappedCurrencyId: CurrencyId = CurrencyId::ForeignAsset(3);
    pub TreasuryAccountId: AccountId = PalletId(*b"treasury").into_account_truncating();
}

impl crate::Config for Test {
    type Currency = NativeCurrency;
    type DexWeightInfo = ();
    type Dex = DexGeneral;
    type OnUnbalanced = DealWithFees<Test, GetNativeCurrencyId, TreasuryAccountId>;
    type RuntimeCall = RuntimeCall;
}

impl pallet_transaction_payment::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = MultiTransactionPayment;
    // CurrencyAdapter2<NativeCurrency, DealWithFees<Runtime, GetNativeCurrencyId>>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl testing_helpers::Config for Test {}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u128;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> u128 {
        0
    };
}

pub struct MockDustRemovalWhitelist;
impl Contains<AccountId> for MockDustRemovalWhitelist {
    fn contains(_a: &AccountId) -> bool {
        true
    }
}

impl orml_tokens::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type Amount = i128;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type MaxLocks = ();
    type DustRemovalWhitelist = MockDustRemovalWhitelist;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type CurrencyHooks = ();
}

impl currency::Config for Test {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetCollateralCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = CurrencyConvert;
}

impl dex_general::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MultiCurrency = Tokens;
    type PalletId = DexGeneralPalletId;
    type AssetId = CurrencyId;
    type LpGenerate = PairLpIdentity;
    type WeightInfo = ();
    type MaxBootstrapRewards = ConstU32<1000>;
    type MaxBootstrapLimits = ConstU32<1000>;
    type EnsurePairAsset = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into();

    dex_general::GenesisConfig::<Test> {
        fee_receiver: None,
        fee_point: 5,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
