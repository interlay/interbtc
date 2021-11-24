use sp_runtime::traits::Identity;

use super::*;
/// Tests for Claim
use crate::mock::*;

#[test]
fn should_compute_vesting_schedule() {
    run_test(|| {
        let start_height: BlockNumber = 10;
        let end_height: BlockNumber = 110;
        let period: BlockNumber = 10;
        let balance: Balance = 1000;

        let vesting_schedule =
            compute_vesting_schedule::<_, _, Identity>(start_height, end_height, period, balance).unwrap();

        assert_eq!(vesting_schedule.end(), Some(110));
        assert_eq!(vesting_schedule.locked_amount(start_height), balance as u64);
        assert_eq!(vesting_schedule.locked_amount(end_height), 0);
    })
}
