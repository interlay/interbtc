//! Loans pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::{AccountBorrows, Pallet as Loans};

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::{self, RawOrigin as SystemOrigin};
use primitives::{
    Balance,
    CurrencyId::{self, Token},
    CDOT as CDOT_CURRENCY, CKBTC as CKBTC_CURRENCY, CKSM as CKSM_CURRENCY, DOT as DOT_CURRENCY, KBTC as KBTC_CURRENCY,
    KINT as KINT_CURRENCY, KSM as KSM_CURRENCY,
};
use rate_model::{InterestRateModel, JumpModel};
use sp_std::prelude::*;

const SEED: u32 = 0;

const KSM: CurrencyId = Token(KSM_CURRENCY);
const KBTC: CurrencyId = Token(KBTC_CURRENCY);
const CKSM: CurrencyId = Token(CKSM_CURRENCY);
const CKBTC: CurrencyId = Token(CKBTC_CURRENCY);
const DOT: CurrencyId = Token(DOT_CURRENCY);
const CDOT: CurrencyId = Token(CDOT_CURRENCY);
const KINT: CurrencyId = Token(KINT_CURRENCY);

const RATE_MODEL_MOCK: InterestRateModel = InterestRateModel::Jump(JumpModel {
    base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
    jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
    full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
    jump_utilization: Ratio::from_percent(80),
});

fn market_mock<T: Config>() -> Market<BalanceOf<T>> {
    Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        state: MarketState::Active,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        ptoken_id: CurrencyId::Token(CKBTC_CURRENCY),
    }
}

fn pending_market_mock<T: Config>(ptoken_id: CurrencyId) -> Market<BalanceOf<T>> {
    let mut market = market_mock::<T>();
    market.state = MarketState::Pending;
    market.ptoken_id = ptoken_id;
    market
}

fn transfer_initial_balance<T: Config + orml_tokens::Config<CurrencyId = CurrencyId, Balance = Balance>>(
    caller: T::AccountId,
) {
    let account_id = T::Lookup::unlookup(caller.clone());
    orml_tokens::Pallet::<T>::set_balance(
        SystemOrigin::Root.into(),
        account_id.clone(),
        CKSM,
        10_000_000_000_000_u128,
        0_u128,
    )
    .unwrap();
    orml_tokens::Pallet::<T>::set_balance(
        SystemOrigin::Root.into(),
        account_id.clone(),
        KBTC,
        10_000_000_000_000_u128,
        0_u128,
    )
    .unwrap();

    orml_tokens::Pallet::<T>::set_balance(
        SystemOrigin::Root.into(),
        account_id.clone(),
        KSM,
        10_000_000_000_000_u128,
        0_u128,
    )
    .unwrap();

    orml_tokens::Pallet::<T>::set_balance(
        SystemOrigin::Root.into(),
        account_id,
        CDOT,
        10_000_000_000_000_u128,
        0_u128,
    )
    .unwrap();
}

fn set_account_borrows<T: Config>(
    who: T::AccountId,
    asset_id: AssetIdOf<T>,
    borrow_balance: BalanceOf<T>,
) {
    AccountBorrows::<T>::insert(
        asset_id,
        &who,
        BorrowSnapshot {
            principal: borrow_balance,
            borrow_index: Rate::one(),
        },
    );
    TotalBorrows::<T>::insert(asset_id, borrow_balance);
    T::Assets::burn_from(asset_id, &who, borrow_balance).unwrap();
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
    where_clause {
        where
            T: orml_tokens::Config<CurrencyId = CurrencyId, Balance = Balance>
    }

    add_market {
    }: _(SystemOrigin::Root, DOT, pending_market_mock::<T>(CDOT))
    verify {
        assert_last_event::<T>(Event::<T>::NewMarket(DOT, pending_market_mock::<T>(CDOT)).into());
    }

    activate_market {
    }: _(SystemOrigin::Root, DOT)
    verify {
        assert_last_event::<T>(Event::<T>::ActivatedMarket(DOT).into());
    }

    update_rate_model {
    }: _(SystemOrigin::Root, KBTC, RATE_MODEL_MOCK)
    verify {
        let mut market = pending_market_mock::<T>(CKBTC);
        market.rate_model = RATE_MODEL_MOCK;
        assert_last_event::<T>(Event::<T>::UpdatedMarket(KBTC, market).into());
    }

    update_market {
    }: _(
        SystemOrigin::Root,
        KSM,
        Some(Ratio::from_percent(50)),
        Some(Ratio::from_percent(55)),
        Some(Ratio::from_percent(50)),
        Some(Ratio::from_percent(15)),
        Some(Ratio::from_percent(3)),
        Some(Rate::from_inner(Rate::DIV / 100 * 110)),
        Some(1_000_000_000_000_000_000_000u128),
        Some(1_000_000_000_000_000_000_000u128)
    )
    verify {
        let mut market = pending_market_mock::<T>(CKSM);
        market.reserve_factor = Ratio::from_percent(50);
        market.close_factor = Ratio::from_percent(15);
        assert_last_event::<T>(Event::<T>::UpdatedMarket(KSM, market).into());
    }

    force_update_market {
    }: _(SystemOrigin::Root,KBTC, pending_market_mock::<T>(CKBTC))
    verify {
        assert_last_event::<T>(Event::<T>::UpdatedMarket(KBTC, pending_market_mock::<T>(CKBTC)).into());
    }

    add_reward {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
    }: _(SystemOrigin::Signed(caller.clone()), 1_000_000_000_000_u128)
    verify {
        assert_last_event::<T>(Event::<T>::RewardAdded(caller, 1_000_000_000_000_u128).into());
    }

    withdraw_missing_reward {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        let receiver = T::Lookup::unlookup(caller.clone());
    }: _(SystemOrigin::Root, receiver, 500_000_000_000_u128)
    verify {
        assert_last_event::<T>(Event::<T>::RewardWithdrawn(caller, 500_000_000_000_u128).into());
    }

    update_market_reward_speed {
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
    }: _(SystemOrigin::Root, KBTC, Some(1_000_000), Some(1_000_000))
    verify {
        assert_last_event::<T>(Event::<T>::MarketRewardSpeedUpdated(KBTC, 1_000_000, 1_000_000).into());
    }

    claim_reward {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, 100_000_000));
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        assert_ok!(Loans::<T>::update_market_reward_speed(SystemOrigin::Root.into(), KBTC, Some(1_000_000), Some(1_000_000)));
        let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
        frame_system::Pallet::<T>::set_block_number(target_height);
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {
        assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
    }

    claim_reward_for_market {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, 100_000_000));
        assert_ok!(Loans::<T>::add_reward(SystemOrigin::Signed(caller.clone()).into(), 1_000_000_000_000_u128));
        assert_ok!(Loans::<T>::update_market_reward_speed(SystemOrigin::Root.into(), KBTC, Some(1_000_000), Some(1_000_000)));
        let target_height = frame_system::Pallet::<T>::block_number().saturating_add(One::one());
        frame_system::Pallet::<T>::set_block_number(target_height);
    }: _(SystemOrigin::Signed(caller.clone()), KBTC)
    verify {
        assert_last_event::<T>(Event::<T>::RewardPaid(caller, 1_000_000).into());
    }


    mint {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        let amount: u32 = 100_000;
    }: _(SystemOrigin::Signed(caller.clone()), KBTC, amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Deposited(caller, KBTC, amount.into()).into());
    }

    borrow {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), KBTC, true));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC, borrowed_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Borrowed(caller, KBTC, borrowed_amount.into()).into());
    }

    redeem {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 100_000_000;
        let redeem_amount: u32 = 100_000;
        // assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC, redeem_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::Redeemed(caller, KBTC, redeem_amount.into()).into());
    }

    redeem_all {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 100_000_000;
        // assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC)
    verify {
        assert_last_event::<T>(Event::<T>::Redeemed(caller, KBTC, deposit_amount.into()).into());
    }

    repay_borrow {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        let repay_amount: u32 = 100;
        // assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), KBTC, true));
        assert_ok!(Loans::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), KBTC, borrowed_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC, repay_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, KBTC, repay_amount.into()).into());
    }

    repay_borrow_all {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 100_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));

        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(caller.clone()).into(), KBTC, true));
        assert_ok!(Loans::<T>::borrow(SystemOrigin::Signed(caller.clone()).into(), KBTC, borrowed_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC)
    verify {
        assert_last_event::<T>(Event::<T>::RepaidBorrow(caller, KBTC, borrowed_amount.into()).into());
    }

    collateral_asset {
        let caller: T::AccountId = whitelisted_caller();
        transfer_initial_balance::<T>(caller.clone());
        let deposit_amount: u32 = 200_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(caller.clone()).into(), KBTC, deposit_amount.into()));
    }: _(SystemOrigin::Signed(caller.clone()), KBTC, true)
    verify {
        assert_last_event::<T>(Event::<T>::CollateralAssetAdded(caller, KBTC).into());
    }

    liquidate_borrow {
        let alice: T::AccountId = account("Sample", 100, SEED);
        let bob: T::AccountId = account("Sample", 101, SEED);
        transfer_initial_balance::<T>(alice.clone());
        transfer_initial_balance::<T>(bob.clone());
        let deposit_amount: u32 = 200_000_000;
        let borrowed_amount: u32 = 200_000_000;
        let liquidate_amount: u32 = 100_000_000;
        let incentive_amount: u32 = 110_000_000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), CDOT, pending_market_mock::<T>(KINT)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), CDOT));
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KSM, pending_market_mock::<T>(CKSM)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KSM));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(bob.clone()).into(), KSM, deposit_amount.into()));
        assert_ok!(Loans::<T>::mint(SystemOrigin::Signed(alice.clone()).into(), CDOT, deposit_amount.into()));
        assert_ok!(Loans::<T>::collateral_asset(SystemOrigin::Signed(alice.clone()).into(), CDOT, true));
        set_account_borrows::<T>(alice.clone(), KSM, borrowed_amount.into());
    }: _(SystemOrigin::Signed(bob.clone()), alice.clone(), KSM, liquidate_amount.into(), CDOT)
    verify {
        assert_last_event::<T>(Event::<T>::LiquidatedBorrow(bob.clone(), alice.clone(), KSM, CDOT, liquidate_amount.into(), incentive_amount.into()).into());
    }

    add_reserves {
        let caller: T::AccountId = whitelisted_caller();
        let payer = T::Lookup::unlookup(caller.clone());
        transfer_initial_balance::<T>(caller.clone());
        let amount: u32 = 2000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
    }: _(SystemOrigin::Root, payer, KBTC, amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::ReservesAdded(caller, KBTC, amount.into(), amount.into()).into());
    }

    reduce_reserves {
        let caller: T::AccountId = whitelisted_caller();
        let payer = T::Lookup::unlookup(caller.clone());
        transfer_initial_balance::<T>(caller.clone());
        let add_amount: u32 = 2000;
        let reduce_amount: u32 = 1000;
        assert_ok!(Loans::<T>::add_market(SystemOrigin::Root.into(), KBTC, pending_market_mock::<T>(CKBTC)));
        assert_ok!(Loans::<T>::activate_market(SystemOrigin::Root.into(), KBTC));
        assert_ok!(Loans::<T>::add_reserves(SystemOrigin::Root.into(), payer.clone(), KBTC, add_amount.into()));
    }: _(SystemOrigin::Root, payer, KBTC, reduce_amount.into())
    verify {
        assert_last_event::<T>(Event::<T>::ReservesReduced(caller, KBTC, reduce_amount.into(), (add_amount-reduce_amount).into()).into());
    }
}

// impl_benchmark_test_suite!(Loans, crate::mock::new_test_ext(), crate::mock::Test);
