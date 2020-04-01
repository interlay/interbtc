use crate::mock::{ExchangeRateOracle, ExtBuilder, Origin, System, Test, TestEvent};
use crate::Error;

use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

#[test]
fn set_exchange_rate_success() {
    ExtBuilder::build().execute_with(|| {
        MockContext::new()
            .mock_safe(ExchangeRateOracle::get_authorized_oracle, || {
                MockResult::Return(3)
            })
            .run(|| {
                let result = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
                assert_ok!(result);

                let exchange_rate = ExchangeRateOracle::get_exchange_rate().unwrap();
                assert_eq!(exchange_rate, 100);

                assert_emitted!(Event::SetExchangeRate(3, 100));
            });
    })
}

#[test]
fn set_exchange_rate_max_delay_passed() {
    ExtBuilder::build().execute_with(|| {
        let mut first_call_to_recover = false;
        MockContext::new()
            .mock_safe(ExchangeRateOracle::get_authorized_oracle, || {
                MockResult::Return(3)
            })
            .mock_safe(ExchangeRateOracle::is_max_delay_passed, || {
                MockResult::Return(Ok(true))
            })
            // XXX: hacky way to ensure that `recover_from_oracle_offline` was
            // indeed called. mocktopus does not seem to have a `assert_called`
            // kind of feature yet
            .mock_safe(ExchangeRateOracle::recover_from_oracle_offline, move || {
                MockResult::Return(if first_call_to_recover {
                    Err(Error::RuntimeError)
                } else {
                    first_call_to_recover = true;
                    Ok(())
                })
            })
            .run(|| {
                let first_res = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
                assert_ok!(first_res);

                let second_res = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
                assert_err!(second_res, Error::RuntimeError);
            })
    })
}

#[test]
fn set_exchange_rate_wrong_oracle() {
    ExtBuilder::build().execute_with(|| {
        MockContext::new()
            .mock_safe(ExchangeRateOracle::get_authorized_oracle, || {
                MockResult::Return(4)
            })
            .run(|| {
                assert_ok!(ExchangeRateOracle::set_exchange_rate(Origin::signed(4), 20));

                let result = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
                assert_err!(result, Error::InvalidOracleSource);

                let exchange_rate = ExchangeRateOracle::get_exchange_rate().unwrap();
                assert_eq!(exchange_rate, 20);

                assert_not_emitted!(Event::SetExchangeRate(3, 100));
                assert_not_emitted!(Event::SetExchangeRate(4, 100));
            })
    })
}

#[test]
fn get_exchange_rate_after_delay() {
    ExtBuilder::build().execute_with(|| {
        MockContext::new()
            .mock_safe(ExchangeRateOracle::is_max_delay_passed, || {
                MockResult::Return(Ok(true))
            })
            .run(|| {
                let result = ExchangeRateOracle::get_exchange_rate();
                assert_err!(result, Error::MissingExchangeRate);
            })
    })
}

#[test]
fn is_max_delay_passed() {
    ExtBuilder::build().execute_with(|| {
        let now = 1585776145;

        let mock_context = || {
            MockContext::new()
                .mock_safe(ExchangeRateOracle::seconds_since_epoch, move || {
                    MockResult::Return(Ok(now))
                })
                .mock_safe(ExchangeRateOracle::get_last_exchange_rate_time, move || {
                    MockResult::Return(now - 3600)
                })
        };

        // max delay is 30 minutes but 1 hour passed
        mock_context()
            .mock_safe(ExchangeRateOracle::get_max_delay, || {
                MockResult::Return(1800)
            })
            .run(|| {
                assert!(ExchangeRateOracle::is_max_delay_passed().unwrap());
            });

        // max delay is 2 hours and 1 hour passed
        mock_context()
            .mock_safe(ExchangeRateOracle::get_max_delay, || {
                MockResult::Return(7200)
            })
            .run(|| {
                assert!(!ExchangeRateOracle::is_max_delay_passed().unwrap());
            });
    })
}

#[test]
fn seconds_since_epoch() {
    ExtBuilder::build().execute_with(|| {
        let now = 1585776145;
        let ten_years = 3600 * 24 * 365;
        let timestamp = ExchangeRateOracle::seconds_since_epoch().unwrap();
        // check that the value of timestamp looks reasonable
        // this test will start failing in 2030 or so
        assert!(now - ten_years < timestamp);
        assert!(timestamp < now + ten_years);
    });
}
