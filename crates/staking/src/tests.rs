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
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 40);
        assert_ok!(Staking::compute_stake(&VAULT, &BOB), 40);
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

        assert_ok!(Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(10000), None));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 950);

        assert_ok!(Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(950), None));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 0);

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &BOB, fixed!(10000)));
        assert_ok!(Staking::compute_stake(&VAULT, &BOB), 19950);

        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(1000)));
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(10000)));

        assert_ok!(Staking::compute_stake(&VAULT, &BOB), 9949);

        assert_ok!(Staking::compute_reward(DOT, &VAULT, &ALICE), 1023);
        assert_ok!(Staking::compute_reward(DOT, &VAULT, &BOB), 1975);
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
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 100);
        assert_err!(
            Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(200), None),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient_after_slashing() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(100)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 100);
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE), 0);
        assert_err!(
            Staking::withdraw_stake(DOT, &VAULT, &ALICE, fixed!(100), None),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_force_refund() {
    run_test(|| {
        let mut nonce = Staking::nonce(&VAULT);
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(100)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(100)));
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(100)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(100)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &ALICE, fixed!(10)));
        assert_ok!(Staking::distribute_reward(DOT, &VAULT, fixed!(100)));

        // vault stake & rewards pre-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &VAULT), 150);
        assert_ok!(Staking::compute_reward_at_index(nonce, DOT, &VAULT, &VAULT), 71);

        // alice stake & rewards pre-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &ALICE), 60);
        assert_ok!(Staking::compute_reward_at_index(nonce, DOT, &VAULT, &ALICE), 28);

        assert_ok!(Staking::force_refund(DOT, &VAULT));

        nonce = Staking::nonce(&VAULT);

        // vault stake & rewards post-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &VAULT), 150);
        assert_ok!(Staking::compute_reward_at_index(nonce, DOT, &VAULT, &VAULT), 0);

        assert_ok!(Staking::compute_reward_at_index(nonce - 1, DOT, &VAULT, &VAULT), 71);

        // alice stake & rewards post-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &ALICE), 0);
        assert_ok!(Staking::compute_reward_at_index(nonce, DOT, &VAULT, &ALICE), 0);

        assert_ok!(Staking::compute_stake_at_index(nonce - 1, &VAULT, &ALICE), 60);
        assert_ok!(Staking::compute_reward_at_index(nonce - 1, DOT, &VAULT, &ALICE), 28);
    })
}

#[test]
fn should_compute_stake_after_adjustments() {
    // this replicates a failing integration test due to repeated
    // deposits and slashing which led to incorrect stake
    run_test(|| {
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(100)));
        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(1152923504604516976)));
        assert_ok!(Staking::slash_stake(DOT, &VAULT, fixed!(1152923504604516976 + 100)));

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(1_000_000)));

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(1152924504603286976)));
        assert_ok!(Staking::slash_stake(
            DOT,
            &VAULT,
            fixed!(1152924504603286976 + 1_000_000)
        ));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT), 0);

        assert_ok!(Staking::deposit_stake(DOT, &VAULT, &VAULT, fixed!(1_000_000)));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT), 1_000_000);
    })
}
