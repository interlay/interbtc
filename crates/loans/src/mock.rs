// Copyright 2022 Interlay.
// This file is part of Interlay.

// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub use super::*;

use crate as loans;

use currency::Amount;
use frame_benchmarking::whitelisted_caller;
use frame_support::{construct_runtime, parameter_types, traits::Everything, PalletId};
use frame_system::EnsureRoot;
use mocktopus::{macros::mockable, mocking::MockResult};
use orml_traits::{currency::MutationHooks, parameter_type_with_key};
use primitives::{
    CurrencyId::{self, LendToken, Token},
    DOT, IBTC, INTR, KBTC, KINT, KSM,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32, FixedI128};
use sp_std::vec::Vec;
use traits::OracleApi;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Loans: loans::{Pallet, Storage, Call, Event<T>, Config},
        TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>},
        Currency: currency::{Pallet},
        Utility: pallet_utility,
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type AccountId = AccountId32;
pub type BlockNumber = u64;

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);
pub const DAVE: AccountId = AccountId32::new([4u8; 32]);
pub const EVE: AccountId = AccountId32::new([5u8; 32]);

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Zero::zero()
    };
}

pub type RawAmount = i128;

pub struct CurrencyHooks<T>(PhantomData<T>);
impl<T: orml_tokens::Config + pallet::Config>
    MutationHooks<T::AccountId, T::CurrencyId, <T as currency::Config>::Balance> for CurrencyHooks<T>
where
    T::AccountId: From<sp_runtime::AccountId32>,
{
    type OnDust = ();
    type OnSlash = OnSlashHook<T>;
    type PreDeposit = PreDeposit<T>;
    type PostDeposit = PostDeposit<T>;
    type PreTransfer = PreTransfer<T>;
    type PostTransfer = PostTransfer<T>;
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

impl orml_tokens::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type Amount = RawAmount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type CurrencyHooks = CurrencyHooks<Test>;
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = Everything;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
}

pub type SignedFixedPoint = FixedI128;
pub type SignedInner = i128;
pub type UnsignedFixedPoint = FixedU128;

pub struct CurrencyConvert;

#[cfg_attr(test, mockable)]
impl OracleApi<Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(amount: &Amount<Test>, to: CurrencyId) -> Result<Amount<Test>, DispatchError> {
        Ok(amount.clone()) // exchange rate simulated to 1:1
    }
}

type Conversion = currency::CurrencyConvert<Test, CurrencyConvert, Loans>;

pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(DOT);
pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(INTR);
pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);

parameter_types! {
    pub const GetCollateralCurrencyId: CurrencyId = DEFAULT_COLLATERAL_CURRENCY;
    pub const GetNativeCurrencyId: CurrencyId = DEFAULT_NATIVE_CURRENCY;
    pub const GetWrappedCurrencyId: CurrencyId = DEFAULT_WRAPPED_CURRENCY;
}

impl currency::Config for Test {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetCollateralCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = Conversion;
}

impl pallet_utility::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
}

impl oracle::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type OnExchangeRateChange = ();
    type WeightInfo = ();
}

impl security::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}

parameter_types! {
    pub const LoansPalletId: PalletId = PalletId(*b"par/loan");
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = LoansPalletId;
    type ReserveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type UnixTime = TimestampPallet;
    type RewardAssetId = GetNativeCurrencyId;
    type ReferenceAssetId = GetWrappedCurrencyId;
    type OnExchangeRateChange = ();
}

pub const LEND_DOT: CurrencyId = LendToken(1);
pub const LEND_KINT: CurrencyId = LendToken(2);
pub const LEND_KSM: CurrencyId = LendToken(3);
pub const LEND_KBTC: CurrencyId = LendToken(4);
pub const LEND_IBTC: CurrencyId = LendToken(5);

pub const DEFAULT_MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000_000; // 1
pub const DEFAULT_MIN_EXCHANGE_RATE: u128 = 20_000_000_000_000_000; // 0.02

#[cfg(test)]
pub fn with_price(
    maybe_currency_price: Option<(CurrencyId, FixedU128)>,
) -> impl Fn(&Amount<Test>, CurrencyId) -> MockResult<(&Amount<Test>, CurrencyId), Result<Amount<Test>, DispatchError>>
{
    move |amount: &Amount<Test>, to: CurrencyId| {
        let (custom_currency, custom_price) = maybe_currency_price.unwrap_or((amount.currency(), FixedU128::one()));
        match (amount.currency(), to) {
            currencies if currencies == (custom_currency, DEFAULT_WRAPPED_CURRENCY) => {
                let fixed_point_amount = amount.to_unsigned_fixed_point().unwrap();
                let new_amount = fixed_point_amount.mul(custom_price);
                return MockResult::Return(Amount::from_unsigned_fixed_point(new_amount, to));
            }
            currencies if currencies == (DEFAULT_WRAPPED_CURRENCY, custom_currency) => {
                let fixed_point_amount = amount.to_unsigned_fixed_point().unwrap();
                let new_amount = fixed_point_amount.div(custom_price);
                return MockResult::Return(Amount::from_unsigned_fixed_point(new_amount, to));
            }
            // The default is a 1:1 exchange rate
            (_, currency) if currency == DEFAULT_WRAPPED_CURRENCY => {
                return MockResult::Return(Ok(Amount::new(amount.amount(), DEFAULT_WRAPPED_CURRENCY)));
            }
            (currency, x) if currency == DEFAULT_WRAPPED_CURRENCY => {
                return MockResult::Return(Ok(Amount::new(amount.amount(), x)));
            }
            (a, b) if a == b => {
                return MockResult::Return(Ok(amount.clone()));
            }
            (_, _) => return MockResult::Return(Err(Error::<Test>::InvalidExchangeRate.into())),
        }
    }
}

#[cfg(test)]
pub(crate) fn set_mock_balances() {
    Tokens::set_balance(RuntimeOrigin::root(), ALICE, Token(KSM), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), ALICE, Token(DOT), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), ALICE, Token(KBTC), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), ALICE, Token(IBTC), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), BOB, Token(KSM), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), BOB, Token(DOT), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), BOB, Token(KBTC), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), DAVE, Token(DOT), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), DAVE, Token(KBTC), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), DAVE, Token(KINT), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(RuntimeOrigin::root(), DAVE, Token(INTR), 1_000_000_000_000_000, 0).unwrap();
    Tokens::set_balance(
        RuntimeOrigin::root(),
        whitelisted_caller(),
        Token(KINT),
        1_000_000_000_000_000,
        0,
    )
    .unwrap();
}

#[cfg(test)]
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    use mocktopus::mocking::Mockable;

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    GenesisBuild::<Test>::assimilate_storage(
        &loans::GenesisConfig {
            max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
            min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
        },
        &mut t,
    )
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        // Init assets
        set_mock_balances();

        // Set exchange rate with the reference currency to the default value
        CurrencyConvert::convert.mock_safe(with_price(None));
        // Init Markets
        Loans::add_market(RuntimeOrigin::root(), Token(DOT), market_mock(LEND_DOT)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), Token(DOT)).unwrap();
        Loans::add_market(RuntimeOrigin::root(), Token(KINT), market_mock(LEND_KINT)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), Token(KINT)).unwrap();
        Loans::add_market(RuntimeOrigin::root(), Token(KSM), market_mock(LEND_KSM)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), Token(KSM)).unwrap();
        Loans::add_market(RuntimeOrigin::root(), Token(KBTC), market_mock(LEND_KBTC)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), Token(KBTC)).unwrap();
        Loans::add_market(RuntimeOrigin::root(), Token(IBTC), market_mock(LEND_IBTC)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), Token(IBTC)).unwrap();

        System::set_block_number(0);
        Security::set_active_block_number(1);
        TimestampPallet::set_timestamp(6000);
    });
    ext
}

#[cfg(test)]
pub(crate) fn new_test_ext_no_markets() -> sp_io::TestExternalities {
    use mocktopus::mocking::Mockable;

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    GenesisBuild::<Test>::assimilate_storage(
        &loans::GenesisConfig {
            max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
            min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
        },
        &mut t,
    )
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        // Init assets
        set_mock_balances();

        // Set exchange rate with the reference currency to the default value
        CurrencyConvert::convert.mock_safe(with_price(None));

        System::set_block_number(0);
        TimestampPallet::set_timestamp(6000);
    });
    ext
}

/// Progress to the given block, and then finalize the block.
pub(crate) fn _run_to_block(n: BlockNumber) {
    Loans::on_finalize(System::block_number());
    for b in (System::block_number() + 1)..=n {
        System::set_block_number(b);
        Loans::on_initialize(b);
        TimestampPallet::set_timestamp(6000 * b);
        if b != n {
            Loans::on_finalize(b);
        }
    }
}

pub fn almost_equal(target: u128, value: u128) -> bool {
    let target = target as i128;
    let value = value as i128;
    let diff = (target - value).abs() as u128;
    diff < micro_unit(1)
}

pub fn accrue_interest_per_block(asset_id: CurrencyId, block_delta_secs: u64, run_to_block: u64) {
    for i in 1..run_to_block {
        TimestampPallet::set_timestamp(6000 + (block_delta_secs * 1000 * i));
        Loans::accrue_interest(asset_id).unwrap();
    }
}

pub fn unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(12))
}

pub fn milli_unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(9))
}

pub fn micro_unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(6))
}

pub fn million_unit(d: u128) -> u128 {
    unit(d) * 10_u128.pow(6)
}

pub const fn market_mock(lend_token_id: CurrencyId) -> Market<Balance> {
    Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        state: MarketState::Pending,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        lend_token_id,
    }
}

pub const MARKET_MOCK: Market<Balance> = market_mock(LendToken(1200));

pub const ACTIVE_MARKET_MOCK: Market<Balance> = {
    let mut market = MARKET_MOCK;
    market.state = MarketState::Active;
    market
};
