// use crate::mock::{run_test, Origin, System, Test, TestEvent, VaultRegistry};
use crate::mock::run_test;
// use crate::Error;

// use frame_support::{assert_err, assert_ok};
// use mocktopus::mocking::*;

// type Event = crate::Event<Test>;

// // use macro to avoid messing up stack trace
// macro_rules! assert_emitted {
//     ($event:expr) => {
//         let test_event = TestEvent::test_events($event);
//         assert!(System::events().iter().any(|a| a.event == test_event));
//     };
// }

// macro_rules! assert_not_emitted {
//     ($event:expr) => {
//         let test_event = TestEvent::test_events($event);
//         assert!(!System::events().iter().any(|a| a.event == test_event));
//     };
// }

#[test]
fn set_exchange_rate_success() {
    run_test(|| {
        // VaultRegistry::get_authorized_oracle.mock_safe(|| MockResult::Return(3));
        // let result = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
        // assert_ok!(result);

        // let exchange_rate = ExchangeRateOracle::get_exchange_rate().unwrap();
        // assert_eq!(exchange_rate, 100);

        // assert_emitted!(Event::SetExchangeRate(3, 100));
    });
}
