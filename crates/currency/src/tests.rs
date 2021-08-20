
#[test]
fn test_calculate_for() {
    run_test(|| {
        let currency = DOT;
        let tests: Vec<(Amount, FixedU128, u128)> = vec![
            (
                Amount::new(1 * 10u128.pow(8), currency), // 1 BTC
                FixedU128::checked_from_rational(1, 2).unwrap(), // 50%
                Amount::new(50000000,
            ),
            (
                Amount::new(50000000, currency), // 0.5 BTC
                FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
                Amount::new(2500000, currency),
            ),
            (
                Amount::new(25000000, currency), // 0.25 BTC
                FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
                Amount::new(125000, currency),
            ),
            (
                Amount::new(12500000, currency), // 0.125 BTC
                FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
                Amount::new(625, currency),
            ),
            (
                Amount::new(1 * 10u128.pow(10), currency),// 1 DOT
                FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
                Amount::new(1000000000, currency),
            ),
        ];

        for (amount, percent, expected) in tests {
            let actual = Fee::calculate_for(amount, percent).unwrap();
            assert_eq!(actual, expected);
        }
    })
}
