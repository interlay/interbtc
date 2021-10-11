use crate::{Error, GetCompact};
use sp_core::U256;

/// Target Timespan: 2 weeks (1209600 seconds)
// https://github.com/bitcoin/bitcoin/blob/5ba5becbb5d8c794efe579caeea7eea64f895a13/src/chainparams.cpp#L77
pub const TARGET_TIMESPAN: u64 = 14 * 24 * 60 * 60;

/// Used in Bitcoin's retarget algorithm
pub const TARGET_TIMESPAN_DIVISOR: u64 = 4;

/// Unrounded Maximum Target
/// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
pub const UNROUNDED_MAX_TARGET: U256 = U256([
    <u64>::max_value(),
    <u64>::max_value(),
    <u64>::max_value(),
    0x0000_0000_ffff_ffffu64,
]);

// https://github.com/bitcoin/bitcoin/blob/89b910711c004c21b7d67baa888073742f7f94f0/src/pow.cpp#L49-L72
pub fn calculate_next_work_required(
    previous_target: U256,
    first_block_time: u64,
    last_block_time: u64,
) -> Result<u32, Error> {
    let mut actual_timespan = last_block_time.saturating_sub(first_block_time);

    if actual_timespan < TARGET_TIMESPAN / TARGET_TIMESPAN_DIVISOR {
        actual_timespan = TARGET_TIMESPAN / TARGET_TIMESPAN_DIVISOR;
    }

    if actual_timespan > TARGET_TIMESPAN * TARGET_TIMESPAN_DIVISOR {
        actual_timespan = TARGET_TIMESPAN * TARGET_TIMESPAN_DIVISOR;
    }

    let target = previous_target * actual_timespan;
    let target = target / TARGET_TIMESPAN;

    // ensure target does not exceed max
    if target > UNROUNDED_MAX_TARGET {
        UNROUNDED_MAX_TARGET
    } else {
        target
    }
    .get_compact()
    .ok_or(Error::InvalidCompact)
}

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/test/pow_tests.cpp
#[cfg(test)]
mod tests {
    use super::*;
    use crate::SetCompact;
    use frame_support::assert_ok;

    fn target_set_compact(bits: u32) -> U256 {
        U256::set_compact(bits).unwrap()
    }

    #[test]
    fn get_next_work() {
        let previous_target = target_set_compact(0x1d00ffff);
        let first_block_time = 1261130161; // Block #30240
        let last_block_time = 1262152739; // Block #32255
        assert_ok!(
            calculate_next_work_required(previous_target, first_block_time, last_block_time),
            0x1d00d86a
        );
    }

    #[test]
    fn get_next_work_pow_limit() {
        let previous_target = target_set_compact(0x1d00ffff);
        let first_block_time = 1231006505; // Block #0
        let last_block_time = 1233061996; // Block #2015
        assert_ok!(
            calculate_next_work_required(previous_target, first_block_time, last_block_time),
            0x1d00ffff
        );
    }

    #[test]
    fn get_next_work_lower_limit_actual() {
        let previous_target = target_set_compact(0x1c05a3f4);
        let first_block_time = 1279008237; // Block #66528
        let last_block_time = 1279297671; // Block #68543
        assert_ok!(
            calculate_next_work_required(previous_target, first_block_time, last_block_time),
            0x1c0168fd
        );
    }

    #[test]
    fn get_next_work_upper_limit_actual() {
        let previous_target = target_set_compact(0x1c387f6f);
        let first_block_time = 1263163443; // NOTE: Not an actual block time
        let last_block_time = 1269211443; // Block #46367
        assert_ok!(
            calculate_next_work_required(previous_target, first_block_time, last_block_time),
            0x1d00e1fd
        );
    }

    #[test]
    fn get_next_work_recent() {
        // this is the only test different from the bitcoin pow_tests
        let previous_target = target_set_compact(0x170ed0eb);
        let first_block_time = 1632234876; // Block #701568
        let last_block_time = 1633390031; // Block #703583
        assert_ok!(
            calculate_next_work_required(previous_target, first_block_time, last_block_time),
            0x170e2632 // Block #703584
        );
    }
}
