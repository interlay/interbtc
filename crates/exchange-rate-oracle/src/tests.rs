use crate::{
    mock::{run_test, ExchangeRateOracle, Origin, System, Test, TestError, TestEvent},
    BitcoinInclusionTime, CurrencyId, OracleKey,
};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use sp_arithmetic::FixedU128;
use sp_runtime::FixedPointNumber;

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::ExchangeRateOracle($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::ExchangeRateOracle($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

fn mine_block() {
    crate::Pallet::<Test>::begin_block(0);
}

#[test]
fn feed_values_succeeds() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(CurrencyId::DOT);
        let rate = FixedU128::checked_from_rational(100, 1).unwrap();

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        let result = ExchangeRateOracle::feed_values(Origin::signed(3), vec![(key.clone(), rate)]);
        assert_ok!(result);

        mine_block();

        let exchange_rate = ExchangeRateOracle::get_exchange_rate(key.clone()).unwrap();
        assert_eq!(exchange_rate, rate);

        assert_emitted!(Event::SetExchangeRate(3, vec![(key.clone(), rate)]));
    });
}

#[test]
fn feed_values_recovers_from_oracle_offline() {
    run_test(|| {
        let rate = FixedU128::checked_from_rational(1, 1).unwrap();
        let key = OracleKey::ExchangeRate(CurrencyId::DOT);

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));

        unsafe {
            let mut oracle_recovered = false;
            ExchangeRateOracle::recover_from_oracle_offline.mock_raw(|| {
                oracle_recovered = true;
                MockResult::Return(())
            });

            assert_ok!(ExchangeRateOracle::feed_values(Origin::signed(3), vec![(key, rate)]));
            mine_block();
            assert!(oracle_recovered, "Oracle should be recovered from offline");
        }
    });
}

#[test]
fn feed_values_fails_with_invalid_oracle_source() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(CurrencyId::DOT);
        let successful_rate = FixedU128::checked_from_rational(20, 1).unwrap();
        let failed_rate = FixedU128::checked_from_rational(100, 1).unwrap();

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        assert_ok!(ExchangeRateOracle::feed_values(
            Origin::signed(4),
            vec![(key.clone(), successful_rate)]
        ));

        mine_block();

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(false));
        assert_err!(
            ExchangeRateOracle::feed_values(Origin::signed(3), vec![(key.clone(), failed_rate)]),
            TestError::InvalidOracleSource
        );

        mine_block();

        let exchange_rate = ExchangeRateOracle::get_exchange_rate(key.clone()).unwrap();
        assert_eq!(exchange_rate, successful_rate);

        assert_not_emitted!(Event::SetExchangeRate(3, vec![(key.clone(), failed_rate)]));
        assert_not_emitted!(Event::SetExchangeRate(4, vec![(key.clone(), failed_rate)]));
    });
}

#[test]
fn getting_exchange_rate_fails_with_missing_exchange_rate() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(CurrencyId::DOT);
        assert_err!(
            ExchangeRateOracle::get_exchange_rate(key),
            TestError::MissingExchangeRate
        );
        assert_err!(
            ExchangeRateOracle::wrapped_to_collateral(0),
            TestError::MissingExchangeRate
        );
        assert_err!(
            ExchangeRateOracle::collateral_to_wrapped(0),
            TestError::MissingExchangeRate
        );
    });
}

#[test]
fn wrapped_to_collateral() {
    run_test(|| {
        ExchangeRateOracle::get_exchange_rate
            .mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
        let test_cases = [(0, 0), (2, 4), (10, 20)];
        for (input, expected) in test_cases.iter() {
            let result = ExchangeRateOracle::wrapped_to_collateral(*input);
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn collateral_to_wrapped() {
    run_test(|| {
        ExchangeRateOracle::get_exchange_rate
            .mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
        let test_cases = [(0, 0), (4, 2), (20, 10), (21, 10)];
        for (input, expected) in test_cases.iter() {
            let result = ExchangeRateOracle::collateral_to_wrapped(*input);
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn test_is_invalidated() {
    run_test(|| {
        let now = 1585776145;
        ExchangeRateOracle::get_current_time.mock_safe(move || MockResult::Return(now));
        ExchangeRateOracle::get_max_delay.mock_safe(|| MockResult::Return(3600));
        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));

        let key = OracleKey::ExchangeRate(CurrencyId::DOT);
        let rate = FixedU128::checked_from_rational(100, 1).unwrap();

        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        assert_ok!(ExchangeRateOracle::feed_values(
            Origin::signed(3),
            vec![(key.clone(), rate)]
        ));
        mine_block();

        // max delay is 60 minutes, 60+ passed
        assert!(ExchangeRateOracle::is_outdated(&key, now + 3601));

        // max delay is 60 minutes, 30 passed
        ExchangeRateOracle::get_current_time.mock_safe(move || MockResult::Return(now + 1800));
        assert!(!ExchangeRateOracle::is_outdated(&key, now + 3599));
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
fn insert_authorized_oracle_succeeds() {
    run_test(|| {
        let oracle = 1;
        let key = OracleKey::ExchangeRate(CurrencyId::DOT);
        let rate = FixedU128::checked_from_rational(1, 1).unwrap();
        assert_err!(
            ExchangeRateOracle::feed_values(Origin::signed(oracle), vec![]),
            TestError::InvalidOracleSource
        );
        assert_err!(
            ExchangeRateOracle::insert_authorized_oracle(Origin::signed(oracle), oracle, Vec::<u8>::new()),
            DispatchError::BadOrigin
        );
        assert_ok!(ExchangeRateOracle::insert_authorized_oracle(
            Origin::root(),
            oracle,
            Vec::<u8>::new()
        ));
        assert_ok!(ExchangeRateOracle::feed_values(
            Origin::signed(oracle),
            vec![(key, rate)]
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
        assert_ok!(ExchangeRateOracle::remove_authorized_oracle(Origin::root(), oracle,));
    });
}

#[test]
fn set_btc_tx_fees_per_byte_succeeds() {
    run_test(|| {
        ExchangeRateOracle::is_authorized.mock_safe(|_| MockResult::Return(true));

        let keys = vec![
            OracleKey::FeeEstimation(BitcoinInclusionTime::Fast),
            OracleKey::FeeEstimation(BitcoinInclusionTime::Half),
            OracleKey::FeeEstimation(BitcoinInclusionTime::Hour),
        ];

        let values: Vec<_> = keys
            .iter()
            .enumerate()
            .map(|(idx, key)| (key.clone(), FixedU128::checked_from_rational(idx as u32, 1).unwrap()))
            .collect();

        assert_ok!(ExchangeRateOracle::feed_values(Origin::signed(3), values.clone()));
        mine_block();

        for (key, value) in values {
            assert_eq!(ExchangeRateOracle::get_exchange_rate(key).unwrap(), value);
        }
    });
}

#[test]
fn begin_block_set_oracle_offline_succeeds() {
    run_test(|| unsafe {
        let mut oracle_reported = false;
        ExchangeRateOracle::report_oracle_offline.mock_raw(|_| {
            oracle_reported = true;
            MockResult::Return(())
        });

        ExchangeRateOracle::begin_block(0);
        assert!(oracle_reported, "Oracle should be reported as offline");
    });
}
