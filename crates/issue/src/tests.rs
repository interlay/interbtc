use crate::mock::*;
use crate::RawEvent;
use frame_support::{assert_noop, assert_ok};
/// Tests for Issue
use x_core::Error;

use bitcoin::types::H256Le;
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;

fn request_issue(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, Error> {
    Issue::_request_issue(origin, amount, vault, collateral)
}

fn insert_vault(id: AccountId) {
    <vault_registry::Module<Test>>::insert_vault(
        id,
        vault_registry::Vault {
            id: id,
            to_be_issued_tokens: 0,
            issued_tokens: 0,
            to_be_redeemed_tokens: 0,
            collateral: 0,
            btc_address: H160([0; 20]),
            banned_until: None,
        },
    );
}

fn insert_vaults(ids: &[AccountId]) {
    for id in ids {
        insert_vault(*id);
    }
}

fn request_issue_ok(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> H256 {
    insert_vaults(&[ALICE, BOB]);
    match Issue::_request_issue(origin, amount, vault, collateral) {
        Ok(act) => act,
        Err(err) => {
            println!("{}", err.message());
            panic!(err);
        }
    }
}

fn execute_issue(origin: AccountId, issue_id: &H256) -> Result<(), Error> {
    Issue::_execute_issue(
        origin,
        *issue_id,
        H256Le::zero(),
        0,
        vec![0u8; 100],
        vec![0u8; 100],
    )
}

fn execute_issue_ok(origin: AccountId, issue_id: &H256) {
    // TODO: mock btc_relay calls instead
    Issue::verify_inclusion_and_validate_transaction
        .mock_safe(|_, _, _, _, _, _, _| MockResult::Return(Ok(())));

    assert_ok!(execute_issue(origin, issue_id));
}

fn cancel_issue(origin: AccountId, issue_id: &H256) -> Result<(), Error> {
    Issue::_cancel_issue(origin, *issue_id)
}

fn create_test_vault() {
    <vault_registry::Module<Test>>::insert_vault(
        BOB,
        vault_registry::Vault {
            id: BOB,
            to_be_issued_tokens: 0,
            issued_tokens: 0,
            to_be_redeemed_tokens: 0,
            collateral: 0,
            btc_address: H160([0; 20]),
            banned_until: None,
        },
    );
}

#[test]
fn test_request_issue_banned_fails() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::insert_vault(
            BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                collateral: 0,
                btc_address: H160([0; 20]),
                banned_until: Some(1),
            },
        );
        assert_noop!(request_issue(ALICE, 3, BOB, 0), Error::VaultBanned);
    })
}

#[test]
fn test_request_issue_insufficient_collateral_fails() {
    run_test(|| {
        insert_vaults(&[ALICE, BOB]);
        Issue::set_issue_griefing_collateral(10);
        create_test_vault();
        assert_noop!(
            request_issue(ALICE, 3, BOB, 0),
            Error::InsufficientCollateral,
        );
    })
}

#[test]
fn test_request_issue_succeeds() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount: Balance = 3;
        create_test_vault();
        let issue_id = request_issue_ok(origin, amount, vault, 0);

        let request_issue_event = TestEvent::test_events(RawEvent::RequestIssue(
            issue_id,
            origin,
            amount,
            vault,
            H160([0; 20]),
        ));
        assert!(System::events()
            .iter()
            .any(|a| a.event == request_issue_event));
    })
}

#[test]
fn test_execute_issue_not_found_fails() {
    run_test(|| {
        create_test_vault();
        assert_noop!(execute_issue(ALICE, &H256([0; 32])), Error::IssueIdNotFound);
    })
}

#[test]
fn test_execute_issue_unauthorized_fails() {
    run_test(|| {
        create_test_vault();
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_noop!(execute_issue(CAROL, &issue_id), Error::UnauthorizedUser);
    })
}

#[test]
fn test_execute_issue_commit_period_expired_fails() {
    run_test(|| {
        create_test_vault();
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_noop!(execute_issue(ALICE, &issue_id), Error::CommitPeriodExpired);
    })
}

#[test]
fn test_execute_issue_succeeds() {
    run_test(|| {
        create_test_vault();
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        <system::Module<Test>>::set_block_number(20);
        Issue::set_issue_period(10);
        execute_issue_ok(ALICE, &issue_id);

        let execute_issue_event =
            TestEvent::test_events(RawEvent::ExecuteIssue(issue_id, ALICE, BOB));
        assert!(System::events()
            .iter()
            .any(|a| a.event == execute_issue_event));

        assert_noop!(cancel_issue(ALICE, &issue_id), Error::IssueIdNotFound);
    })
}

#[test]
fn test_cancel_issue_not_found_fails() {
    run_test(|| {
        assert_noop!(cancel_issue(ALICE, &H256([0; 32])), Error::IssueIdNotFound,);
    })
}

#[test]
fn test_cancel_issue_not_expired_fails() {
    run_test(|| {
        create_test_vault();
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        Issue::set_issue_period(2);
        <system::Module<Test>>::set_block_number(99);
        assert_noop!(cancel_issue(ALICE, &issue_id), Error::TimeNotExpired,);
    })
}

#[test]
fn test_cancel_issue_succeeds() {
    run_test(|| {
        Issue::set_issue_period(10);
        <system::Module<Test>>::set_block_number(20);
        create_test_vault();
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_ok!(cancel_issue(ALICE, &issue_id));
    })
}
