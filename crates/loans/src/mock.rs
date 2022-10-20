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

use crate as pallet_loans;

use currency::Amount;
use frame_benchmarking::whitelisted_caller;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{Everything, SortedMembers},
    PalletId,
};
use frame_system::EnsureRoot;
use mocktopus::{macros::mockable, mocking::*};
use orml_traits::{parameter_type_with_key, DataFeeder, DataProvider, DataProviderExtended};
use primitives::{
    CurrencyId::{ForeignAsset, PToken, Token},
    Moment, PriceDetail, DOT, IBTC, INTR, KBTC, KINT, KSM,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32, FixedI128};
use sp_std::vec::Vec;
use std::{cell::RefCell, collections::HashMap};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Loans: pallet_loans::{Pallet, Storage, Call, Event<T>, Config},
        TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>},
        Currency: currency::{Pallet},
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
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
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

impl orml_tokens::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type Amount = RawAmount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type OnSlash = OnSlashHook<Test>;
    type OnDeposit = OnDepositHook<Test>;
    type OnTransfer = OnTransferHook<Test>;
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = Everything;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

pub type SignedFixedPoint = FixedI128;
pub type SignedInner = i128;
pub type UnsignedFixedPoint = FixedU128;
pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(
        amount: &currency::Amount<Test>,
        to: CurrencyId,
    ) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
        let amount = convert_to(to, amount.amount())?;
        Ok(Amount::new(amount, to))
    }
}

#[cfg_attr(test, mockable)]
pub fn convert_to(to: CurrencyId, amount: Balance) -> Result<Balance, sp_runtime::DispatchError> {
    Ok(amount) // default conversion 1:1 - overwritable with mocktopus
}

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
    type CurrencyConversion = CurrencyConvert;
}

// pallet-price is using for benchmark compilation
pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct MockDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for MockDataProvider {
    fn get(_asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
        Some(TimeStampedPrice {
            value: Price::saturating_from_integer(100),
            timestamp: 0,
        })
    }
}

impl DataProviderExtended<CurrencyId, TimeStampedPrice> for MockDataProvider {
    fn get_no_op(_key: &CurrencyId) -> Option<TimeStampedPrice> {
        None
    }

    fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
        vec![]
    }
}

impl DataFeeder<CurrencyId, TimeStampedPrice, AccountId> for MockDataProvider {
    fn feed_value(_: AccountId, _: CurrencyId, _: TimeStampedPrice) -> sp_runtime::DispatchResult {
        Ok(())
    }
}

parameter_types! {
    pub const RelayCurrency: CurrencyId = Token(KSM);
}

pub struct AliceCreatePoolOrigin;
impl SortedMembers<AccountId> for AliceCreatePoolOrigin {
    fn sorted_members() -> Vec<AccountId> {
        vec![ALICE]
    }
}

pub struct MockPriceFeeder;

impl MockPriceFeeder {
    thread_local! {
        pub static PRICES: RefCell<HashMap<CurrencyId, Option<PriceDetail>>> = {
            RefCell::new(
                // Include a foreign assets to act as a liquidation-free collateral for now.
                // TODO: Remove liquidation-free collateral
                vec![Token(KINT), Token(DOT), Token(KSM), Token(KBTC), Token(INTR), Token(IBTC), ForeignAsset(100000)]
                    .iter()
                    .map(|&x| (x, Some((Price::saturating_from_integer(1), 1))))
                    .collect()
            )
        };
    }

    pub fn set_price(asset_id: CurrencyId, price: Price) {
        Self::PRICES.with(|prices| {
            prices.borrow_mut().insert(asset_id, Some((price, 1u64)));
        });
    }

    pub fn reset() {
        Self::PRICES.with(|prices| {
            for (_, val) in prices.borrow_mut().iter_mut() {
                *val = Some((Price::saturating_from_integer(1), 1u64));
            }
        })
    }
}

impl PriceFeeder for MockPriceFeeder {
    fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
        Self::PRICES.with(|prices| {
            let p = prices.borrow();
            let v = p.get(asset_id).unwrap();
            *v
        })
    }
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}

parameter_types! {
    pub const LoansPalletId: PalletId = PalletId(*b"par/loan");
    pub const RewardAssetId: CurrencyId = Token(KINT);
}

impl Config for Test {
    type Event = Event;
    type PriceFeeder = MockPriceFeeder;
    type PalletId = LoansPalletId;
    type ReserveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type UnixTime = TimestampPallet;
    type Assets = Tokens;
    type RewardAssetId = RewardAssetId;
}

pub const CDOT: CurrencyId = PToken(1);
pub const CKINT: CurrencyId = PToken(2);
pub const CKSM: CurrencyId = PToken(3);
pub const CKBTC: CurrencyId = PToken(4);
pub const CIBTC: CurrencyId = PToken(5);

pub const DEFAULT_MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000_000; // 1
pub const DEFAULT_MIN_EXCHANGE_RATE: u128 = 20_000_000_000_000_000; // 0.02

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    GenesisBuild::<Test>::assimilate_storage(
        &pallet_loans::GenesisConfig {
            max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
            min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
        },
        &mut t,
    )
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        // Init assets

        Tokens::set_balance(Origin::root(), ALICE, Token(KSM), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), ALICE, Token(DOT), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), ALICE, Token(KBTC), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), ALICE, Token(IBTC), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), BOB, Token(KSM), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), BOB, Token(DOT), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), DAVE, Token(DOT), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), DAVE, Token(KBTC), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), DAVE, Token(KINT), 1000_000000000000, 0).unwrap();
        Tokens::set_balance(Origin::root(), whitelisted_caller(), Token(KINT), 1000_000000000000, 0).unwrap();

        MockPriceFeeder::set_price(Token(KBTC), 1.into());
        MockPriceFeeder::set_price(Token(DOT), 1.into());
        MockPriceFeeder::set_price(Token(KSM), 1.into());
        MockPriceFeeder::set_price(CDOT, 1.into());
        // Init Markets
        Loans::add_market(Origin::root(), Token(DOT), market_mock(CDOT)).unwrap();
        Loans::activate_market(Origin::root(), Token(DOT)).unwrap();
        Loans::add_market(Origin::root(), Token(KINT), market_mock(CKINT)).unwrap();
        Loans::activate_market(Origin::root(), Token(KINT)).unwrap();
        Loans::add_market(Origin::root(), Token(KSM), market_mock(CKSM)).unwrap();
        Loans::activate_market(Origin::root(), Token(KSM)).unwrap();
        Loans::add_market(Origin::root(), Token(KBTC), market_mock(CKBTC)).unwrap();
        Loans::activate_market(Origin::root(), Token(KBTC)).unwrap();
        Loans::add_market(Origin::root(), Token(IBTC), market_mock(CIBTC)).unwrap();
        Loans::activate_market(Origin::root(), Token(IBTC)).unwrap();

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

pub const fn market_mock(ptoken_id: CurrencyId) -> Market<Balance> {
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
        ptoken_id,
    }
}

pub const MARKET_MOCK: Market<Balance> = market_mock(ForeignAsset(1200));

pub const ACTIVE_MARKET_MOCK: Market<Balance> = {
    let mut market = MARKET_MOCK;
    market.state = MarketState::Active;
    market
};
