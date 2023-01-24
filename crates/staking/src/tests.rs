/// Tests for Staking
use crate::mock::*;
use frame_support::{assert_err, assert_ok};

// type Event = crate::Event<Test>;

#[macro_export]
macro_rules! fixed {
    ($amount:expr) => {
        sp_arithmetic::FixedI128::from($amount)
    };
}

#[test]
#[cfg_attr(rustfmt, rustfmt_skip)]
fn reproduce_broken_state() {
    run_test(|| {
        use crate::pallet::*;
        let account = VAULT.account_id;
        let currency = Token(INTR);
        let wrong_currency = Token(KBTC);
        
        let f = |x: i128| {
            SignedFixedPoint::from_inner(x)
        };

        // state at block 0x5aaf4dc2ca1a1043e2c37ba85a1d1487a0e4cc79d3beb30e6e646d2190223851
        RewardPerToken::<Test>::insert(currency, (0, VAULT), f(8328262397106661114));
        RewardTally::<Test>::insert(currency, (0, VAULT, account), f(594980318984627591665452302579139));
        SlashPerToken::<Test>::insert(0, VAULT,  f(10288025703175927));
        SlashTally::<Test>::insert(0, (VAULT, account), f(734987987016863199590580128394));
        Stake::<Test>::insert(0, (VAULT, account), f(71441111076342999999817587297983));
        TotalCurrentStake::<Test>::insert(0, VAULT, f(71441111076343000000000000000000));
        TotalRewards::<Test>::insert(currency, (0, VAULT), f(1135651379916000000000000000000));
        TotalStake::<Test>::insert(0, VAULT, f(71441111076342999999817587297983));

        assert_ok!(Staking::broken_slash_stake_do_not_use(wrong_currency, &VAULT, fixed!(109_808_965_219)));
        assert_ok!(Staking::broken_slash_stake_do_not_use(wrong_currency, &VAULT, fixed!(109_808_965_219)));
        assert_ok!(Staking::broken_slash_stake_do_not_use(wrong_currency, &VAULT, fixed!(1_479_196_975_788)));

        // state at block 0xe56fd1e1c66ca658284cda6334865cb3ac413fdb6a9272f00f350b6f29787ba5
        // Note: RewardPerToken is unchanged, incorrect!  
        assert_eq!(RewardPerToken::<Test>::get(currency, (0, VAULT)), f(8328262397106661114));
        assert_eq!(RewardTally::<Test>::get(currency, (0, VAULT, account)), f(594980318984627591665452302579139));
        assert_eq!(SlashPerToken::<Test>::get(0, VAULT,), f(34067259825257565));
        assert_eq!(SlashTally::<Test>::get(0, (VAULT, account)), f(734987987016863199590580128394));
        assert_eq!(Stake::<Test>::get(0, (VAULT, account)), f(71441111076342999999817587297983));
        assert_eq!(TotalCurrentStake::<Test>::get(0, VAULT), f(69742296170117000000000000000000));
        assert_eq!(TotalRewards::<Test>::get(currency, (0, VAULT)), f(1135651379916000000000000000000));
        assert_eq!(TotalStake::<Test>::get(0, VAULT), f(71441111076342999999817587297983));

        // The bug that we observed
        Staking::distribute_reward(currency, &VAULT, f(14234191584160000000000000000000)).unwrap();
        assert_eq!(Staking::withdraw_reward(currency, &VAULT, &account).unwrap(), 86015280993);
    })
}

#[test]
#[cfg_attr(rustfmt, rustfmt_skip)]
fn slash_stake_does_not_break_state() {
    run_test(|| {
        use crate::pallet::*;
        let account = VAULT.account_id;
        let currency = Token(INTR);
        
        let f = |x: i128| {
            SignedFixedPoint::from_inner(x)
        };

        // state at block 0x5aaf4dc2ca1a1043e2c37ba85a1d1487a0e4cc79d3beb30e6e646d2190223851
        RewardPerToken::<Test>::insert(currency, (0, VAULT), f(8328262397106661114));
        RewardTally::<Test>::insert(currency, (0, VAULT, account), f(594980318984627591665452302579139));
        SlashPerToken::<Test>::insert(0, VAULT,  f(10288025703175927));
        SlashTally::<Test>::insert(0, (VAULT, account), f(734987987016863199590580128394));
        Stake::<Test>::insert(0, (VAULT, account), f(71441111076342999999817587297983));
        TotalCurrentStake::<Test>::insert(0, VAULT, f(71441111076343000000000000000000));
        TotalRewards::<Test>::insert(currency, (0, VAULT), f(1135651379916000000000000000000));
        TotalStake::<Test>::insert(0, VAULT, f(71441111076342999999817587297983));

        assert_ok!(Staking::slash_stake(&VAULT, fixed!(109_808_965_219)));
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(109_808_965_219)));
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(1_479_196_975_788)));

        // state at block 0xe56fd1e1c66ca658284cda6334865cb3ac413fdb6a9272f00f350b6f29787ba5
        // NOTE: RewardPerToken is updated correctly
        // updated value in v0.9.26 since `CheckedDiv` lost precision with rounding (was 8531126040549884148)
        assert_eq!(RewardPerToken::<Test>::get(currency, (0, VAULT)), f(8531126040549884146));
        assert_eq!(RewardTally::<Test>::get(currency, (0, VAULT, account)), f(594980318984627591665452302579139));
        assert_eq!(SlashPerToken::<Test>::get(0, VAULT,), f(34067259825257565));
        assert_eq!(SlashTally::<Test>::get(0, (VAULT, account)), f(734987987016863199590580128394));
        assert_eq!(Stake::<Test>::get(0, (VAULT, account)), f(71441111076342999999817587297983));
        assert_eq!(TotalCurrentStake::<Test>::get(0, VAULT), f(69742296170117000000000000000000));
        assert_eq!(TotalRewards::<Test>::get(currency, (0, VAULT)), f(1135651379916000000000000000000));
        assert_eq!(TotalStake::<Test>::get(0, VAULT), f(71441111076342999999817587297983));

        // The bug that we observed
        Staking::distribute_reward(currency, &VAULT, f(14234191584160000000000000000000)).unwrap();
        assert_eq!(Staking::withdraw_reward(currency, &VAULT, &account).unwrap(), 14234191584160);
    })
}

#[test]
fn should_stake_and_earn_rewards() {
    run_test(|| {
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(50)));
        assert_ok!(Staking::deposit_stake(&VAULT, &BOB.account_id, fixed!(50)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(100)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 50);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 50);
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(20)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 40);
        assert_ok!(Staking::compute_stake(&VAULT, &BOB.account_id), 40);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 50);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 50);
    })
}

#[test]
fn continues_functioning_after_slash() {
    run_test(|| {
        // without the `apply_slash` in withdraw_rewards, the following sequence fails in the last step:
        // [distribute_reward, slash_stake, withdraw_reward, distribute_reward, withdraw_reward]

        // step 1: initial (normal) flow
        assert_ok!(Staking::deposit_stake(&VAULT, &ALICE.account_id, fixed!(50)));
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(10000)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 10000);

        // step 2: slash
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(30)));
        assert_ok!(Staking::compute_stake(&VAULT, &ALICE.account_id), 20);

        // step 3: withdraw rewards
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 10000);
        assert_ok!(Staking::withdraw_reward(Token(IBTC), &VAULT, &ALICE.account_id), 10000);

        // Now distribute more rewards - behavior should be back to normal.
        // The slash should not have any effect on this!
        assert_ok!(Staking::distribute_reward(Token(IBTC), &VAULT, fixed!(10000)));
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 10000);
        assert_ok!(Staking::withdraw_reward(Token(IBTC), &VAULT, &ALICE.account_id), 10000);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 0);
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

        assert_ok!(Staking::slash_stake(&VAULT, fixed!(50)));
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(50)));

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
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(10000)));

        assert_ok!(Staking::compute_stake(&VAULT, &BOB.account_id), 9950);

        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &ALICE.account_id), 1023);
        assert_ok!(Staking::compute_reward(Token(IBTC), &VAULT, &BOB.account_id), 1976);
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
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(100)));
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
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(100)));
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
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(1152923504604516976 + 100)));

        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(1_000_000)));

        assert_ok!(Staking::deposit_stake(
            &VAULT,
            &VAULT.account_id,
            fixed!(1152924504603286976)
        ));
        assert_ok!(Staking::slash_stake(&VAULT, fixed!(1152924504603286976 + 1_000_000)));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT.account_id), 0);

        assert_ok!(Staking::deposit_stake(&VAULT, &VAULT.account_id, fixed!(1_000_000)));

        assert_ok!(Staking::compute_stake(&VAULT, &VAULT.account_id), 1_000_000);
    })
}
