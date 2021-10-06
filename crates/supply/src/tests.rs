/// Tests for Supply
use crate::mock::*;

#[test]
fn should_inflate_supply_from_start_height() {
    run_test(|| {
        Supply::begin_block(0);
        let mut start_height = 100;
        assert_eq!(Supply::start_height(), Some(start_height));
        assert_eq!(Supply::last_emission(), 0);

        for emission in [200_000, 204_000] {
            Supply::begin_block(start_height);
            start_height += YEARS;
            assert_eq!(Supply::start_height(), Some(start_height));
            assert_eq!(Supply::last_emission(), emission);
        }
    })
}
