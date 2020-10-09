use crate::ext;
use crate::mock::*;

use crate::types::{PolkaBTC, RedeemRequest, DOT};
use bitcoin::types::H256Le;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use sp_std::convert::TryInto;
use vault_registry::{Vault, VaultStatus};

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::test_events($event);
        assert_eq!(
            System::events()
                .iter()
                .filter(|a| a.event == test_event)
                .count(),
            $times
        );
    };
}

fn btc_to_u128(amount: PolkaBTC<Test>) -> Result<u128, DispatchError> {
    TryInto::<u128>::try_into(amount).map_err(|_e| TestError::ConversionError.into())
}

fn u128_to_dot(x: u128) -> Result<DOT<Test>, DispatchError> {
    TryInto::<DOT<Test>>::try_into(x).map_err(|_| TestError::ConversionError.into())
}

fn btcdot_parity(btc: PolkaBTC<Test>) -> Result<DOT<Test>, DispatchError> {
    let dot = btc_to_u128(btc)?;
    u128_to_dot(dot)
}

fn inject_redeem_request(
    key: H256,
    value: RedeemRequest<AccountId, BlockNumber, Balance, Balance>,
) {
    Redeem::insert_redeem_request(key, value)
}

#[test]
fn test_ensure_parachain_running_or_error_liquidated_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_err!(
            Redeem::ensure_parachain_running_or_error_liquidated(),
            SecurityError::ParachainNotRunning
        );

        ext::security::ensure_parachain_status_has_only_specific_errors::<Test>
            .mock_safe(|_| MockResult::Return(Err(SecurityError::InvalidBTCRelay.into())));

        assert_err!(
            Redeem::ensure_parachain_running_or_error_liquidated(),
            SecurityError::InvalidBTCRelay
        );
    })
}

#[test]
fn test_ensure_parachain_running_or_error_liquidated_succeeds() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Ok(())));

        assert_ok!(Redeem::ensure_parachain_running_or_error_liquidated());

        ext::security::ensure_parachain_status_has_only_specific_errors::<Test>
            .mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(Redeem::ensure_parachain_running_or_error_liquidated());
    })
}

#[test]
fn test_request_redeem_fails_with_amount_exceeds_user_balance() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });
        <treasury::Module<Test>>::mint(ALICE, 2);
        let amount = 10_000_000;
        assert_err!(
            Redeem::request_redeem(
                Origin::signed(ALICE),
                amount,
                H160::from_slice(&[0; 20]),
                BOB
            ),
            TestError::AmountExceedsUserBalance
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_not_found() {
    run_test(|| {
        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 0, H160::from_slice(&[0; 20]), BOB),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_banned() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: Some(1),
                status: VaultStatus::Active,
            }))
        });
        ext::vault_registry::ensure_not_banned::<Test>
            .mock_safe(|_, _| MockResult::Return(Err(VaultRegistryError::VaultBanned.into())));

        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 0, H160::from_slice(&[0; 20]), BOB),
            VaultRegistryError::VaultBanned
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_liquidated() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: Some(1),
                status: VaultStatus::Liquidated,
            }))
        });

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 0, H160::from_slice(&[0; 20]), BOB),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_fails_with_amount_exceeds_vault_balance() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });
        <treasury::Module<Test>>::mint(ALICE, 2);

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        let amount = 11;
        assert_err!(
            Redeem::request_redeem(
                Origin::signed(ALICE),
                amount,
                H160::from_slice(&[0; 20]),
                BOB
            ),
            TestError::AmountExceedsVaultBalance
        );
    })
}

#[test]
fn test_request_redeem_succeeds_in_running_state() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
                status: VaultStatus::Active,
            },
        );

        let redeemer = ALICE;
        let amount = 9;

        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>.mock_safe(
            move |vault_id, amount_btc| {
                assert_eq!(vault_id, &BOB);
                assert_eq!(amount_btc, amount);

                MockResult::Return(Ok(()))
            },
        );

        ext::treasury::lock::<Test>.mock_safe(move |account, amount_polka_btc| {
            assert_eq!(account, redeemer);
            assert_eq!(amount_polka_btc, amount);

            MockResult::Return(Ok(()))
        });

        ext::security::get_secure_id::<Test>.mock_safe(move |_| MockResult::Return(H256([0; 32])));

        assert_ok!(Redeem::request_redeem(
            Origin::signed(redeemer.clone()),
            amount,
            H160([0; 20]),
            BOB
        ));

        assert_emitted!(Event::RequestRedeem(
            H256([0; 32]),
            redeemer.clone(),
            amount,
            BOB,
            H160([0; 20]),
        ));
        assert_ok!(
            Redeem::get_redeem_request_from_id(&H256([0; 32])),
            RedeemRequest {
                vault: BOB,
                opentime: 1,
                amount_polka_btc: amount,
                amount_btc: amount,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: redeemer.clone(),
                btc_address: H160([0; 20]),
            }
        );
    })
}

#[test]
fn test_partial_redeem_factor() {
    run_test(|| {
        ext::vault_registry::total_liquidation_value::<Test>
            .mock_safe(|| MockResult::Return(Ok(1000)));
        ext::treasury::get_total_supply::<Test>.mock_safe(|| MockResult::Return(10));

        assert_ok!(Redeem::get_partial_redeem_factor(), 100);
    })
}

#[test]
fn test_request_redeem_succeeds_in_error_state() {
    run_test(|| {
        ext::security::ensure_parachain_status_has_only_specific_errors::<Test>
            .mock_safe(|_| MockResult::Return(Ok(())));

        ext::security::is_parachain_error_liquidation::<Test>
            .mock_safe(|| MockResult::Return(true));

        Redeem::get_partial_redeem_factor.mock_safe(|| MockResult::Return(Ok(50_000)));

        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));

        let redeemer = ALICE;
        let amount = 10 * 100_000_000;

        <treasury::Module<Test>>::mint(ALICE, amount);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: amount,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
                status: VaultStatus::Active,
            },
        );
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>.mock_safe(
            move |vault_id, amount_btc| {
                assert_eq!(vault_id, &BOB);
                assert_eq!(amount_btc, amount / 2);

                MockResult::Return(Ok(()))
            },
        );

        ext::vault_registry::redeem_tokens_liquidation::<Test>.mock_safe(
            move |vault_id, amount_polka_btc| {
                assert_eq!(vault_id, &BOB);
                assert_eq!(amount_polka_btc, amount / 2);

                MockResult::Return(Ok(()))
            },
        );

        ext::treasury::lock::<Test>.mock_safe(move |account, amount_polka_btc| {
            assert_eq!(account, redeemer);
            assert_eq!(amount_polka_btc, amount);

            MockResult::Return(Ok(()))
        });

        ext::security::get_secure_id::<Test>.mock_safe(move |_| MockResult::Return(H256([0; 32])));

        assert_ok!(Redeem::request_redeem(
            Origin::signed(redeemer.clone()),
            amount,
            H160([0; 20]),
            BOB
        ));

        assert_emitted!(Event::RequestRedeem(
            H256([0; 32]),
            redeemer.clone(),
            amount,
            BOB,
            H160([0; 20]),
        ));
        assert_ok!(
            Redeem::get_redeem_request_from_id(&H256([0; 32])),
            RedeemRequest {
                vault: BOB,
                opentime: 1,
                amount_polka_btc: amount,
                amount_btc: amount / 2,
                amount_dot: amount / 2,
                premium_dot: 0,
                redeemer: redeemer.clone(),
                btc_address: H160([0; 20]),
            }
        );
    })
}

#[test]
fn test_execute_redeem_fails_with_redeem_id_not_found() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        assert_err!(
            Redeem::execute_redeem(
                Origin::signed(BOB),
                H256([0u8; 32]),
                H256Le::zero(),
                0,
                Vec::default(),
                Vec::default()
            ),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_execute_redeem_fails_with_unauthorized_vault() {
    run_test(|| {
        Redeem::get_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                vault: BOB,
                opentime: 0,
                amount_polka_btc: 0,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            }))
        });

        assert_err!(
            Redeem::execute_redeem(
                Origin::signed(CAROL),
                H256([0u8; 32]),
                H256Le::zero(),
                0,
                Vec::default(),
                Vec::default()
            ),
            TestError::UnauthorizedVault
        );
    })
}

#[test]
fn test_execute_redeem_fails_with_commit_period_expired() {
    run_test(|| {
        <frame_system::Module<Test>>::set_block_number(20);

        Redeem::get_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                vault: BOB,
                opentime: 30,
                amount_polka_btc: 0,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            }))
        });

        assert_err!(
            Redeem::execute_redeem(
                Origin::signed(BOB),
                H256([0u8; 32]),
                H256Le::zero(),
                0,
                Vec::default(),
                Vec::default()
            ),
            TestError::CommitPeriodExpired
        );
    })
}

#[test]
fn test_execute_redeem_succeeds() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        System::set_block_number(40);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                btc_address: H160([0; 20]),
                banned_until: None,
                status: VaultStatus::Active,
            },
        );
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                vault: BOB,
                opentime: 20,
                amount_polka_btc: 100,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            },
        );

        ext::treasury::burn::<Test>.mock_safe(move |redeemer, amount_polka_btc| {
            assert_eq!(redeemer, ALICE);
            assert_eq!(amount_polka_btc, 100);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_polka_btc| {
            assert_eq!(vault, &BOB);
            assert_eq!(amount_polka_btc, 100);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(BOB),
            H256([0u8; 32]),
            H256Le::zero(),
            0,
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem(H256([0; 32]), ALICE, BOB));
        assert_err!(
            Redeem::get_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemIdNotFound,
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_redeem_id_not_found() {
    run_test(|| {
        assert_err!(
            Redeem::cancel_redeem(Origin::signed(ALICE), H256([0u8; 32]), false),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_time_not_expired() {
    run_test(|| {
        System::set_block_number(20);

        Redeem::get_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                vault: BOB,
                opentime: 0,
                amount_polka_btc: 0,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            }))
        });

        assert_err!(
            Redeem::cancel_redeem(Origin::signed(ALICE), H256([0u8; 32]), false),
            TestError::TimeNotExpired
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_unauthorized_caller() {
    run_test(|| {
        <frame_system::Module<Test>>::set_block_number(20);

        Redeem::get_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                vault: BOB,
                opentime: 0,
                amount_polka_btc: 0,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            }))
        });

        assert_noop!(
            Redeem::cancel_redeem(Origin::signed(CAROL), H256([0u8; 32]), true),
            TestError::UnauthorizedUser
        );
    })
}

#[test]
fn test_cancel_redeem_succeeds() {
    run_test(|| {
        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                vault: BOB,
                opentime: 10,
                amount_polka_btc: 0,
                amount_btc: 0,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: H160([0; 20]),
            },
        );

        ext::vault_registry::ban_vault::<Test>.mock_safe(|vault, height| {
            assert_eq!(vault, BOB);
            assert_eq!(height, 1);
            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::cancel_redeem(
            Origin::signed(ALICE),
            H256([0u8; 32]),
            false
        ));
        assert_err!(
            Redeem::get_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemIdNotFound,
        );
        assert_emitted!(Event::CancelRedeem(H256([0; 32]), ALICE));
    })
}
