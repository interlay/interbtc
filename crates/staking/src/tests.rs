/// Tests for Staking
use crate::mock::*;
use frame_support::{assert_err, assert_ok};

// type Event = crate::Event<Test>;

macro_rules! fixed {
    ($amount:expr) => {
        sp_arithmetic::FixedI128::from($amount)
    };
}

#[test]
fn should_stake_and_earn_rewards() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(50)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &BOB, fixed!(50)));
        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 50);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 50);
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(20)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 40);
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &BOB), 40);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 50);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 50);
    })
}

#[test]
fn should_stake_and_distribute_and_withdraw() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(10000)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &BOB, fixed!(10000)));

        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(1000)));
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 500);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 500);

        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(50)));
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(50)));

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(1000)));
        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(1000)));

        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 1023);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 976);

        assert_ok!(Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(10000)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 950);

        assert_ok!(Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(950)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 0);

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &BOB, fixed!(10000)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &BOB), 19950);

        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(1000)));
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(10000)));

        assert_ok!(Staking::compute_stake(DOT, &VAULT, &BOB), 9950);

        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 1023);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 1976);
    })
}

#[test]
fn should_stake_and_withdraw_rewards() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(100)));
        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 100);
        assert_ok!(Staking::withdraw_reward(DOT, &VAULT, &ALICE), 100);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 0);
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(100)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 100);
        assert_err!(
            Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(200)),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient_after_slashing() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(100)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 100);
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_stake(DOT, &VAULT, &ALICE), 0);
        assert_err!(
            Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(100)),
            TestError::InsufficientFunds
        );
    })
}
