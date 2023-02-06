use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Hooks};
use orml_traits::MultiCurrency;

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Farming($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

const POOL_CURRENCY_ID: CurrencyId = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
const REWARD_CURRENCY_ID: CurrencyId = CurrencyId::Token(INTR);

#[test]
fn should_create_and_remove_reward_schedule() {
    run_test(|| {
        let reward_schedule = RewardSchedule {
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            TreasuryAccountId::get(),
            REWARD_CURRENCY_ID,
            total_amount,
            0
        ));

        // creating a reward pool should transfer from treasury
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
            reward_schedule.period_count,
            reward_schedule.total().unwrap(),
        ));

        // check pool balance
        assert_eq!(
            Tokens::total_balance(REWARD_CURRENCY_ID, &Farming::pool_account_id(&POOL_CURRENCY_ID)),
            total_amount
        );

        // deleting a reward pool should transfer back to treasury
        assert_ok!(Farming::remove_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
        ));

        // check treasury balance
        assert_eq!(
            Tokens::total_balance(REWARD_CURRENCY_ID, &TreasuryAccountId::get()),
            total_amount
        );

        assert_emitted!(Event::RewardScheduleUpdated {
            pool_currency_id: POOL_CURRENCY_ID,
            reward_currency_id: REWARD_CURRENCY_ID,
            period_count: 0,
            per_period: 0,
        });
    })
}

#[test]
fn should_overwrite_existing_schedule() {
    run_test(|| {
        let reward_schedule_1 = RewardSchedule {
            period_count: 200,
            per_period: 20,
        };
        let reward_schedule_2 = RewardSchedule {
            period_count: 100,
            per_period: 10,
        };
        let total_amount = reward_schedule_1.total().unwrap() + reward_schedule_2.total().unwrap();
        let total_period_count = reward_schedule_1.period_count + reward_schedule_2.period_count;
        let total_reward_per_period = total_amount / total_period_count as u128;

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            TreasuryAccountId::get(),
            REWARD_CURRENCY_ID,
            total_amount,
            0
        ));

        // create first reward schedule
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
            reward_schedule_1.period_count,
            reward_schedule_1.total().unwrap(),
        ));

        // check pool balance
        assert_eq!(
            Tokens::total_balance(REWARD_CURRENCY_ID, &Farming::pool_account_id(&POOL_CURRENCY_ID)),
            reward_schedule_1.total().unwrap(),
        );

        // overwrite second reward schedule
        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
            reward_schedule_2.period_count,
            reward_schedule_2.total().unwrap(),
        ));

        // check pool balance now includes both
        assert_eq!(
            Tokens::total_balance(REWARD_CURRENCY_ID, &Farming::pool_account_id(&POOL_CURRENCY_ID)),
            total_amount,
        );

        assert_emitted!(Event::RewardScheduleUpdated {
            pool_currency_id: POOL_CURRENCY_ID,
            reward_currency_id: REWARD_CURRENCY_ID,
            period_count: total_period_count,
            per_period: total_reward_per_period,
        });
    })
}

fn mint_and_deposit(account_id: AccountId, amount: Balance) {
    assert_ok!(Tokens::set_balance(
        RuntimeOrigin::root(),
        account_id,
        POOL_CURRENCY_ID,
        amount,
        0
    ));

    assert_ok!(Farming::deposit(RuntimeOrigin::signed(account_id), POOL_CURRENCY_ID,));
}

#[test]
fn should_deposit_and_withdraw_stake() {
    run_test(|| {
        let pool_tokens = 1000;
        let account_id = 0;

        let reward_schedule = RewardSchedule {
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            TreasuryAccountId::get(),
            REWARD_CURRENCY_ID,
            total_amount,
            0
        ));

        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
            reward_schedule.period_count,
            reward_schedule.total().unwrap(),
        ));

        // mint and deposit stake
        mint_and_deposit(account_id, pool_tokens);

        // can't withdraw more stake than reserved
        let withdraw_amount = pool_tokens * 2;
        assert_err!(
            Farming::withdraw(RuntimeOrigin::signed(account_id), POOL_CURRENCY_ID, withdraw_amount),
            TestError::InsufficientStake
        );

        // only withdraw half of deposit
        let withdraw_amount = pool_tokens / 2;
        assert_ok!(Farming::withdraw(
            RuntimeOrigin::signed(account_id),
            POOL_CURRENCY_ID,
            withdraw_amount
        ));
        assert_eq!(Tokens::free_balance(POOL_CURRENCY_ID, &account_id), withdraw_amount);
    })
}

#[test]
fn should_deposit_stake_and_claim_reward() {
    run_test(|| {
        let pool_tokens = 1000;
        let account_id = 0;

        // setup basic reward schedule
        let reward_schedule = RewardSchedule {
            period_count: 100,
            per_period: 1000,
        };
        let total_amount = reward_schedule.total().unwrap();

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            TreasuryAccountId::get(),
            REWARD_CURRENCY_ID,
            total_amount,
            0
        ));

        assert_ok!(Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
            reward_schedule.period_count,
            reward_schedule.total().unwrap(),
        ));

        // mint and deposit stake
        mint_and_deposit(account_id, pool_tokens);

        // check that we distribute per period
        Farming::on_initialize(10);
        assert_emitted!(Event::RewardDistributed {
            pool_currency_id: POOL_CURRENCY_ID,
            reward_currency_id: REWARD_CURRENCY_ID,
            amount: reward_schedule.per_period,
        });
        assert_eq!(
            RewardSchedules::<Test>::get(POOL_CURRENCY_ID, REWARD_CURRENCY_ID).period_count,
            reward_schedule.period_count - 1
        );

        // withdraw reward
        assert_ok!(Farming::claim(
            RuntimeOrigin::signed(account_id),
            POOL_CURRENCY_ID,
            REWARD_CURRENCY_ID,
        ));
        // only one account with stake so they get all rewards
        assert_eq!(
            Tokens::free_balance(REWARD_CURRENCY_ID, &account_id),
            reward_schedule.per_period
        );
    })
}
