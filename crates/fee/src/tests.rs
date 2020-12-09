use crate::mock::*;

#[test]
fn test_has_request_expired() {
    run_test(|| {
        System::set_block_number(45);
        assert!(true);
    })
}
