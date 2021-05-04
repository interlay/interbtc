use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use sp_arithmetic::{FixedPointNumber, FixedU128};

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::fee($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

#[test]
fn test_calculate_for() {
    run_test(|| {
        let tests: Vec<(u128, FixedU128, u128)> = vec![
            (
                1 * 10u128.pow(8),                               // 1 BTC
                FixedU128::checked_from_rational(1, 2).unwrap(), // 50%
                50000000,
            ),
            (
                50000000,                                          // 0.5 BTC
                FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
                2500000,
            ),
            (
                25000000,                                           // 0.25 BTC
                FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
                125000,
            ),
            (
                12500000,                                             // 0.125 BTC
                FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
                625,
            ),
            (
                1 * 10u128.pow(10),                               // 1 DOT
                FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
                1000000000,
            ),
        ];

        for (amount, percent, expected) in tests {
            let actual = Fee::calculate_for(amount, percent).unwrap();
            assert_eq!(actual, expected);
        }
    })
}

#[test]
fn test_begin_block_only_epoch() {
    run_test(|| {
        <EpochRewardsIssuing<Test>>::set(100);
        <EpochPeriod<Test>>::set(5);
        assert_ok!(Fee::begin_block(15));
        assert_eq!(<EpochRewardsIssuing<Test>>::get(), 0);
    })
}

#[test]
fn test_begin_block_not_epoch() {
    run_test(|| {
        <EpochRewardsIssuing<Test>>::set(100);
        <EpochPeriod<Test>>::set(5);
        assert_ok!(Fee::begin_block(17));
        assert_eq!(<EpochRewardsIssuing<Test>>::get(), 100);
    })
}

#[test]
fn test_rewards_accrue_per_epoch() {
    run_test(|| {
        <RelayerRewards<Test>>::set(FixedU128::checked_from_integer(1).unwrap());
        <EpochPeriod<Test>>::set(50);

        ext::sla::get_relayer_rewards::<Test>
            .mock_safe(|total_issuing, total_backing| MockResult::Return(Ok(vec![(0, total_issuing, total_backing)])));

        <EpochRewardsIssuing<Test>>::set(100);
        assert_ok!(Fee::begin_block(2000));
        assert_eq!(<EpochRewardsIssuing<Test>>::get(), 0);
        assert_eq!(<TotalRewardsIssuing<Test>>::get(0), 100);

        <EpochRewardsIssuing<Test>>::set(200);
        assert_ok!(Fee::begin_block(4000));
        assert_eq!(<EpochRewardsIssuing<Test>>::get(), 0);
        assert_eq!(<TotalRewardsIssuing<Test>>::get(0), 300);
    })
}

#[test]
fn test_relayer_rewards_for_epoch_in_issuing() {
    run_test(|| {
        <RelayerRewards<Test>>::set(FixedU128::checked_from_rational(3, 100).unwrap());
        <VaultRewards<Test>>::set(FixedU128::checked_from_rational(77, 100).unwrap());

        Fee::increase_issuing_rewards_for_epoch(50);
        Fee::increase_issuing_rewards_for_epoch(50);
        assert_eq!(<EpochRewardsIssuing<Test>>::get(), 100);

        let total_relayer_rewards = Fee::relayer_rewards_for_epoch_in_issuing().unwrap();
        assert_eq!(total_relayer_rewards, 3);
    })
}

#[test]
fn test_ensure_rationals_sum_to_one_fails() {
    run_test(|| {
        assert_err!(
            Fee::ensure_rationals_sum_to_one(vec![
                FixedU128::checked_from_rational(45, 100).unwrap(),
                FixedU128::checked_from_rational(3, 100).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
            ]),
            TestError::InvalidRewardDist
        );
    })
}

#[test]
fn test_ensure_rationals_sum_to_one_succeeds() {
    run_test(|| {
        assert_ok!(Fee::ensure_rationals_sum_to_one(vec![
            FixedU128::checked_from_rational(77, 100).unwrap(),
            FixedU128::checked_from_rational(3, 100).unwrap(),
            FixedU128::checked_from_rational(15, 100).unwrap(),
            FixedU128::checked_from_rational(5, 100).unwrap(),
        ],),);
    })
}

#[test]
fn test_withdraw_issuing_fails_with_insufficient_balance() {
    run_test(|| {
        assert_err!(
            Fee::withdraw_issuing(Origin::signed(0), 1000),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn test_withdraw_issuing_succeeds() {
    run_test(|| {
        <TotalRewardsIssuing<Test>>::insert(0, 1000);
        ext::treasury::transfer::<Test>.mock_safe(|fee_pool, signer, amount| {
            assert_eq!(Fee::fee_pool_account_id(), fee_pool);
            assert_eq!(signer, 0);
            assert_eq!(amount, 1000);
            MockResult::Return(Ok(()))
        });

        assert_ok!(Fee::withdraw_issuing(Origin::signed(0), 1000));
        assert_emitted!(Event::WithdrawIssuing(0, 1000));
    })
}

#[test]
fn test_withdraw_backing_fails_with_insufficient_balance() {
    run_test(|| {
        assert_err!(
            Fee::withdraw_backing(Origin::signed(0), 1000),
            TestError::InsufficientFunds
        );
    })
}

#[test]
fn test_withdraw_backing_succeeds() {
    run_test(|| {
        <TotalRewardsBacking<Test>>::insert(0, 1000);
        ext::collateral::transfer::<Test>.mock_safe(|fee_pool, signer, amount| {
            assert_eq!(Fee::fee_pool_account_id(), fee_pool);
            assert_eq!(signer, 0);
            assert_eq!(amount, 1000);
            MockResult::Return(Ok(()))
        });

        assert_ok!(Fee::withdraw_backing(Origin::signed(0), 1000));
        assert_emitted!(Event::WithdrawBacking(0, 1000));
    })
}
