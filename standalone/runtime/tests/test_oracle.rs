mod mock;
use mock::*;

#[test]
fn integration_test_oracle_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::ExchangeRateOracle(ExchangeRateOracleCall::feed_values(vec![]))
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
