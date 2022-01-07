mod mock;
use mock::{assert_eq, *};

#[test]
fn integration_test_oracle_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Oracle(OracleCall::feed_values { values: vec![] }).dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered
        );
    })
}
