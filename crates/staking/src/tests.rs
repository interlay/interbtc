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
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(50)));
        assert_ok!(Staking::deposit_stake(&VAULT, &BOB.account_id, fixed!(50)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 50);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 50);
        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(20)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 40);
        assert_ok!(Staking::compute_stake(&VAULT, &BOB.account_id), 40);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 50);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 50);
    })
}

#[test]
fn should_stake_and_distribute_and_withdraw() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(10000)));
        assert_ok!(Staking::deposit_stake(&VAULT, &BOB.account_id, fixed!(10000)));

        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(1000)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 500);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 500);

        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(50)));
        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(50)));

        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(1000)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(1000)));

        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 1023);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 976);

        assert_ok!(Staking::withdraw_stake(&VAULT, &ALICE.account_id, fixed!(10000), None));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 950);

        assert_ok!(Staking::withdraw_stake(&VAULT, &ALICE.account_id, fixed!(950), None));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 0);

        assert_ok!(Staking::deposit_stake(&VAULT, &BOB.account_id, fixed!(10000)));
        assert_ok!(Staking::compute_stake(&VAULT, &BOB.account_id), 19950);

        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(1000)));
        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(10000)));

        assert_ok!(Staking::compute_stake(&VAULT, &BOB.account_id), 9949);

        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 1023);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 1975);
    })
}

#[test]
fn should_stake_and_withdraw_rewards() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(100)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 100);
        assert_ok!(Staking::withdraw_reward(Token(IBTC), &VAULT, &ALICE.account_id), 100);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 0);
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(100)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 100);
        assert_err!(
            Staking::withdraw_stake(&VAULT, &ALICE.account_id, fixed!(200), None),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient_after_slashing() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(100)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 100);
        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 0);
        assert_err!(
            Staking::withdraw_stake(&VAULT, &ALICE.account_id, fixed!(100), None),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_force_refund() {
    run_test(|| {
        let mut nonce = Staking::nonce(&VAULT);
        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(100)));
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(100)));
        assert_ok!(Staking::slash_stake(Token(IBTC), &VAULT, fixed!(100)));
        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(100)));
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(10)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(100)));

        // vault stake & rewards pre-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &VAULT.account_id), 150);
        assert_ok!(
            Staking::compute_reward_at_index(nonce, Token(IBTC), &VAULT, &VAULT.account_id),
            71
        );

        // alice stake & rewards pre-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &ALICE.account_id), 60);
        assert_ok!(
            Staking::compute_reward_at_index(nonce, Token(IBTC), &VAULT, &ALICE.account_id),
            28
        );

        assert_ok!(Staking::force_refund(&VAULT));

        nonce = Staking::nonce(&VAULT);

        // vault stake & rewards post-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &VAULT.account_id), 150);
        assert_ok!(
            Staking::compute_reward_at_index(nonce, Token(IBTC), &VAULT, &VAULT.account_id),
            0
        );

        assert_ok!(
            Staking::compute_reward_at_index(nonce - 1, Token(IBTC), &VAULT, &VAULT.account_id),
            71
        );

        // alice stake & rewards post-refund
        assert_ok!(Staking::compute_stake_at_index(nonce, &VAULT, &ALICE.account_id), 0);
        assert_ok!(
            Staking::compute_reward_at_index(nonce, Token(IBTC), &VAULT, &ALICE.account_id),
            0
        );

        assert_ok!(
            Staking::compute_stake_at_index(nonce - 1, &VAULT, &ALICE.account_id),
            60
        );
        assert_ok!(
            Staking::compute_reward_at_index(nonce - 1, Token(IBTC), &VAULT, &ALICE.account_id),
            28
        );
    })
}

#[test]
fn should_compute_stake_after_adjustments() {
    // this replicates a failing integration test due to repeated
    // deposits and slashing which led to incorrect stake
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(100)));
        assert_ok!(Staking::deposit_stake(
            &VAULT,
            &VAULT.account_id,
            fixed!(1152923504604516976)
        ));
        assert_ok!(Staking::slash_stake(
            Token(IBTC),
            &VAULT,
            fixed!(1152923504604516976 + 100)
        ));

        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(1_000_000)));

        assert_ok!(Staking::deposit_stake(
            &VAULT,
            &VAULT.account_id,
            fixed!(1152924504603286976)
        ));
        assert_ok!(Staking::slash_stake(
            Token(IBTC),
            &VAULT,
            fixed!(1152924504603286976 + 1_000_000)
        ));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT.account_id), 0);

        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(1_000_000)));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT.account_id), 1_000_000);
    })
}
