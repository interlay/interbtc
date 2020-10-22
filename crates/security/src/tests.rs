use crate::mock::*;
use crate::ErrorCode;
use crate::StatusCode;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchResult};
use sp_core::H256;

type Event = crate::Event;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::test_events($event);
        assert_eq!(
            System::events()
                .iter()
                .filter(|a| a.event == test_event)
                .count(),
            $times
        );
    };
}

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
            TestError::ParachainNotRunning
        );

        Security::set_parachain_status(StatusCode::Shutdown);
        assert_noop!(
            Security::_ensure_parachain_status_running(),
            TestError::ParachainNotRunning
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
            TestError::ParachainShutdown
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

fn test_recover_from_<F>(recover: F, error_codes: Vec<ErrorCode>)
where
    F: FnOnce() -> DispatchResult,
{
    for err in &error_codes {
        Security::insert_error(err.clone());
    }
    assert_ok!(recover());
    for err in &error_codes {
        assert_eq!(Security::get_errors().contains(&err), false);
    }
    assert_eq!(Security::get_parachain_status(), StatusCode::Running);
    assert_emitted!(Event::RecoverFromErrors(StatusCode::Running, error_codes));
}

#[test]
fn test_recover_from_liquidation_succeeds() {
    run_test(|| {
        test_recover_from_(
            Security::recover_from_liquidation,
            vec![ErrorCode::Liquidation],
        );
    })
}

#[test]
fn test_recover_from_oracle_offline_succeeds() {
    run_test(|| {
        test_recover_from_(
            Security::recover_from_oracle_offline,
            vec![ErrorCode::OracleOffline],
        );
    })
}

#[test]
fn test_recover_from_btc_relay_failure_succeeds() {
    run_test(|| {
        test_recover_from_(
            Security::recover_from_btc_relay_failure,
            vec![ErrorCode::InvalidBTCRelay, ErrorCode::NoDataBTCRelay],
        );
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
        frame_system::Module::<Test>::set_parent_hash(H256::zero());
        assert_eq!(
            Security::_get_secure_id(&1),
            H256::from_slice(&[
                71, 121, 67, 63, 246, 65, 71, 242, 66, 184, 148, 234, 23, 56, 62, 52, 108, 82, 213,
                33, 160, 200, 214, 1, 13, 46, 37, 138, 95, 245, 117, 109
            ])
        );
    })
}

#[test]
fn test_get_secure_ids_not_equal() {
    run_test(|| {
        let left = Security::_get_secure_id(&1);
        let right = Security::_get_secure_id(&1);
        assert_ne!(left, right);
    })
}
