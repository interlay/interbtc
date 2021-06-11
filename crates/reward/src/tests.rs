/// Tests for Reward
use crate::mock::*;
use crate::RewardPool;
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
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(50)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(100)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 50);
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &BOB), 50);
    })
}

#[test]
fn should_distribute_uneven_rewards_equally() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(50)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(451)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 225);
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &BOB), 225);
    })
}

#[test]
fn should_not_update_previous_rewards() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(40)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(1000)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 1000);

        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(20)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 1000);
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &BOB), 0);
    })
}

#[test]
fn should_withdraw_reward() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(45)));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(55)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(2344)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &BOB), 1289);
        assert_ok!(Reward::withdraw_reward(DOT, RewardPool::Global, &ALICE), 1054);
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &BOB), 1289);
    })
}

#[test]
fn should_withdraw_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(1312)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(4242)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 4242);
        assert_ok!(Reward::withdraw_stake(DOT, RewardPool::Global, &ALICE, fixed!(1312)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 4242);
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(2000)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 2000);
        assert_err!(
            Reward::withdraw_stake(DOT, RewardPool::Global, &ALICE, fixed!(200)),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_deposit_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(25)));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(25)));
        assert_eq!(Reward::get_stake(DOT, RewardPool::Global, &ALICE), fixed!(50));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(50)));
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(1000)));
        assert_ok!(Reward::compute_reward(DOT, &RewardPool::Global, &ALICE), 500);
    })
}

#[test]
fn should_not_distribute_rewards_without_stake() {
    run_test(|| {
        assert_ok!(Reward::distribute(DOT, RewardPool::Global, fixed!(1000)), fixed!(0));
        assert_eq!(Reward::total_rewards((DOT, RewardPool::Global)), fixed!(0));
    })
}

#[test]
fn should_distribute_with_many_rewards() {
    // test that reward tally doesn't overflow
    run_test(|| {
        let mut rng = rand::thread_rng();
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &ALICE, fixed!(9230404)));
        assert_ok!(Reward::deposit_stake(DOT, RewardPool::Global, &BOB, fixed!(234234444)));
        for _ in 0..30 {
            // NOTE: this will overflow compute_reward with > u32
            assert_ok!(Reward::distribute(
                DOT,
                RewardPool::Global,
                fixed!(rng.gen::<u32>() as i128)
            ));
        }
        let alice_reward = Reward::compute_reward(DOT, &RewardPool::Global, &ALICE).unwrap();
        assert_ok!(Reward::withdraw_reward(DOT, RewardPool::Global, &ALICE), alice_reward);
        let bob_reward = Reward::compute_reward(DOT, &RewardPool::Global, &BOB).unwrap();
        assert_ok!(Reward::withdraw_reward(DOT, RewardPool::Global, &BOB), bob_reward);
    })
}
