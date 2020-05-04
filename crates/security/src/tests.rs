use crate::mock::*;
use crate::StatusCode;

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
fn test_get_secure_id() {
    run_test(|| {
        let left = Security::get_secure_id(&1);
        let right = Security::get_secure_id(&1);
        assert_ne!(left, right);
    })
}
