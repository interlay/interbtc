use crate::mock::*;
use crate::*;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use sp_arithmetic::{FixedPointNumber, FixedU128};

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
        <EpochRewards<Test>>::set(100);
        <EpochPeriod<Test>>::set(5);
        assert_ok!(Fee::begin_block(15));
        assert_eq!(<EpochRewards<Test>>::get(), 0);
    })
}

#[test]
fn test_begin_block_not_epoch() {
    run_test(|| {
        <EpochRewards<Test>>::set(100);
        <EpochPeriod<Test>>::set(5);
        assert_ok!(Fee::begin_block(17));
        assert_eq!(<EpochRewards<Test>>::get(), 100);
    })
}

#[test]
fn test_rewards_accrue_per_epoch() {
    run_test(|| {
        <RelayerRewards<Test>>::set(FixedU128::checked_from_integer(1).unwrap());
        <EpochPeriod<Test>>::set(50);

        ext::sla::get_relayer_rewards::<Test>
            .mock_safe(|total| MockResult::Return(vec![(0, Ok(total))]));

        <EpochRewards<Test>>::set(100);
        assert_ok!(Fee::begin_block(2000));
        assert_eq!(<EpochRewards<Test>>::get(), 0);
        assert_eq!(<TotalRewards<Test>>::get(0), 100);

        <EpochRewards<Test>>::set(200);
        assert_ok!(Fee::begin_block(4000));
        assert_eq!(<EpochRewards<Test>>::get(), 0);
        assert_eq!(<TotalRewards<Test>>::get(0), 300);
    })
}

#[test]
fn test_relayer_rewards_for_epoch() {
    run_test(|| {
        <RelayerRewards<Test>>::set(FixedU128::checked_from_rational(3, 100).unwrap());
        <VaultRewards<Test>>::set(FixedU128::checked_from_rational(77, 100).unwrap());

        Fee::increase_rewards_for_epoch(50);
        Fee::increase_rewards_for_epoch(50);
        assert_eq!(<EpochRewards<Test>>::get(), 100);

        let total_relayer_rewards = Fee::relayer_rewards_for_epoch().unwrap();
        assert_eq!(total_relayer_rewards, 3);
    })
}

#[test]
fn test_ensure_rewards_are_valid_fails() {
    run_test(|| {
        assert_err!(
            Fee::ensure_rewards_are_valid(
                FixedU128::checked_from_rational(45, 100).unwrap(),
                FixedU128::checked_from_rational(3, 100).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
            ),
            TestError::InvalidRewardDist
        );
    })
}

#[test]
fn test_ensure_rewards_are_valid_succeeds() {
    run_test(|| {
        assert_ok!(Fee::ensure_rewards_are_valid(
            FixedU128::checked_from_rational(77, 100).unwrap(),
            FixedU128::checked_from_rational(3, 100).unwrap(),
            FixedU128::checked_from_rational(15, 100).unwrap(),
            FixedU128::checked_from_rational(5, 100).unwrap(),
        ),);
    })
}
