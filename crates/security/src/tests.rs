use crate::mock::*;
use crate::ErrorCode;
use crate::StatusCode;
use frame_support::{assert_noop, assert_ok};
use x_core::Error;

#[test]
fn test_get_and_set_parachain_status() {
    run_test(|| {
        let status_code = Security::get_parachain_status();
        assert_eq!(status_code, StatusCode::Running);
        Security::set_parachain_status(StatusCode::Shutdown);
        let status_code = Security::get_parachain_status();
        assert_eq!(status_code, StatusCode::Shutdown);
    })
}

#[test]
fn test_is_ensure_parachain_running_succeeds() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Running);
        assert_ok!(Security::_ensure_parachain_status_running());
    })
}

#[test]
fn test_is_ensure_parachain_running_fails() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Error);
        assert_noop!(
            Security::_ensure_parachain_status_running(),
            Error::ParachainNotRunning
        );

        Security::set_parachain_status(StatusCode::Shutdown);
        assert_noop!(
            Security::_ensure_parachain_status_running(),
            Error::ParachainNotRunning
        );
    })
}

#[test]
fn test_is_ensure_parachain_not_shutdown_succeeds() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Running);
        assert_ok!(Security::_ensure_parachain_status_not_shutdown());

        Security::set_parachain_status(StatusCode::Error);
        assert_ok!(Security::_ensure_parachain_status_not_shutdown());
    })
}

#[test]
fn test_is_ensure_parachain_not_shutdown_fails() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Shutdown);
        assert_noop!(
            Security::_ensure_parachain_status_not_shutdown(),
            Error::ParachainShutdown
        );
    })
}

#[test]
fn test_is_parachain_error_no_data_btcrelay() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Error);
        assert_ok!(Security::mutate_errors(|errors| {
            errors.insert(ErrorCode::NoDataBTCRelay);
            Ok(())
        }));
        assert_eq!(Security::_is_parachain_error_no_data_btcrelay(), true);
    })
}

#[test]
fn test_is_parachain_error_invalid_btcrelay() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Error);
        assert_ok!(Security::mutate_errors(|errors| {
            errors.insert(ErrorCode::InvalidBTCRelay);
            Ok(())
        }));
        assert_eq!(Security::_is_parachain_error_invalid_btcrelay(), true);
    })
}

#[test]
fn test_is_parachain_error_oracle_offline() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Error);
        assert_ok!(Security::mutate_errors(|errors| {
            errors.insert(ErrorCode::OracleOffline);
            Ok(())
        }));
        assert_eq!(Security::_is_parachain_error_oracle_offline(), true);
    })
}

#[test]
fn test_is_parachain_error_liquidation() {
    run_test(|| {
        Security::set_parachain_status(StatusCode::Error);
        assert_ok!(Security::mutate_errors(|errors| {
            errors.insert(ErrorCode::Liquidation);
            Ok(())
        }));
        assert_eq!(Security::_is_parachain_error_liquidation(), true);
    })
}

#[test]
fn test_get_nonce() {
    run_test(|| {
        let left = Security::get_nonce();
        let right = Security::get_nonce();
        assert_eq!(right, left + 1);
    })
}

#[test]
fn test_get_secure_id() {
    run_test(|| {
        let left = Security::_get_secure_id(&1);
        let right = Security::_get_secure_id(&1);
        assert_ne!(left, right);
    })
}
