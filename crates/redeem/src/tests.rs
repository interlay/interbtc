use crate::{ext, mock::*};

use crate::types::{PolkaBTC, RedeemRequest, RedeemRequestStatus, DOT};
use btc_relay::{BtcAddress, BtcPublicKey};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use primitive_types::H256;
use security::Pallet as Security;
use sp_core::H160;
use sp_std::convert::TryInto;
use vault_registry::{VaultStatus, Wallet};

type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::redeem($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::redeem($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

fn btc_to_u128(amount: PolkaBTC<Test>) -> Result<u128, DispatchError> {
    TryInto::<u128>::try_into(amount).map_err(|_e| TestError::TryIntoIntError.into())
}

fn u128_to_dot(x: u128) -> Result<DOT<Test>, DispatchError> {
    TryInto::<DOT<Test>>::try_into(x).map_err(|_| TestError::TryIntoIntError.into())
}

fn btcdot_parity(btc: PolkaBTC<Test>) -> Result<DOT<Test>, DispatchError> {
    let dot = btc_to_u128(btc)?;
    u128_to_dot(dot)
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
        <treasury::Pallet<Test>>::mint(ALICE, 2);
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
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                replace_collateral: 0,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
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
            Redeem::request_redeem(Origin::signed(ALICE), 0, BtcAddress::default(), BOB),
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
            Redeem::request_redeem(Origin::signed(ALICE), 0, BtcAddress::default(), BOB),
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
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
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

        ext::treasury::lock::<Test>.mock_safe(move |account, amount_polka_btc| {
            assert_eq!(account, redeemer);
            assert_eq!(amount_polka_btc, amount);

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
        ));
        assert_ok!(
            Redeem::get_open_redeem_request_from_id(&H256([0; 32])),
            RedeemRequest {
                period: Redeem::redeem_period(),
                vault: BOB,
                opentime: 1,
                fee: redeem_fee,
                amount_btc: amount - redeem_fee,
                premium_dot: 0,
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
            assert_eq!(redeemer_id, ALICE);
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
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        assert_err!(
            Redeem::execute_redeem(Origin::signed(BOB), H256([0u8; 32]), Vec::default(), Vec::default()),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_execute_redeem_succeeds_with_another_account() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                backing_collateral: 0,
                replace_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
            },
        );
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 0))));

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            },
        );

        ext::treasury::burn::<Test>.mock_safe(move |redeemer, amount_polka_btc| {
            assert_eq!(redeemer, ALICE);
            assert_eq!(amount_polka_btc, 100);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_polka_btc, premium, _| {
            assert_eq!(vault, &BOB);
            assert_eq!(amount_polka_btc, 100);
            assert_eq!(premium, 0);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(ALICE),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem(
            H256([0; 32]),
            ALICE,
            100,
            0,
            BOB,
            Redeem::get_current_inclusion_fee().unwrap()
        ));
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCompleted,
        );
    })
}

#[test]
fn test_execute_redeem_fails_with_commit_period_expired() {
    run_test(|| {
        Security::<Test>::set_active_block_number(40);

        Redeem::get_open_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 20,
                fee: 0,
                amount_btc: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            }))
        });

        assert_err!(
            Redeem::execute_redeem(Origin::signed(BOB), H256([0u8; 32]), Vec::default(), Vec::default()),
            TestError::CommitPeriodExpired
        );
    })
}

#[test]
fn test_execute_redeem_succeeds() {
    run_test(|| {
        ext::oracle::btc_to_dots::<Test>.mock_safe(|x| MockResult::Return(btcdot_parity(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                backing_collateral: 0,
                replace_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active(true),
            },
        );
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 0))));

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            },
        );

        ext::treasury::burn::<Test>.mock_safe(move |redeemer, amount_polka_btc| {
            assert_eq!(redeemer, ALICE);
            assert_eq!(amount_polka_btc, 100);

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_polka_btc, premium, _| {
            assert_eq!(vault, &BOB);
            assert_eq!(amount_polka_btc, 100);
            assert_eq!(premium, 0);

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(BOB),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem(
            H256([0; 32]),
            ALICE,
            100,
            0,
            BOB,
            Redeem::get_current_inclusion_fee().unwrap()
        ));
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
                premium_dot: 0,
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
                premium_dot: 0,
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
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee().unwrap(),
            },
        );

        let current_block_number = ext::security::active_block_number::<Test>();
        Security::<Test>::set_active_block_number(current_block_number + Redeem::redeem_period() + 10);

        ext::vault_registry::ban_vault::<Test>.mock_safe(move |vault| {
            assert_eq!(vault, BOB);
            MockResult::Return(Ok(()))
        });
        ext::sla::calculate_slashed_amount::<Test>.mock_safe(move |_, _, _| MockResult::Return(Ok(0)));
        ext::vault_registry::slash_collateral_saturated::<Test>.mock_safe(move |_, _, _| MockResult::Return(Ok(0)));
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
        assert_emitted!(Event::CancelRedeem(H256([0; 32]), ALICE, BOB, 0, false));
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
