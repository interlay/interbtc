use crate::{mock::*, CurrencyId, OracleKey};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use sp_arithmetic::FixedU128;
use sp_runtime::FixedPointNumber;

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Oracle($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Oracle($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

fn mine_block() {
    crate::Pallet::<Test>::begin_block(0);
}

#[test]
fn feed_values_succeeds() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(Token(DOT));
        let rate = FixedU128::checked_from_rational(100, 1).unwrap();

        Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        let result = Oracle::feed_values(Origin::signed(3), vec![(key.clone(), rate)]);
        assert_ok!(result);

        mine_block();

        let exchange_rate = Oracle::get_price(key.clone()).unwrap();
        assert_eq!(exchange_rate, rate);

        assert_emitted!(Event::FeedValues {
            oracle_id: 3,
            values: vec![(key.clone(), rate)]
        });
    });
}

mod oracle_offline_detection {
    use super::*;

    type SecurityPallet = security::Pallet<Test>;
    use security::StatusCode;

    enum SubmittingOracle {
        OracleA,
        OracleB,
    }
    use SubmittingOracle::*;

    fn set_time(time: u64) {
        Oracle::get_current_time.mock_safe(move || MockResult::Return(time));
        mine_block();
    }

    fn feed_value(currency_id: CurrencyId, oracle: SubmittingOracle) {
        assert_ok!(Oracle::feed_values(
            Origin::signed(match oracle {
                OracleA => 1,
                OracleB => 2,
            }),
            vec![(OracleKey::ExchangeRate(currency_id), FixedU128::from(1))]
        ));
        mine_block();
    }

    #[test]
    fn basic_oracle_offline_logic() {
        run_test(|| {
            Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
            Oracle::get_max_delay.mock_safe(move || MockResult::Return(10));

            set_time(0);
            feed_value(Token(DOT), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);

            set_time(5);
            feed_value(Token(KSM), OracleA);

            // DOT expires after block 10
            set_time(10);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
            set_time(11);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);

            // feeding KSM makes no difference
            feed_value(Token(KSM), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);

            // feeding DOT makes it running again
            feed_value(Token(DOT), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);

            // KSM expires after t=21 (it was set at t=11)
            set_time(21);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
            set_time(22);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);

            // check that status remains ERROR until BOTH currencies have been updated
            set_time(100);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);
            feed_value(Token(DOT), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);
            feed_value(Token(KSM), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
        });
    }

    #[test]
    fn oracle_offline_logic_with_multiple_oracles() {
        run_test(|| {
            Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
            Oracle::get_max_delay.mock_safe(move || MockResult::Return(10));

            set_time(0);
            feed_value(Token(DOT), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);

            set_time(5);
            feed_value(Token(KSM), OracleA);

            set_time(7);
            feed_value(Token(DOT), OracleB);

            // OracleA's DOT submission expires at 10, but OracleB's only at 17. However, KSM expires at 15:
            set_time(15);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
            set_time(16);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);

            // Feeding KSM brings it back online
            feed_value(Token(KSM), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);

            // check that status is set of ERROR when both oracle's DOT submission expired
            set_time(17);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
            set_time(18);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Error);

            // A DOT submission by any oracle brings it back online
            feed_value(Token(DOT), OracleA);
            assert_eq!(SecurityPallet::parachain_status(), StatusCode::Running);
        });
    }
}

#[test]
fn feed_values_fails_with_invalid_oracle_source() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(Token(DOT));
        let successful_rate = FixedU128::checked_from_rational(20, 1).unwrap();
        let failed_rate = FixedU128::checked_from_rational(100, 1).unwrap();

        Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        assert_ok!(Oracle::feed_values(
            Origin::signed(4),
            vec![(key.clone(), successful_rate)]
        ));

        mine_block();

        Oracle::is_authorized.mock_safe(|_| MockResult::Return(false));
        assert_err!(
            Oracle::feed_values(Origin::signed(3), vec![(key.clone(), failed_rate)]),
            TestError::InvalidOracleSource
        );

        mine_block();

        let exchange_rate = Oracle::get_price(key.clone()).unwrap();
        assert_eq!(exchange_rate, successful_rate);

        assert_not_emitted!(Event::FeedValues {
            oracle_id: 3,
            values: vec![(key.clone(), failed_rate)]
        });
        assert_not_emitted!(Event::FeedValues {
            oracle_id: 4,
            values: vec![(key.clone(), failed_rate)]
        });
    });
}

#[test]
fn getting_exchange_rate_fails_with_missing_exchange_rate() {
    run_test(|| {
        let key = OracleKey::ExchangeRate(Token(DOT));
        assert_err!(Oracle::get_price(key), TestError::MissingExchangeRate);
        assert_err!(
            Oracle::wrapped_to_collateral(0, Token(DOT)),
            TestError::MissingExchangeRate
        );
        assert_err!(
            Oracle::collateral_to_wrapped(0, Token(DOT)),
            TestError::MissingExchangeRate
        );
    });
}

#[test]
fn wrapped_to_collateral() {
    run_test(|| {
        Oracle::get_price.mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
        let test_cases = [(0, 0), (2, 4), (10, 20)];
        for (input, expected) in test_cases.iter() {
            let result = Oracle::wrapped_to_collateral(*input, Token(DOT));
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn collateral_to_wrapped() {
    run_test(|| {
        Oracle::get_price.mock_safe(|_| MockResult::Return(Ok(FixedU128::checked_from_rational(2, 1).unwrap())));
        let test_cases = [(0, 0), (4, 2), (20, 10), (21, 10)];
        for (input, expected) in test_cases.iter() {
            let result = Oracle::collateral_to_wrapped(*input, Token(DOT));
            assert_ok!(result, *expected);
        }
    });
}

#[test]
fn test_is_invalidated() {
    run_test(|| {
        let now = 1585776145;
        Oracle::get_current_time.mock_safe(move || MockResult::Return(now));
        Oracle::get_max_delay.mock_safe(|| MockResult::Return(3600));
        Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));

        let key = OracleKey::ExchangeRate(Token(DOT));
        let rate = FixedU128::checked_from_rational(100, 1).unwrap();

        Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));
        assert_ok!(Oracle::feed_values(Origin::signed(3), vec![(key.clone(), rate)]));
        mine_block();

        // max delay is 60 minutes, 60+ passed
        assert!(Oracle::is_outdated(&key, now + 3601));

        // max delay is 60 minutes, 30 passed
        Oracle::get_current_time.mock_safe(move || MockResult::Return(now + 1800));
        assert!(!Oracle::is_outdated(&key, now + 3599));
    });
}

#[test]
fn oracle_names_have_genesis_info() {
    run_test(|| {
        let actual = String::from_utf8(Oracle::authorized_oracles(0)).unwrap();
        let expected = "test".to_owned();
        assert_eq!(actual, expected);
    });
}

#[test]
fn insert_authorized_oracle_succeeds() {
    run_test(|| {
        let oracle = 1;
        let key = OracleKey::ExchangeRate(Token(DOT));
        let rate = FixedU128::checked_from_rational(1, 1).unwrap();
        assert_err!(
            Oracle::feed_values(Origin::signed(oracle), vec![]),
            TestError::InvalidOracleSource
        );
        assert_err!(
            Oracle::insert_authorized_oracle(Origin::signed(oracle), oracle, Vec::<u8>::new()),
            DispatchError::BadOrigin
        );
        assert_ok!(Oracle::insert_authorized_oracle(
            Origin::root(),
            oracle,
            Vec::<u8>::new()
        ));
        assert_ok!(Oracle::feed_values(Origin::signed(oracle), vec![(key, rate)]));
    });
}

#[test]
fn remove_authorized_oracle_succeeds() {
    run_test(|| {
        let oracle = 1;
        Oracle::insert_oracle(oracle, Vec::<u8>::new());
        assert_err!(
            Oracle::remove_authorized_oracle(Origin::signed(oracle), oracle),
            DispatchError::BadOrigin
        );
        assert_ok!(Oracle::remove_authorized_oracle(Origin::root(), oracle,));
    });
}

#[test]
fn set_btc_tx_fees_per_byte_succeeds() {
    run_test(|| {
        Oracle::is_authorized.mock_safe(|_| MockResult::Return(true));

        let keys = vec![OracleKey::FeeEstimation];

        let values: Vec<_> = keys
            .iter()
            .enumerate()
            .map(|(idx, key)| (key.clone(), FixedU128::checked_from_rational(idx as u32, 1).unwrap()))
            .collect();

        assert_ok!(Oracle::feed_values(Origin::signed(3), values.clone()));
        mine_block();

        for (key, value) in values {
            assert_eq!(Oracle::get_price(key).unwrap(), value);
        }
    });
}

#[test]
fn begin_block_set_oracle_offline_succeeds() {
    run_test(|| unsafe {
        let mut oracle_reported = false;
        Oracle::report_oracle_offline.mock_raw(|| {
            oracle_reported = true;
            MockResult::Return(())
        });

        Oracle::begin_block(0);
        assert!(oracle_reported, "Oracle should be reported as offline");
    });
}
