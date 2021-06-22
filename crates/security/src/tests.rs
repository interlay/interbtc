use crate::{mock::*, ErrorCode, StatusCode};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Security($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::Security($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

#[test]
fn test_get_and_set_status() {
    run_test(|| {
        let status_code = Security::get_parachain_status();
        assert_eq!(status_code, StatusCode::Running);
        Security::set_status(StatusCode::Shutdown);
        let status_code = Security::get_parachain_status();
        assert_eq!(status_code, StatusCode::Shutdown);
    })
}

#[test]
fn test_is_ensure_parachain_running_succeeds() {
    run_test(|| {
        Security::set_status(StatusCode::Running);
        assert_ok!(Security::ensure_parachain_status_running());
    })
}

#[test]
fn test_is_ensure_parachain_running_fails() {
    run_test(|| {
        Security::set_status(StatusCode::Error);
        assert_noop!(
            Security::ensure_parachain_status_running(),
            TestError::ParachainNotRunning
        );

        Security::set_status(StatusCode::Shutdown);
        assert_noop!(
            Security::ensure_parachain_status_running(),
            TestError::ParachainNotRunning
        );
    })
}

#[test]
fn test_is_ensure_parachain_not_shutdown_succeeds() {
    run_test(|| {
        Security::set_status(StatusCode::Running);
        assert_ok!(Security::ensure_parachain_status_not_shutdown());

        Security::set_status(StatusCode::Error);
        assert_ok!(Security::ensure_parachain_status_not_shutdown());
    })
}

#[test]
fn test_is_ensure_parachain_not_shutdown_fails() {
    run_test(|| {
        Security::set_status(StatusCode::Shutdown);
        assert_noop!(
            Security::ensure_parachain_status_not_shutdown(),
            TestError::ParachainShutdown
        );
    })
}

#[test]
fn test_is_parachain_error_oracle_offline() {
    run_test(|| {
        Security::set_status(StatusCode::Error);
        Security::insert_error(ErrorCode::OracleOffline);
        assert_eq!(Security::is_parachain_error_oracle_offline(), true);
    })
}

fn test_recover_from_<F>(recover: F, error_codes: Vec<ErrorCode>)
where
    F: FnOnce(),
{
    for err in &error_codes {
        Security::insert_error(err.clone());
    }
    recover();
    for err in &error_codes {
        assert_eq!(Security::get_errors().contains(&err), false);
    }
    assert_eq!(Security::get_parachain_status(), StatusCode::Running);
    assert_emitted!(Event::RecoverFromErrors(StatusCode::Running, error_codes));
}

#[test]
fn test_recover_from_oracle_offline_succeeds() {
    run_test(|| {
        test_recover_from_(Security::recover_from_oracle_offline, vec![ErrorCode::OracleOffline]);
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
fn testget_secure_id() {
    run_test(|| {
        frame_system::Pallet::<Test>::set_parent_hash(H256::zero());
        assert_eq!(
            Security::get_secure_id(&1),
            H256::from_slice(&[
                71, 121, 67, 63, 246, 65, 71, 242, 66, 184, 148, 234, 23, 56, 62, 52, 108, 82, 213, 33, 160, 200, 214,
                1, 13, 46, 37, 138, 95, 245, 117, 109
            ])
        );
    })
}

#[test]
fn testget_secure_ids_not_equal() {
    run_test(|| {
        let left = Security::get_secure_id(&1);
        let right = Security::get_secure_id(&1);
        assert_ne!(left, right);
    })
}

#[test]
fn testget_increment_active_block_succeeds() {
    run_test(|| {
        let initial_active_block = Security::active_block_number();
        Security::set_status(StatusCode::Running);
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block + 1);
    })
}

#[test]
fn testget_active_block_not_incremented_if_not_running() {
    run_test(|| {
        let initial_active_block = Security::active_block_number();

        // not updated if there is an error
        Security::set_status(StatusCode::Error);
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block);

        // not updated if there is shutdown
        Security::set_status(StatusCode::Shutdown);
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block);
    })
}
