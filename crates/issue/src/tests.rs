use crate::{ext, mock::*, Backing, Config, Issuing, RawEvent};

use btc_relay::{BtcAddress, BtcPublicKey};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_arithmetic::FixedU128;
use sp_core::H160;
use sp_runtime::FixedPointNumber;
use vault_registry::{Vault, VaultStatus, Wallet};

fn request_issue(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, DispatchError> {
    // Default: Parachain status is "RUNNING". Set manually for failure testing
    ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    ext::vault_registry::register_deposit_address::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    Issue::_request_issue(origin, amount, vault, collateral)
}

fn request_issue_ok(origin: AccountId, amount: Balance, vault: AccountId, collateral: Balance) -> H256 {
    ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

    // Default: Parachain status is "RUNNING". Set manually for failure testing
    ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    ext::vault_registry::register_deposit_address::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    Issue::_request_issue(origin, amount, vault, collateral).unwrap()
}

fn execute_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    Issue::_execute_issue(origin, *issue_id, vec![0u8; 100], vec![0u8; 100])
}

fn cancel_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    Issue::_cancel_issue(origin, *issue_id)
}

fn init_zero_vault<T: Config>(
    id: T::AccountId,
) -> Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>, <T as vault_registry::Config>::SignedFixedPoint> {
    let mut vault = Vault::default();
    vault.id = id;
    vault
}

fn get_dummy_request_id() -> H256 {
    H256::zero()
}

#[test]
fn test_request_issue_banned_fails() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Pallet<Test>>::_set_exchange_rate(
            FixedU128::one()
        ));
        <vault_registry::Pallet<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                backing_collateral: 0,
                wallet: Wallet::new(BtcPublicKey::default()),
                banned_until: Some(1),
                status: VaultStatus::Active(true),
                ..Default::default()
            },
        );
        assert_noop!(request_issue(ALICE, 3, BOB, 0), VaultRegistryError::VaultBanned);
    })
}

#[test]
fn test_request_issue_insufficient_collateral_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        ext::oracle::issuing_to_backing::<Test>.mock_safe(|_| MockResult::Return(Ok(10000000)));

        assert_noop!(request_issue(ALICE, 3, BOB, 0), TestError::InsufficientCollateral,);
    })
}

#[test]
fn test_request_issue_succeeds() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount: Balance = 3;
        let issue_fee = 1;
        let issue_griefing_collateral = 20;

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        ext::fee::get_issue_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(issue_fee)));

        ext::fee::get_issue_griefing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(issue_griefing_collateral)));

        let issue_id = request_issue_ok(origin, amount, vault, issue_griefing_collateral);

        let request_issue_event = TestEvent::issue(RawEvent::RequestIssue(
            issue_id,
            origin,
            amount - issue_fee,
            issue_fee,
            issue_griefing_collateral,
            vault,
            BtcAddress::default(),
            BtcPublicKey::default(),
        ));
        assert!(System::events().iter().any(|a| a.event == request_issue_event));
    })
}

#[test]
fn test_execute_issue_not_found_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        assert_noop!(execute_issue(ALICE, &H256([0; 32])), TestError::IssueIdNotFound);
    })
}

#[test]
fn test_execute_issue_commit_period_expired_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <security::Pallet<Test>>::set_active_block_number(20);
        assert_noop!(execute_issue(ALICE, &issue_id), TestError::CommitPeriodExpired);
    })
}

#[test]
fn test_execute_issue_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <security::Pallet<Test>>::set_active_block_number(5);

        ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));
        ext::btc_relay::verify_and_validate_transaction::<Test>
            .mock_safe(|_, _, _, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 3))));

        assert_ok!(execute_issue(ALICE, &issue_id));

        let execute_issue_event = TestEvent::issue(RawEvent::ExecuteIssue(issue_id, ALICE, 3, BOB));
        assert!(System::events().iter().any(|a| a.event == execute_issue_event));

        assert_noop!(cancel_issue(ALICE, &issue_id), TestError::IssueCompleted);
    })
}

#[test]
fn test_execute_issue_overpayment_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <security::Pallet<Test>>::set_active_block_number(5);
        ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));

        ext::btc_relay::verify_and_validate_transaction::<Test>
            .mock_safe(|_, _, _, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 5))));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        unsafe {
            let mut increase_tokens_called = false;
            let mut refund_called = false;

            ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_raw(|_, amount| {
                increase_tokens_called = true;
                assert_eq!(amount, 2);
                MockResult::Return(Ok(()))
            });

            // check that request_refund is not called..
            ext::refund::request_refund::<Test>.mock_raw(|_, _, _, _, _| {
                refund_called = true;
                MockResult::Return(Ok(None))
            });

            assert_ok!(execute_issue(ALICE, &issue_id));
            assert_eq!(increase_tokens_called, true);
            assert_eq!(refund_called, false);
        }
    })
}

#[test]
fn test_execute_issue_refund_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <security::Pallet<Test>>::set_active_block_number(5);
        ext::security::ensure_parachain_status_not_shutdown::<Test>.mock_safe(|| MockResult::Return(Ok(())));

        // pay 103 instead of the expected 3
        ext::btc_relay::verify_and_validate_transaction::<Test>
            .mock_safe(|_, _, _, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 103))));

        // return some arbitrary error
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, amount| {
            assert_eq!(amount, 100);
            MockResult::Return(Err(TestError::IssueCompleted.into()))
        });
        ext::vault_registry::register_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        unsafe {
            let mut refund_called = false;

            // check that a refund for amount=100 is requested
            ext::refund::request_refund::<Test>.mock_raw(|amount, _, _, _, _| {
                refund_called = true;
                assert_eq!(amount, 100);
                MockResult::Return(Ok(None))
            });
            assert_ok!(execute_issue(ALICE, &issue_id));
            assert_eq!(refund_called, true);
        }
    })
}
#[test]
fn test_cancel_issue_not_found_fails() {
    run_test(|| {
        assert_noop!(cancel_issue(ALICE, &H256([0; 32])), TestError::IssueIdNotFound,);
    })
}

#[test]
fn test_cancel_issue_not_expired_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        // issue period is 10, we issued at block 1, so at block 5 the cancel should fail
        <security::Pallet<Test>>::set_active_block_number(5);
        assert_noop!(cancel_issue(ALICE, &issue_id), TestError::TimeNotExpired,);
    })
}

#[test]
fn test_cancel_issue_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        ext::vault_registry::decrease_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        ext::vault_registry::transfer_funds::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        // issue period is 10, we issued at block 1, so at block 15 the cancel should succeed
        <security::Pallet<Test>>::set_active_block_number(15);
        assert_ok!(cancel_issue(ALICE, &issue_id));
    })
}

#[test]
fn test_request_issue_parachain_not_running_fails() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount: Balance = 3;

        ext::security::ensure_parachain_status_not_shutdown::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_noop!(
            Issue::_request_issue(origin, amount, vault, 0),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_execute_issue_parachain_not_running_fails() {
    run_test(|| {
        let origin = ALICE;

        ext::security::ensure_parachain_status_not_shutdown::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_noop!(
            Issue::_execute_issue(origin, H256::zero(), vec![0u8; 100], vec![0u8; 100],),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_set_issue_period_only_root() {
    run_test(|| {
        assert_noop!(
            Issue::set_issue_period(Origin::signed(ALICE), 1),
            DispatchError::BadOrigin
        );
        assert_ok!(Issue::set_issue_period(Origin::root(), 1));
    })
}
