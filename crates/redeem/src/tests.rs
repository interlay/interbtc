use crate::{ext, mock::*};

use crate::types::{Collateral, RedeemRequest, RedeemRequestStatus, Wrapped};
use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::ParachainCurrency;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use security::Pallet as Security;
use sp_core::{H160, H256};
use sp_std::convert::TryInto;
use vault_registry::{VaultStatus, Wallet};

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Redeem($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::Redeem($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

fn convert_currency<I, O: std::convert::TryFrom<I>>(amount: I) -> Result<O, DispatchError> {
    TryInto::<O>::try_into(amount).map_err(|_e| TestError::TryIntoIntError.into())
}

fn btcdot_parity(wrapped: Wrapped<Test>) -> Result<Collateral<Test>, DispatchError> {
    let collateral: u128 = convert_currency(wrapped)?;
    convert_currency(collateral)
}

fn inject_redeem_request(key: H256, value: RedeemRequest<AccountId, BlockNumber, Balance, Balance>) {
    Redeem::insert_redeem_request(key, value)
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

#[test]
fn test_request_redeem_fails_with_amount_exceeds_user_balance() {
    run_test(|| {
        assert_ok!(<Test as vault_registry::Config>::Wrapped::mint(&ALICE, 2));
        let amount = 10_000_000;
        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), amount, BtcAddress::default(), BOB),
            TestError::AmountExceedsUserBalance
        );
    })
}

#[test]
fn test_request_redeem_fails_with_amount_below_minimum() {
    run_test(|| {
        ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                replace_collateral: 0,
                to_be_redeemed_tokens: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
                ..Default::default()
            },
        );

        let redeemer = ALICE;
        let amount = 9;

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(move |vault_id, amount_btc| {
            assert_eq!(vault_id, &BOB);
            assert_eq!(amount_btc, amount);

            MockResult::Return(Ok(()))
        });

        assert_err!(
            Redeem::request_redeem(Origin::signed(redeemer), 1, BtcAddress::random(), BOB),
            TestError::AmountBelowDustAmount
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_not_found() {
    run_test(|| {
        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 1500, BtcAddress::default(), BOB),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_banned() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>
            .mock_safe(|_| MockResult::Return(Err(VaultRegistryError::VaultBanned.into())));

        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 1500, BtcAddress::default(), BOB),
            VaultRegistryError::VaultBanned
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_liquidated() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        assert_err!(
            Redeem::request_redeem(Origin::signed(ALICE), 3, BtcAddress::random(), BOB),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_succeeds_with_normal_redeem() {
    run_test(|| {
        ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
                ..Default::default()
            },
        );

        let redeemer = ALICE;
        let amount = 9;
        let redeem_fee = 5;

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(move |vault_id, amount_btc| {
            assert_eq!(vault_id, &BOB);
            assert_eq!(amount_btc, amount - redeem_fee);

            MockResult::Return(Ok(()))
        });

        ext::treasury::lock::<Test>.mock_safe(move |account, amount_wrapped| {
            assert_eq!(account, &redeemer);
            assert_eq!(amount_wrapped, amount);

            MockResult::Return(Ok(()))
        });

        ext::security::get_secure_id::<Test>.mock_safe(move |_| MockResult::Return(H256([0; 32])));

        ext::fee::get_redeem_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(redeem_fee)));

        assert_ok!(Redeem::request_redeem(
            Origin::signed(redeemer),
            amount,
            BtcAddress::P2PKH(H160::zero()),
            BOB
        ));

        assert_emitted!(Event::RequestRedeem(
            H256([0; 32]),
            redeemer,
            amount - redeem_fee,
            redeem_fee,
            0,
            BOB,
            BtcAddress::P2PKH(H160::zero()),
            Redeem::get_current_inclusion_fee().unwrap()
        ));
        assert_ok!(
            Redeem::get_open_redeem_request_from_id(&H256([0; 32])),
            RedeemRequest {
                period: Redeem::redeem_period(),
                vault: BOB,
                opentime: 1,
                fee: redeem_fee,
                amount_btc: amount - redeem_fee,
                premium: 0,
                redeemer,
                btc_address: BtcAddress::P2PKH(H160::zero()),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            }
        );
    })
}

#[test]
fn test_liquidation_redeem_succeeds() {
    run_test(|| {
        let total_amount = 10 * 100_000_000;

        ext::treasury::get_balance::<Test>.mock_safe(move |_| MockResult::Return(total_amount));

        ext::treasury::lock::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::treasury::burn::<Test>.mock_safe(move |redeemer_id, amount| {
            assert_eq!(redeemer_id, &ALICE);
            assert_eq!(amount, total_amount);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens_liquidation::<Test>.mock_safe(move |redeemer_id, amount| {
            assert_eq!(redeemer_id, &ALICE);
            assert_eq!(amount, total_amount);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::liquidation_redeem(Origin::signed(ALICE), total_amount,));
    })
}

#[test]
fn test_execute_redeem_fails_with_redeem_id_not_found() {
    run_test(|| {
        ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        assert_err!(
            Redeem::execute_redeem(Origin::signed(BOB), H256([0u8; 32]), Vec::default(), Vec::default()),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_execute_redeem_succeeds_with_another_account() {
    run_test(|| {
        ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                replace_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
                ..Default::default()
            },
        );
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

        let btc_fee = Redeem::get_current_inclusion_fee().unwrap();

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: btc_fee,
            },
        );

        ext::treasury::burn::<Test>.mock_safe(move |redeemer, amount_wrapped| {
            assert_eq!(redeemer, &ALICE);
            assert_eq!(amount_wrapped, 100);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_wrapped, premium, _| {
            assert_eq!(vault, &BOB);
            assert_eq!(amount_wrapped, 100);
            assert_eq!(premium, 0);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(ALICE),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem(H256([0; 32]), ALICE, 100, 0, BOB, btc_fee,));
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCompleted,
        );
    })
}

#[test]
fn test_execute_redeem_succeeds() {
    run_test(|| {
        ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                replace_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
                ..Default::default()
            },
        );
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

        let btc_fee = Redeem::get_current_inclusion_fee().unwrap();

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: btc_fee,
            },
        );

        ext::treasury::burn::<Test>.mock_safe(move |redeemer, amount_wrapped| {
            assert_eq!(redeemer, &ALICE);
            assert_eq!(amount_wrapped, 100);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_wrapped, premium, _| {
            assert_eq!(vault, &BOB);
            assert_eq!(amount_wrapped, 100);
            assert_eq!(premium, 0);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(BOB),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem(H256([0; 32]), ALICE, 100, 0, BOB, btc_fee,));
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCompleted,
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
        Security::<Test>::set_active_block_number(10);

        Redeem::get_open_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 0,
                fee: 0,
                amount_btc: 0,
                premium: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
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
        Security::<Test>::set_active_block_number(20);

        Redeem::get_open_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 0,
                fee: 0,
                amount_btc: 0,
                premium: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
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
                period: 0,
                vault: BOB,
                opentime: 10,
                fee: 0,
                amount_btc: 0,
                premium: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            },
        );

        ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(true)));

        ext::vault_registry::ban_vault::<Test>.mock_safe(move |vault| {
            assert_eq!(vault, BOB);
            MockResult::Return(Ok(()))
        });
        ext::vault_registry::transfer_funds_saturated::<Test>.mock_safe(move |_, _, _| MockResult::Return(Ok(0)));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(vault_registry::types::Vault {
                status: VaultStatus::Active(true),
                ..Default::default()
            }))
        });
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_ok!(Redeem::cancel_redeem(Origin::signed(ALICE), H256([0u8; 32]), false));
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCancelled,
        );
        assert_emitted!(Event::CancelRedeem(
            H256([0; 32]),
            ALICE,
            BOB,
            0,
            RedeemRequestStatus::Retried
        ));
    })
}

#[test]
fn test_set_redeem_period_only_root() {
    run_test(|| {
        assert_noop!(
            Redeem::set_redeem_period(Origin::signed(ALICE), 1),
            DispatchError::BadOrigin
        );
        assert_ok!(Redeem::set_redeem_period(Origin::root(), 1));
    })
}

mod spec_based_tests {
    use super::*;

    #[test]
    fn test_request_reduces_to_be_replaced() {
        // Checked POSTCONDITION: `decreaseToBeReplacedTokens` MUST be called, supplying `vault` and `burnedTokens`.
        // The returned `replaceCollateral` MUST be released by this function.
        run_test(|| {
            let amount_to_redeem = 100;
            let replace_collateral = 100;
            assert_ok!(<Test as vault_registry::Config>::Wrapped::mint(
                &ALICE,
                amount_to_redeem
            ));
            ext::vault_registry::ensure_not_banned::<Test>.mock_safe(move |_vault_id| MockResult::Return(Ok(())));
            ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
                .mock_safe(move |_vault_id, _amount| MockResult::Return(Ok(())));
            ext::vault_registry::is_vault_below_premium_threshold::<Test>
                .mock_safe(move |_vault_id| MockResult::Return(Ok(false)));
            let redeem_fee = Fee::get_redeem_fee(amount_to_redeem).unwrap();
            let burned_tokens = amount_to_redeem - redeem_fee;

            ext::vault_registry::decrease_to_be_replaced_tokens::<Test>.mock_safe(move |vault_id, tokens| {
                assert_eq!(vault_id, &BOB);
                assert_eq!(tokens, burned_tokens);
                MockResult::Return(Ok((0, 0)))
            });

            // The returned `replaceCollateral` MUST be released
            ext::collateral::release_collateral::<Test>.mock_safe(move |vault_id, collateral| {
                assert_eq!(vault_id, &BOB);
                assert_eq!(collateral, replace_collateral);
                MockResult::Return(Ok(()))
            });

            assert_ok!(Redeem::request_redeem(
                Origin::signed(ALICE),
                amount_to_redeem,
                BtcAddress::random(),
                BOB
            ));
        })
    }
}
