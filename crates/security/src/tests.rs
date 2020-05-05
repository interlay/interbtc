use crate::mock::*;
use crate::ErrorCode;
use crate::StatusCode;
use frame_support::assert_ok;

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
