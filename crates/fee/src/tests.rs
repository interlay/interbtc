use crate::mock::*;
// use crate::*;
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
