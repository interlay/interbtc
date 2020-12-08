use crate::mock::{run_test, ExchangeRateOracle, Origin, System, Test, TestError, TestEvent};
use crate::GRANULARITY;

use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
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
    run_test(|| {
        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        ExchangeRateOracle::btc_dot_to_satoshi_planck
            .mock_safe(|amount| MockResult::Return(Ok(amount)));
        let result = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
        assert_ok!(result);

        let exchange_rate = ExchangeRateOracle::get_exchange_rate().unwrap();
        assert_eq!(exchange_rate, 100 * 10u128.pow(GRANULARITY as u32));

        assert_emitted!(Event::SetExchangeRate(
            3,
            100 * 10u128.pow(GRANULARITY as u32)
        ));
    });
}

#[test]
fn set_exchange_rate_max_delay_passed() {
    run_test(|| {
        let mut first_call_to_recover = false;
        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        ExchangeRateOracle::is_max_delay_passed.mock_safe(|| MockResult::Return(true));
        // XXX: hacky way to ensure that `recover_from_oracle_offline` was
        // indeed called. mocktopus does not seem to have a `assert_called`
        // kind of feature yet
        ExchangeRateOracle::recover_from_oracle_offline.mock_safe(move || {
            MockResult::Return(if first_call_to_recover {
                Err(DispatchError::BadOrigin)
            } else {
                first_call_to_recover = true;
                Ok(())
            })
        });
        let first_res = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
        assert_ok!(first_res);

        let second_res = ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100);
        assert_err!(second_res, DispatchError::BadOrigin);
    });
}

#[test]
fn set_exchange_rate_wrong_oracle() {
    run_test(|| {
        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        ExchangeRateOracle::btc_dot_to_satoshi_planck
            .mock_safe(|amount| MockResult::Return(Ok(amount)));
        assert_ok!(ExchangeRateOracle::set_exchange_rate(Origin::signed(4), 20));

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(false));
        assert_err!(
            ExchangeRateOracle::set_exchange_rate(Origin::signed(3), 100),
            TestError::InvalidOracleSource
        );

        let exchange_rate = ExchangeRateOracle::get_exchange_rate().unwrap();
        assert_eq!(exchange_rate, 20 * 10u128.pow(GRANULARITY as u32));

        assert_not_emitted!(Event::SetExchangeRate(3, 100));
        assert_not_emitted!(Event::SetExchangeRate(4, 100));
    });
}

#[test]
fn get_exchange_rate_after_delay() {
    run_test(|| {
        ExchangeRateOracle::is_max_delay_passed.mock_safe(|| MockResult::Return(true));
        let result = ExchangeRateOracle::get_exchange_rate();
        assert_err!(result, TestError::MissingExchangeRate);
    });
}

#[test]
fn btc_to_dots() {
    run_test(|| {
        ExchangeRateOracle::get_exchange_rate.mock_safe(|| MockResult::Return(Ok(200000)));
        let test_cases = [(0, 0), (2, 4), (10, 20)];
        for (input, expected) in test_cases.iter() {
            let result = ExchangeRateOracle::btc_to_dots(*input);
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn dots_to_btc() {
    run_test(|| {
        ExchangeRateOracle::get_exchange_rate.mock_safe(|| MockResult::Return(Ok(200000)));
        let test_cases = [(0, 0), (4, 2), (20, 10), (21, 10)];
        for (input, expected) in test_cases.iter() {
            let result = ExchangeRateOracle::dots_to_btc(*input);
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn is_max_delay_passed() {
    run_test(|| {
        let now = 1585776145;

        ExchangeRateOracle::get_current_time.mock_safe(move || MockResult::Return(now));
        ExchangeRateOracle::get_last_exchange_rate_time
            .mock_safe(move || MockResult::Return(now - 3600));

        // max delay is 30 minutes but 1 hour passed
        ExchangeRateOracle::get_max_delay.mock_safe(|| MockResult::Return(1800));
        assert!(ExchangeRateOracle::is_max_delay_passed());

        // max delay is 2 hours and 1 hour passed
        ExchangeRateOracle::get_max_delay.mock_safe(|| MockResult::Return(7200));
        assert!(!ExchangeRateOracle::is_max_delay_passed());
    });
}

#[test]
fn oracle_names_have_genesis_info() {
    run_test(|| {
        let actual = String::from_utf8(ExchangeRateOracle::authorized_oracles(0)).unwrap();
        let expected = "test".to_owned();
        assert_eq!(actual, expected);
    });
}

#[test]
fn convert_btc_dot_to_satoshi_planck() {
    run_test(|| {
        assert_eq!(
            ExchangeRateOracle::btc_dot_to_satoshi_planck(3461).unwrap(),
            346100
        );

        assert_eq!(
            ExchangeRateOracle::btc_dot_to_satoshi_planck(3855).unwrap(),
            385500
        );
    });
}

#[test]
fn insert_authorized_oracle_succeeds() {
    run_test(|| {
        let oracle = 1;
        assert_err!(
            ExchangeRateOracle::set_exchange_rate(Origin::signed(oracle), 100),
            TestError::InvalidOracleSource
        );
        assert_err!(
            ExchangeRateOracle::insert_authorized_oracle(
                Origin::signed(oracle),
                oracle,
                Vec::<u8>::new()
            ),
            DispatchError::BadOrigin
        );
        assert_ok!(ExchangeRateOracle::insert_authorized_oracle(
            Origin::root(),
            oracle,
            Vec::<u8>::new()
        ));
    });
}

#[test]
fn remove_authorized_oracle_succeeds() {
    run_test(|| {
        let oracle = 1;
        ExchangeRateOracle::insert_oracle(oracle, Vec::<u8>::new());
        assert_err!(
            ExchangeRateOracle::remove_authorized_oracle(Origin::signed(oracle), oracle),
            DispatchError::BadOrigin
        );
        assert_ok!(ExchangeRateOracle::remove_authorized_oracle(
            Origin::root(),
            oracle,
        ));
    });
}
