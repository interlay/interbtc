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
#[cfg_attr(rustfmt, rustfmt_skip)]
fn reproduce_live_state() {
    // This function is most useful for debugging. Keeping this test here for convenience
    // and to function as an additional regression test
    run_test(|| {
        let f = |x: i128| SignedFixedPoint::from_inner(x);
        let currency = Token(INTR);

        // state for a3eFe9M2HbAgrQrShEDH2CEvXACtzLhSf4JGkwuT9SQ1EV4ti at block 0xb47ed0e773e25c81da2cc606495ab6f716c3c2024f9beb361605860912fee652
        crate::RewardPerToken::<Test>::insert(currency, (), f(1_699_249_738_518_636_122_154_288_694));
        crate::RewardTally::<Test>::insert(currency, ((), ALICE), f(164_605_943_476_265_834_062_592_062_507_811_208));
        crate::Stake::<Test>::insert(((), ALICE), f(97_679_889_000_000_000_000_000_000));
        crate::TotalRewards::<Test>::insert(currency, f(8_763_982_459_262_268_000_000_000_000_000_000));
        crate::TotalStake::<Test>::insert((), f(2_253_803_217_000_000_000_000_000_000));

        assert_ok!(Reward::compute_reward(&(), &ALICE, currency), 1376582365513566);
    })
}

#[test]
fn should_distribute_rewards_equally() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(100)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 50);
        assert_ok!(Reward::compute_reward(&(), &BOB, Token(IBTC)), 50);
    })
}

#[test]
fn should_distribute_uneven_rewards_equally() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(50)));
        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(451)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 225);
        assert_ok!(Reward::compute_reward(&(), &BOB, Token(IBTC)), 225);
    })
}

#[test]
fn should_not_update_previous_rewards() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(40)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(1000)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 1000);

        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(20)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 1000);
        assert_ok!(Reward::compute_reward(&(), &BOB, Token(IBTC)), 0);
    })
}

#[test]
fn should_withdraw_reward() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(45)));
        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(55)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(2344)));
        assert_ok!(Reward::compute_reward(&(), &BOB, Token(IBTC)), 1289);
        assert_ok!(Reward::withdraw_reward(&(), &ALICE, Token(IBTC)), 1054);
        assert_ok!(Reward::compute_reward(&(), &BOB, Token(IBTC)), 1289);
    })
}

#[test]
fn should_withdraw_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(1312)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(4242)));
        // rounding in `CheckedDiv` loses some precision
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 4241);
        assert_ok!(Reward::withdraw_stake(&(), &ALICE, fixed!(1312)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 4241);
    })
}

#[test]
fn should_not_withdraw_stake_if_balance_insufficient() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(2000)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 2000);
        assert_err!(
            Reward::withdraw_stake(&(), &ALICE, fixed!(200)),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn should_deposit_stake() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(25)));
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(25)));
        assert_eq!(Reward::stake(&(), &ALICE), fixed!(50));
        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(50)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(1000)));
        assert_ok!(Reward::compute_reward(&(), &ALICE, Token(IBTC)), 500);
    })
}

#[test]
fn should_not_distribute_rewards_without_stake() {
    run_test(|| {
        assert_err!(
            Reward::distribute_reward(&(), Token(IBTC), fixed!(1000)),
            TestError::ZeroTotalStake
        );
        assert_eq!(Reward::total_rewards(Token(IBTC)), fixed!(0));
    })
}

#[test]
fn should_distribute_with_many_rewards() {
    // test that reward tally doesn't overflow
    run_test(|| {
        let mut rng = rand::thread_rng();
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(9230404)));
        assert_ok!(Reward::deposit_stake(&(), &BOB, fixed!(234234444)));
        for _ in 0..30 {
            // NOTE: this will overflow compute_reward with > u32
            assert_ok!(Reward::distribute_reward(
                &(),
                Token(IBTC),
                fixed!(rng.gen::<u32>() as i128)
            ));
        }
        let alice_reward = Reward::compute_reward(&(), &ALICE, Token(IBTC)).unwrap();
        assert_ok!(Reward::withdraw_reward(&(), &ALICE, Token(IBTC)), alice_reward);
        let bob_reward = Reward::compute_reward(&(), &BOB, Token(IBTC)).unwrap();
        assert_ok!(Reward::withdraw_reward(&(), &BOB, Token(IBTC)), bob_reward);
    })
}

macro_rules! assert_approx_eq {
    ($left:expr, $right:expr, $delta:expr) => {
        assert!(if $left > $right { $left - $right } else { $right - $left } <= $delta)
    };
}

#[test]
fn should_distribute_with_different_rewards() {
    run_test(|| {
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(&(), Token(IBTC), fixed!(1000)));
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(&(), Token(INTR), fixed!(1000)));
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(&(), Token(DOT), fixed!(1000)));
        assert_ok!(Reward::deposit_stake(&(), &ALICE, fixed!(100)));
        assert_ok!(Reward::distribute_reward(&(), ForeignAsset(0), fixed!(1000)));

        assert_ok!(Reward::withdraw_stake(&(), &ALICE, fixed!(300)));

        assert_approx_eq!(Reward::compute_reward(&(), &ALICE, Token(IBTC)).unwrap(), 1000, 1);
        assert_approx_eq!(Reward::compute_reward(&(), &ALICE, Token(INTR)).unwrap(), 1000, 1);
        assert_approx_eq!(Reward::compute_reward(&(), &ALICE, Token(DOT)).unwrap(), 1000, 1);
        assert_approx_eq!(Reward::compute_reward(&(), &ALICE, ForeignAsset(0)).unwrap(), 1000, 1);
    })
}
