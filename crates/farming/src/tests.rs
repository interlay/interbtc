use crate::{mock::*, RewardSchedule, RewardSchedules};
use frame_support::{assert_err, assert_ok};
use orml_traits::MultiCurrency;

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Farming($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

#[test]
fn should_validate_reward_schedule() {
    let reward_schedule = RewardSchedule::<BlockNumber, Balance> {
        start_height: 200,
        period: 10,
        period_count: 100,
        per_period: 1000,
    };

    assert_eq!(reward_schedule.total().unwrap(), 100000);
    assert!(!reward_schedule.is_ready(50), "before start height");
    assert!(!reward_schedule.is_ready(301), "not period");
    assert!(reward_schedule.is_ready(300), "should be ready");
}

#[test]
fn should_create_and_remove_reward_schedule() {
    run_test(|| {
        let pool_currency_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_schedule = RewardSchedule {
            start_height: 0,
            period: 10,
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Farming::treasury_account_id(),
            CurrencyId::Token(INTR),
            total_amount,
            0
        ));

        // creating a reward pool should transfer from treasury
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            CurrencyId::Token(INTR),
            reward_schedule,
        ));

        // check pool balance
        assert_eq!(
            Tokens::total_balance(CurrencyId::Token(INTR), &Farming::pool_account_id(&pool_currency_id)),
            total_amount
        );

        // deleting a reward pool should transfer back to treasury
        assert_ok!(Farming::remove_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            CurrencyId::Token(INTR),
        ));

        // check treasury balance
        assert_eq!(
            Tokens::total_balance(CurrencyId::Token(INTR), &Farming::treasury_account_id()),
            total_amount
        );
    })
}

fn mint_and_deposit(account_id: AccountId, pool_currency_id: CurrencyId, amount: Balance) {
    assert_ok!(Tokens::set_balance(
        RuntimeOrigin::root(),
        account_id,
        pool_currency_id,
        amount,
        0
    ));

    assert_ok!(Farming::deposit(
        RuntimeOrigin::signed(account_id),
        pool_currency_id,
        amount
    ));
}

#[test]
fn should_overwrite_existing_schedule() {
    run_test(|| {
        let pool_currency_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_currency_id = CurrencyId::Token(INTR);
        let reward_schedule_1 = RewardSchedule {
            start_height: 10,
            period: 20,
            period_count: 30,
            per_period: 40,
        };
        let reward_schedule_2 = RewardSchedule {
            start_height: 40,
            period: 30,
            period_count: 20,
            per_period: 10,
        };
        let total_amount = reward_schedule_1.total().unwrap() + reward_schedule_2.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Farming::treasury_account_id(),
            reward_currency_id,
            total_amount,
            0
        ));

        // create first reward schedule
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            reward_currency_id,
            reward_schedule_1.clone(),
        ));

        // check pool balance
        assert_eq!(
            Tokens::total_balance(reward_currency_id, &Farming::pool_account_id(&pool_currency_id)),
            reward_schedule_1.total().unwrap(),
        );

        // add stake so we can distribute
        mint_and_deposit(0, pool_currency_id, 100);

        // overwrite second reward schedule
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            reward_currency_id,
            reward_schedule_2,
        ));

        assert_emitted!(Event::RewardDistributed {
            pool_currency_id,
            reward_currency_id,
            amount: reward_schedule_1.total().unwrap(),
        });
    })
}

#[test]
fn should_deposit_and_withdraw_stake() {
    run_test(|| {
        let pool_currency_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_currency_id = CurrencyId::Token(INTR);
        let pool_tokens = 1000;
        let account_id = 0;

        let reward_schedule = RewardSchedule {
            start_height: 0,
            period: 1,
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        // can't deposit without schedule
        assert_err!(
            Farming::deposit(RuntimeOrigin::signed(account_id), pool_currency_id, pool_tokens),
            TestError::ScheduleNotFound
        );

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Farming::treasury_account_id(),
            reward_currency_id,
            total_amount,
            0
        ));

        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            reward_currency_id,
            reward_schedule.clone(),
        ));

        // mint and deposit stake
        mint_and_deposit(account_id, pool_currency_id, pool_tokens);

        // only withdraw half of deposit
        let withdraw_amount = pool_tokens / 2;
        assert_ok!(Farming::withdraw(
            RuntimeOrigin::signed(account_id),
            pool_currency_id,
            withdraw_amount
        ));
        assert_eq!(Tokens::free_balance(pool_currency_id, &account_id), withdraw_amount);
    })
}

#[test]
fn should_deposit_stake_and_claim_reward() {
    run_test(|| {
        let pool_currency_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_currency_id = CurrencyId::Token(INTR);
        let pool_tokens = 1000;
        let account_id = 0;

        // setup basic reward schedule
        let reward_schedule = RewardSchedule {
            start_height: 0,
            period: 1, // simulate per block
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Farming::treasury_account_id(),
            reward_currency_id,
            total_amount,
            0
        ));

        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            pool_currency_id,
            reward_currency_id,
            reward_schedule.clone(),
        ));

        // mint and deposit stake
        mint_and_deposit(account_id, pool_currency_id, pool_tokens);

        // check that we distribute per period
        assert_ok!(Farming::begin_block(1));
        assert_emitted!(Event::RewardDistributed {
            pool_currency_id,
            reward_currency_id,
            amount: reward_schedule.per_period,
        });
        assert_eq!(
            RewardSchedules::<Test>::get(pool_currency_id, reward_currency_id).map(|x| x.period_count),
            Some(reward_schedule.period_count - 1)
        );

        // withdraw reward
        assert_ok!(Farming::claim(
            RuntimeOrigin::signed(account_id),
            pool_currency_id,
            reward_currency_id,
        ));
        // only one account with stake so they get all rewards
        assert_eq!(
            Tokens::free_balance(reward_currency_id, &account_id),
            reward_schedule.per_period
        );
    })
}
