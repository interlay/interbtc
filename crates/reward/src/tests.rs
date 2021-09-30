/// Tests for Reward
use crate::mock::*;
use frame_support::{assert_err, assert_ok};
use rand::Rng;

// type Event = crate::Event<Test>;

macro_rules! fixed {
    ($amount:expr) => {
        sp_arithmetic::FixedI128::from($amount)
    };
}

#[test]
fn should_distribute_rewards_equally() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(&BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(100)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 50);
        assert_ok!(Reward::compute_reward(INTERBTC, &BOB), 50);
    })
}

#[test]
fn should_distribute_uneven_rewards_equally() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(&BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(451)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 225);
        assert_ok!(Reward::compute_reward(INTERBTC, &BOB), 225);
    })
}

#[test]
fn should_not_update_previous_rewards() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(40)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(1000)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 1000);

        assert_ok!(Reward::deposit_stake(&BOB, fixed!(20)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 1000);
        assert_ok!(Reward::compute_reward(INTERBTC, &BOB), 0);
    })
}

#[test]
fn should_withdraw_reward() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(45)));
        assert_ok!(Reward::deposit_stake(&BOB, fixed!(55)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(2344)));
        assert_ok!(Reward::compute_reward(INTERBTC, &BOB), 1289);
        assert_ok!(Reward::withdraw_reward(INTERBTC, &ALICE), 1054);
        assert_ok!(Reward::compute_reward(INTERBTC, &BOB), 1289);
    })
}

#[test]
fn should_withdraw_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(1312)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(4242)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 4242);
        assert_ok!(Reward::withdraw_stake(&ALICE, fixed!(1312)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 4242);
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(2000)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 2000);
        assert_err!(
            Reward::withdraw_stake(&ALICE, fixed!(200)),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_deposit_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(25)));
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(25)));
        assert_eq!(Reward::stake(INTERBTC, &ALICE), fixed!(50));
        assert_ok!(Reward::deposit_stake(&BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(1000)));
        assert_ok!(Reward::compute_reward(INTERBTC, &ALICE), 500);
    })
}

#[test]
fn should_not_distribute_rewards_without_stake() {
    run_test(|| {
        assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(1000)), fixed!(1000));
        assert_eq!(Reward::total_rewards(INTERBTC), fixed!(0));
    })
}

#[test]
fn should_distribute_with_many_rewards() {
    // test that reward tally doesn't overflow
    run_test(|| {
        let mut rng = rand::thread_rng();
        assert_ok!(Reward::deposit_stake(&ALICE, fixed!(9230404)));
        assert_ok!(Reward::deposit_stake(&BOB, fixed!(234234444)));
        for _ in 0..30 {
            // NOTE: this will overflow compute_reward with > u32
            assert_ok!(Reward::distribute_reward(INTERBTC, fixed!(rng.gen::<u32>() as i128)));
        }
        let alice_reward = Reward::compute_reward(INTERBTC, &ALICE).unwrap();
        assert_ok!(Reward::withdraw_reward(INTERBTC, &ALICE), alice_reward);
        let bob_reward = Reward::compute_reward(INTERBTC, &BOB).unwrap();
        assert_ok!(Reward::withdraw_reward(INTERBTC, &BOB), bob_reward);
    })
}
