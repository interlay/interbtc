use crate::mock::*;
use frame_support::{
    assert_err, assert_ok,
    dispatch::{DispatchInfo, GetDispatchInfo},
    pallet_prelude::Pays,
    weights::Weight,
};
use helpers::*;
use orml_traits::MultiCurrency;
use sp_runtime::{
    traits::{Dispatchable, SignedExtension},
    transaction_validity::{InvalidTransaction, TransactionValidityError},
};

type Payment = pallet_transaction_payment::ChargeTransactionPayment<Test>;

mod helpers {
    use sp_runtime::transaction_validity::TransactionValidity;

    use super::*;
    /// runs and returns f() without comitting to storage
    pub(super) fn dry_run<T, F: FnOnce() -> T>(f: F) -> T {
        use sp_runtime::{DispatchError, TransactionOutcome};
        frame_support::storage::with_transaction(|| {
            let ret = f();
            TransactionOutcome::Rollback(Result::<T, DispatchError>::Ok(ret))
        })
        .unwrap()
    }

    pub(super) fn setup_dex(currency_1: CurrencyId, currency_2: CurrencyId) {
        let lp = 2323;
        assert_ok!(<Test as dex_general::Config>::MultiCurrency::deposit(
            currency_1,
            &lp,
            1000000000000000000000
        ));
        assert_ok!(<Test as dex_general::Config>::MultiCurrency::deposit(
            currency_2,
            &lp,
            1000000000000000000000
        ));

        assert_ok!(DexGeneral::create_pair(
            RuntimeOrigin::root(),
            currency_1,
            currency_2,
            15,
        ));

        assert_ok!(DexGeneral::add_liquidity(
            RuntimeOrigin::signed(lp),
            currency_1,
            currency_2,
            1000000000000000000000,
            1000000000000000000000,
            0,
            0,
            u64::MAX
        ));
    }
    #[derive(Debug, PartialEq)]
    pub(super) struct BalanceChange {
        pub(super) native_cost: i128,
        pub(super) fee_cost: i128,
    }

    pub(super) fn dry_run_given_call(
        who: u128,
        native_currency: CurrencyId,
        fee_currency: CurrencyId,
        pays_fee: Pays,
        call: RuntimeCall,
        tip: u128,
    ) -> BalanceChange {
        dry_run(|| {
            let initial_fee_balance = 1000000000000;
            assert_ok!(Tokens::set_balance(
                RuntimeOrigin::root(),
                who,
                fee_currency,
                initial_fee_balance,
                0
            ));
            let initial_native_balance = Tokens::free_balance(native_currency, &who);

            let payment_extension = pallet_transaction_payment::ChargeTransactionPayment::<Test>::from(tip);

            let info = call.get_dispatch_info();
            let len = 1231;

            // check that validate executes successfully, but don't commit storage
            dry_run(|| {
                payment_extension.validate(&who, &call, &info, len).unwrap();
                if let Pays::Yes = pays_fee {
                    assert!(Tokens::free_balance(fee_currency, &who) < initial_fee_balance);
                }
            });

            let pre = payment_extension.pre_dispatch(&who, &call, &info, len).unwrap();
            if let Pays::Yes = pays_fee {
                assert!(Tokens::free_balance(fee_currency, &who) < initial_fee_balance);
            }

            let result = call.dispatch(RuntimeOrigin::signed(who));
            let post_info = match result {
                Ok(post_info) => post_info,
                Err(err) => err.post_info,
            };

            Payment::post_dispatch(Some(pre), &info, &post_info, len, &Ok(())).unwrap();

            BalanceChange {
                native_cost: (initial_native_balance as i128) - (Tokens::free_balance(native_currency, &who) as i128),
                fee_cost: (initial_fee_balance as i128) - (Tokens::free_balance(fee_currency, &who) as i128),
            }
        })
    }

    pub(super) fn dry_run_native(expected_weight: u64, actual_weight: Option<u64>, err: bool, pays_fee: Pays) -> u128 {
        let who = 1;
        let native_currency = GetNativeCurrencyId::get();

        let call = RuntimeCall::Testing(testing_helpers::Call::weighted {
            expected: Weight::from_ref_time(expected_weight),
            actual_weight: actual_weight.map(|x| Weight::from_ref_time(x)),
            err,
            pays_fee,
        });
        dry_run_given_call(who, native_currency, native_currency, pays_fee, call, 0).native_cost as u128
    }

    pub(super) fn dry_run_swapped_with_tip(
        expected_weight: u64,
        actual_weight: Option<u64>,
        err: bool,
        pays_fee: Pays,
        tip: u128,
    ) -> BalanceChange {
        dry_run(|| {
            let who = 1;
            let native_currency = GetNativeCurrencyId::get();
            let foreign_currency = GetForeignCurrencyId::get();

            setup_dex(native_currency, foreign_currency);
            let call = RuntimeCall::MultiTransactionPayment(crate::Call::with_fee_swap_path {
                amount_in_max: u128::MAX,
                path: vec![foreign_currency, native_currency],
                call: Box::new(RuntimeCall::Testing(testing_helpers::Call::weighted {
                    expected: Weight::from_ref_time(expected_weight),
                    actual_weight: actual_weight.map(|x| Weight::from_ref_time(x)),
                    err,
                    pays_fee,
                })),
            });
            dry_run_given_call(who, native_currency, foreign_currency, pays_fee, call, tip)
        })
    }

    pub(super) fn validate_swapping_payment(
        expected_weight: u64,
        amount_in_max: u128,
        swap_path: (CurrencyId, CurrencyId),
        initial_fee_balance: u128,
    ) -> TransactionValidity {
        dry_run(|| {
            let who = 1;
            let native_currency = GetNativeCurrencyId::get();
            let foreign_currency = GetForeignCurrencyId::get();

            setup_dex(native_currency, foreign_currency);
            let call = RuntimeCall::MultiTransactionPayment(crate::Call::with_fee_swap_path {
                amount_in_max: amount_in_max,
                path: vec![foreign_currency, native_currency],
                call: Box::new(RuntimeCall::Testing(testing_helpers::Call::weighted {
                    expected: Weight::from_ref_time(expected_weight),
                    actual_weight: None,
                    err: false,
                    pays_fee: Pays::Yes,
                })),
            });
            assert_ok!(Tokens::set_balance(
                RuntimeOrigin::root(),
                who,
                swap_path.0,
                initial_fee_balance,
                0
            ));

            let payment_extension = pallet_transaction_payment::ChargeTransactionPayment::<Test>::from(0);
            let info = call.get_dispatch_info();
            let len = 1231;

            dry_run(|| payment_extension.validate(&who, &call, &info, len))
        })
    }

    pub(super) fn dry_run_swapped(
        expected_weight: u64,
        actual_weight: Option<u64>,
        err: bool,
        pays_fee: Pays,
    ) -> BalanceChange {
        dry_run_swapped_with_tip(expected_weight, actual_weight, err, pays_fee, 0)
    }
}

#[test]
/// basic test that payment in native fee continues to work as normal
fn pay_in_native_fee_succeeds() {
    new_test_ext().execute_with(|| {
        // <Payment as SignedExtension>::pre_dispatch();
        let payment_extension = pallet_transaction_payment::ChargeTransactionPayment::<Test>::from(1);
        let who = 1;
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let weight = Weight::from_ref_time(5);
        let info = DispatchInfo {
            weight: weight,
            ..Default::default()
        };
        let len = 123;

        let native_currency = GetNativeCurrencyId::get();
        let initial = 100000000000000;
        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            who,
            native_currency,
            initial,
            0
        ));

        // check that validate executes successfully, but don't commit storage
        dry_run(|| {
            payment_extension.validate(&who, &call, &info, len).unwrap();
            assert!(Tokens::free_balance(native_currency, &who) < initial);
        });

        let pre = payment_extension.pre_dispatch(&who, &call, &info, len).unwrap();

        let balance = Tokens::free_balance(native_currency, &who);
        assert!(balance < initial);

        let result = call.dispatch(RuntimeOrigin::signed(who)).unwrap();

        Payment::post_dispatch(Some(pre), &info, &result, len, &Ok(())).unwrap();
        // balance shouldn't change by this post_dispatch since weight was as predicted
        assert_eq!(Tokens::free_balance(native_currency, &who), balance);
    });
}

#[test]
/// Test behavior of refunds in native currency
fn pay_in_native_fee_refunds() {
    new_test_ext().execute_with(|| {
        let baseline = dry_run_native(6, None, false, Pays::Yes);

        for err in [true, false] {
            assert_eq!(dry_run_native(6, Some(6), err, Pays::Yes), baseline);

            // Partial refund because less weight was used
            assert!(dry_run_native(6, Some(3), err, Pays::Yes) < baseline);

            // weight of 0: still pays for extrinsic overhead
            assert!(dry_run_native(6, Some(0), err, Pays::Yes) > 0);

            // Pays::No completely refunds everything regardless of weight
            assert_eq!(dry_run_native(6, None, err, Pays::No), 0);
            assert_eq!(dry_run_native(6, Some(6), err, Pays::No), 0);
            assert_eq!(dry_run_native(6, Some(3), err, Pays::No), 0);
            assert_eq!(dry_run_native(6, Some(0), err, Pays::No), 0);
        }
    });
}

#[test]
/// Like `pay_in_native_fee_refunds`, but using a non-native fee currency
fn pay_in_swapped_fee_refunds() {
    new_test_ext().execute_with(|| {
        let baseline = dry_run_swapped(6, None, false, Pays::Yes);
        assert!(baseline.fee_cost > 0); // decrease in fee currency, no change in native
        assert_eq!(baseline.native_cost, 0);

        // `Pays::No` completely refunds everything regardless of weight or err
        let refunded_baseline = dry_run_swapped(6, None, false, Pays::No);
        assert_eq!(refunded_baseline.fee_cost, baseline.fee_cost);
        assert!(refunded_baseline.native_cost < 0); // refunded in native currency
        assert_eq!(dry_run_swapped(6, Some(6), false, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, Some(3), false, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, Some(0), false, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, None, true, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, Some(6), true, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, Some(3), true, Pays::No), refunded_baseline);
        assert_eq!(dry_run_swapped(6, Some(0), true, Pays::No), refunded_baseline);

        for err in [true, false] {
            assert_eq!(dry_run_swapped(6, Some(6), err, Pays::Yes), baseline);

            // Partial refund because less weight was used
            let cost = dry_run_swapped(6, Some(3), err, Pays::Yes);
            assert_eq!(cost.fee_cost, baseline.fee_cost);
            assert!(cost.native_cost < 0); // refunded in native currency
            assert!(cost.native_cost > refunded_baseline.native_cost); // not refunded as much as when its free
        }
    });
}

#[test]
/// test that tips are paid in the fee currency, not the native currency
fn pay_in_swapped_fee_with_tip() {
    new_test_ext().execute_with(|| {
        let baseline = dry_run_swapped_with_tip(6, None, false, Pays::Yes, 0);
        assert!(baseline.fee_cost > 0); // decrease in fee currency, no change in native
        assert_eq!(baseline.native_cost, 0);

        // Paying tip increases the amount of fee taken in the fee currency
        let cost = dry_run_swapped_with_tip(6, None, false, Pays::Yes, 1);
        assert!(cost.fee_cost > baseline.fee_cost);
        assert_eq!(cost.native_cost, 0);

        let pays_no_baseline = dry_run_swapped_with_tip(6, None, false, Pays::No, 0);
        assert!(pays_no_baseline.fee_cost > 0); // decrease in fee currency, no change in native
        assert!(pays_no_baseline.native_cost < 0); // we get refunded

        // `Pays::No` doesn't undo tip, so the same amount should get refunded as the case with 0 fee
        let cost = dry_run_swapped_with_tip(6, None, false, Pays::No, 1);
        assert!(cost.fee_cost > pays_no_baseline.fee_cost);
        assert_eq!(cost.native_cost, pays_no_baseline.native_cost);
    })
}

#[test]
fn pay_fee_in_other_currency_succeeds() {
    new_test_ext().execute_with(|| {
        // <Payment as SignedExtension>::pre_dispatch();
        let native_currency = GetNativeCurrencyId::get();
        let fee_currency = GetForeignCurrencyId::get();
        let swap_path = vec![fee_currency, native_currency];
        setup_dex(fee_currency, native_currency);

        let payment_extension = pallet_transaction_payment::ChargeTransactionPayment::<Test>::from(1);
        let initial = 100000000000000;
        let who = 1;
        let call = RuntimeCall::MultiTransactionPayment(crate::Call::with_fee_swap_path {
            path: swap_path.clone(),
            amount_in_max: u128::MAX,
            call: Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] })),
        });
        let weight = Weight::from_ref_time(5);
        let info = DispatchInfo {
            weight: weight,
            ..Default::default()
        };
        let len = 123;

        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            who,
            fee_currency,
            initial,
            0
        ));

        // check that validate executes successfully, but don't commit storage
        dry_run(|| {
            payment_extension.validate(&who, &call, &info, len).unwrap();
            assert!(Tokens::free_balance(fee_currency, &who) < initial);
        });

        let pre = payment_extension.pre_dispatch(&who, &call, &info, len).unwrap();

        let balance = Tokens::free_balance(fee_currency, &who);
        assert!(balance < initial);

        let result = call.dispatch(RuntimeOrigin::signed(who)).unwrap();

        Payment::post_dispatch(Some(pre), &info, &result, len, &Ok(())).unwrap();
        // balance shouldn't change by this post_dispatch since weight was as predicted
        assert_eq!(Tokens::free_balance(fee_currency, &who), balance);
    });
}

#[test]
fn failing_swaps_dont_get_included_in_block() {
    new_test_ext().execute_with(|| {
        let native_currency = GetNativeCurrencyId::get();
        let fee_currency = GetForeignCurrencyId::get();

        // sanity check: this passes
        assert_ok!(validate_swapping_payment(
            10,
            u128::MAX,
            (fee_currency, native_currency),
            1000000000000
        ));

        let expected_err = TransactionValidityError::Invalid(InvalidTransaction::Payment);

        // insufficient balance
        assert_err!(
            validate_swapping_payment(10, u128::MAX, (fee_currency, native_currency), 1),
            expected_err
        );

        // sufficient balance, but swap path does not end in native currency
        assert_err!(
            validate_swapping_payment(10, u128::MAX, (native_currency, fee_currency), 1000000000000),
            expected_err
        );

        // swap limit too low
        assert_err!(
            validate_swapping_payment(10, 1, (native_currency, fee_currency), 1000000000000),
            expected_err
        );
    });
}
