use crate::{mock::*, Amount};
use sp_runtime::FixedPointNumber;

#[test]
fn test_checked_fixed_point_mul() {
    run_test(|| {
        let currency = Token(DOT);
        let tests: Vec<(Amount<Test>, UnsignedFixedPoint, Amount<Test>)> = vec![
            (
                Amount::new(1 * 10u128.pow(8), currency),                 // 1 BTC
                UnsignedFixedPoint::checked_from_rational(1, 2).unwrap(), // 50%
                Amount::new(50000000, currency),
            ),
            (
                Amount::new(50000000, currency),                            // 0.5 BTC
                UnsignedFixedPoint::checked_from_rational(5, 100).unwrap(), // 5%
                Amount::new(2500000, currency),
            ),
            (
                Amount::new(25000000, currency),                             // 0.25 BTC
                UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
                Amount::new(125000, currency),
            ),
            (
                Amount::new(12500000, currency),                               // 0.125 BTC
                UnsignedFixedPoint::checked_from_rational(5, 100000).unwrap(), // 0.005%
                Amount::new(625, currency),
            ),
            (
                Amount::new(1 * 10u128.pow(10), currency),                 // 1 DOT
                UnsignedFixedPoint::checked_from_rational(1, 10).unwrap(), // 10%
                Amount::new(1000000000, currency),
            ),
        ];

        for (amount, percent, expected) in tests {
            let actual = amount.checked_fixed_point_mul(&percent).unwrap();
            assert_eq!(actual, expected);
        }
    })
}

#[test]
fn test_checked_fixed_point_mul_rounded_up() {
    run_test(|| {
        let currency = Token(DOT);
        let tests: Vec<(Amount<Test>, UnsignedFixedPoint, Amount<Test>)> = vec![
            (
                Amount::new(10, currency),
                UnsignedFixedPoint::checked_from_rational(1, 3).unwrap(),
                Amount::new(4, currency),
            ),
            (
                Amount::new(9, currency),
                UnsignedFixedPoint::checked_from_rational(1, 3).unwrap(),
                Amount::new(3, currency),
            ),
            (
                Amount::new(10, currency),
                UnsignedFixedPoint::checked_from_rational(1, UnsignedFixedPoint::accuracy()).unwrap(),
                Amount::new(1, currency),
            ),
            (
                Amount::new(10, currency),
                UnsignedFixedPoint::from(0),
                Amount::new(0, currency),
            ),
            (
                Amount::new(UnsignedFixedPoint::accuracy(), currency),
                UnsignedFixedPoint::checked_from_rational(1, UnsignedFixedPoint::accuracy()).unwrap(),
                Amount::new(1, currency),
            ),
            (
                Amount::new(UnsignedFixedPoint::accuracy() + 1, currency),
                UnsignedFixedPoint::checked_from_rational(1, UnsignedFixedPoint::accuracy()).unwrap(),
                Amount::new(2, currency),
            ),
        ];

        for (amount, percent, expected) in tests {
            let actual = amount.checked_fixed_point_mul_rounded_up(&percent).unwrap();
            assert_eq!(actual, expected);
        }
    })
}
