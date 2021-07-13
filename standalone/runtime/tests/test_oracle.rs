mod mock;
use mock::*;
use primitives::CurrencyId;

#[test]
fn integration_test_oracle_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::ExchangeRateOracle(ExchangeRateOracleCall::set_exchange_rate(FixedU128::zero()))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );

        assert_noop!(
            Call::ExchangeRateOracle(ExchangeRateOracleCall::set_btc_tx_fees_per_byte(0, 0, 0))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}

#[test]
fn integration_test_oracle() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        let key = CurrencyId::DOT;
        let value = 23u32;

        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, value)])).dispatch(origin_of(account_of(BOB))));

        assert_eq!(OraclePallet::get(&key).unwrap().value, value);
    })
}

#[test]
fn integration_test_oracle_medianizing() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        let key = CurrencyId::DOT;

        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, 10)])).dispatch(origin_of(account_of(ALICE))));
        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, 5)])).dispatch(origin_of(account_of(BOB))));
        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, 15)])).dispatch(origin_of(account_of(CAROL))));

        assert_eq!(OraclePallet::get(&key).unwrap().value, 10);
    })
}

#[test]
fn integration_test_oracle_medianizing_with_even_number() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        let key = CurrencyId::DOT;

        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, 5)])).dispatch(origin_of(account_of(ALICE))));
        assert_ok!(Call::Oracle(OracleCall::feed_values(vec![(key, 15)])).dispatch(origin_of(account_of(BOB))));

        assert_eq!(OraclePallet::get(&key).unwrap().value, 10);
    })
}
