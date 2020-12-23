use crate::mock::*;
use crate::PolkaBTC;
use crate::RawEvent;
use crate::{ext, has_request_expired, Trait};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use primitive_types::H256;
use vault_registry::{Vault, VaultStatus, Wallet};

fn request_issue(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, DispatchError> {
    // Default: Parachain status is "RUNNING". Set manually for failure testing
    ext::security::ensure_parachain_status_running::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::increase_to_be_issued_tokens::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    Issue::_request_issue(origin, amount, vault, collateral)
}

fn request_issue_ok(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> H256 {
    ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

    // Default: Parachain status is "RUNNING". Set manually for failure testing
    ext::security::ensure_parachain_status_running::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::increase_to_be_issued_tokens::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    match Issue::_request_issue(origin, amount, vault, collateral) {
        Ok(act) => act,
        Err(err) => {
            panic!(err);
        }
    }
}

fn execute_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    ext::security::ensure_parachain_status_running::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    Issue::_execute_issue(
        origin,
        *issue_id,
        H256Le::zero(),
        vec![0u8; 100],
        vec![0u8; 100],
    )
}

fn execute_issue_ok(origin: AccountId, issue_id: &H256) {
    ext::security::ensure_parachain_status_running::<Test>.mock_safe(|| MockResult::Return(Ok(())));

    ext::btc_relay::verify_transaction_inclusion::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(())));

    ext::btc_relay::validate_transaction::<Test>.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

    assert_ok!(execute_issue(origin, issue_id));
}

fn cancel_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    Issue::_cancel_issue(origin, *issue_id)
}

fn init_zero_vault<T: Trait>(id: T::AccountId) -> Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>> {
    let mut vault = Vault::default();
    vault.id = id;
    vault
}

fn get_dummy_request_id() -> H256 {
    return H256::zero();
}

#[test]
fn test_request_issue_banned_fails() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <frame_system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                wallet: Wallet::new(BtcAddress::random()),
                banned_until: Some(1),
                status: VaultStatus::Active,
            },
        );
        assert_noop!(
            request_issue(ALICE, 3, BOB, 0),
            VaultRegistryError::VaultBanned
        );
    })
}

#[test]
fn test_request_issue_insufficient_collateral_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::oracle::btc_to_dots::<Test>.mock_safe(|_| MockResult::Return(Ok(10000000)));

        assert_noop!(
            request_issue(ALICE, 3, BOB, 0),
            TestError::InsufficientCollateral,
        );
    })
}

#[test]
fn test_request_issue_succeeds() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount: Balance = 3;
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(origin, amount, vault, 20);

        let request_issue_event = TestEvent::test_events(RawEvent::RequestIssue(
            issue_id,
            origin,
            amount,
            vault,
            BtcAddress::default(),
        ));
        assert!(System::events()
            .iter()
            .any(|a| a.event == request_issue_event));
    })
}

#[test]
fn test_execute_issue_not_found_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        assert_noop!(
            execute_issue(ALICE, &H256([0; 32])),
            TestError::IssueIdNotFound
        );
    })
}

#[test]
fn test_execute_issue_commit_period_expired_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <frame_system::Module<Test>>::set_block_number(20);
        assert_noop!(
            execute_issue(ALICE, &issue_id),
            TestError::CommitPeriodExpired
        );
    })
}

#[test]
fn test_execute_issue_succeeds() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        <frame_system::Module<Test>>::set_block_number(5);
        execute_issue_ok(ALICE, &issue_id);

        let execute_issue_event =
            TestEvent::test_events(RawEvent::ExecuteIssue(issue_id, ALICE, BOB));
        assert!(System::events()
            .iter()
            .any(|a| a.event == execute_issue_event));

        assert_noop!(cancel_issue(ALICE, &issue_id), TestError::IssueCompleted);
    })
}

#[test]
fn test_cancel_issue_not_found_fails() {
    run_test(|| {
        assert_noop!(
            cancel_issue(ALICE, &H256([0; 32])),
            TestError::IssueIdNotFound,
        );
    })
}

#[test]
fn test_cancel_issue_not_expired_fails() {
    run_test(|| {
        <frame_system::Module<Test>>::set_block_number(1);

        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        // issue period is 10, we issued at block 1, so at block 5 the cancel should fail
        <frame_system::Module<Test>>::set_block_number(5);
        assert_noop!(cancel_issue(ALICE, &issue_id), TestError::TimeNotExpired,);
    })
}

#[test]
fn test_cancel_issue_succeeds() {
    run_test(|| {
        <frame_system::Module<Test>>::set_block_number(1);

        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::decrease_to_be_issued_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 20);
        // issue period is 10, we issued at block 1, so at block 15 the cancel should succeed
        <frame_system::Module<Test>>::set_block_number(15);
        assert_ok!(cancel_issue(ALICE, &issue_id));
    })
}

#[test]
fn test_request_issue_parachain_not_running_fails() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount: Balance = 3;

        ext::security::ensure_parachain_status_running::<Test>
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

        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_noop!(
            Issue::_execute_issue(
                origin,
                H256::zero(),
                H256Le::zero(),
                vec![0u8; 100],
                vec![0u8; 100],
            ),
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

#[test]
fn test_has_request_expired() {
    run_test(|| {
        System::set_block_number(45);
        assert!(has_request_expired::<Test>(9, 20));
        assert!(!has_request_expired::<Test>(30, 24));
    })
}
