use crate::{mock::*, RewardSchedule, RewardSchedules};
use frame_support::assert_ok;
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
        minimum_stake: 5000,
        start_height: 200,
        period: 10,
        period_count: 100,
        per_period: 1000,
    };

    assert_eq!(reward_schedule.total().unwrap(), 100000);
    assert!(!reward_schedule.is_ready(50, 6000), "before start height");
    assert!(!reward_schedule.is_ready(300, 3000), "insufficient total stake");
    assert!(!reward_schedule.is_ready(301, 7000), "not period");
    assert!(reward_schedule.is_ready(300, 7000), "should be ready");
}

#[test]
fn should_create_and_remove_reward_schedule() {
    run_test(|| {
        let pool_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_schedule = RewardSchedule {
            minimum_stake: 0,
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
        assert_ok!(Farming::create_reward_schedule(
            RuntimeOrigin::root(),
            pool_id,
            CurrencyId::Token(INTR),
            reward_schedule,
        ));

        // check pool balance
        assert_eq!(
            Tokens::total_balance(CurrencyId::Token(INTR), &Farming::pool_account_id(&pool_id)),
            total_amount
        );

        // deleting a reward pool should transfer back to treasury
        assert_ok!(Farming::remove_reward_schedule(
            RuntimeOrigin::root(),
            pool_id,
            CurrencyId::Token(INTR),
        ));

        // check treasury balance
        assert_eq!(
            Tokens::total_balance(CurrencyId::Token(INTR), &Farming::treasury_account_id()),
            total_amount
        );
    })
}

#[test]
fn should_deposit_stake_and_claim_reward() {
    run_test(|| {
        let reward_currency = CurrencyId::Token(INTR);
        let pool_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let pool_tokens = 1000;

        // setup basic reward schedule
        let reward_schedule = RewardSchedule {
            minimum_stake: 0,
            start_height: 0,
            period: 1, // simulate per block
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Farming::treasury_account_id(),
            reward_currency,
            total_amount,
            0
        ));

        assert_ok!(Farming::create_reward_schedule(
            RuntimeOrigin::root(),
            pool_id,
            reward_currency,
            reward_schedule.clone(),
        ));

        // mint and deposit stake
        let account_id = 0;
        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            account_id,
            pool_id,
            pool_tokens,
            0
        ));

        assert_ok!(Farming::deposit(
            RuntimeOrigin::signed(account_id),
            pool_id,
            pool_tokens
        ));

        // check that we distribute per period
        assert_ok!(Farming::begin_block(1));
        assert_emitted!(Event::RewardDistributed {
            pool_id,
            currency_id: reward_currency,
            amount: reward_schedule.per_period,
        });
        assert_eq!(
            RewardSchedules::<Test>::get(pool_id, reward_currency).map(|x| x.period_count),
            Some(reward_schedule.period_count - 1)
        );

        // withdraw reward
        assert_ok!(Farming::claim(
            RuntimeOrigin::signed(account_id),
            pool_id,
            reward_currency,
        ));
        // only one account with stake
        // so they get all rewards
        assert_eq!(
            Tokens::total_balance(reward_currency, &account_id),
            reward_schedule.per_period
        );
    })
}
