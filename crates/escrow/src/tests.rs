/// Tests for Escrow
use crate::mock::*;
use frame_support::assert_ok;

// type Event = crate::Event<Test>;

#[test]
fn should_lock_and_degrade_power() {
    run_test(|| {
        let amount = 1000;
        let max_time = 100;
        let slope = amount / max_time;
        let end_time = 100;
        let start_time = System::block_number();
        let bias = slope * (end_time - start_time);

        assert_ok!(Escrow::create_lock(&ALICE, amount, end_time));

        for current_time in [0, 50, 100] {
            let balance = bias - (slope * (current_time - start_time));
            assert_eq!(Escrow::balance_at(&ALICE, Some(current_time)), balance);
        }
    })
}
