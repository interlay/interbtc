/*
use crate::mock::*;
use crate::types::PolkaBTC;
use crate::RawEvent;
use crate::{ext, Trait};
use bitcoin::types::H256Le;
use frame_support::{assert_noop, assert_ok};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use vault_registry::Vault;
use x_core::Error;

fn request_issue(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, Error> {
    ext::vault_registry::increase_to_be_issued_tokens::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(H160::from_slice(&[0; 20]))));

    Issue::_request_issue(origin, amount, vault, collateral)
}

fn request_issue_ok(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> H256 {
    ext::vault_registry::increase_to_be_issued_tokens::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(H160::from_slice(&[0; 20]))));

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
    ext::btc_relay::verify_transaction_inclusion::<Test>
        .mock_safe(|_, _, _| MockResult::Return(Ok(())));

    ext::btc_relay::validate_transaction::<Test>.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

    assert_ok!(execute_issue(origin, issue_id));
}

fn cancel_issue(origin: AccountId, issue_id: &H256) -> Result<(), Error> {
    Issue::_cancel_issue(origin, *issue_id)
}

fn init_zero_vault<T: Trait>(id: T::AccountId) -> Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>> {
    let mut vault = Vault::default();
    vault.id = id;
    vault
}

#[test]
fn test_request_issue_banned_fails() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
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
        Issue::set_issue_griefing_collateral(10);
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

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
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

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
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        assert_noop!(execute_issue(ALICE, &H256([0; 32])), Error::IssueIdNotFound);
    })
}

#[test]
fn test_execute_issue_unauthorized_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_noop!(execute_issue(CAROL, &issue_id), Error::UnauthorizedUser);
    })
}

#[test]
fn test_execute_issue_commit_period_expired_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_noop!(execute_issue(ALICE, &issue_id), Error::CommitPeriodExpired);
    })
}

#[test]
fn test_execute_issue_succeeds() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

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
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));

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
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault::<Test>(BOB))));
        ext::vault_registry::decrease_to_be_issued_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(ALICE, 3, BOB, 0);
        assert_ok!(cancel_issue(ALICE, &issue_id));
    })
}

/* use crate::mock::*;
use frame_support::assert_noop;
use primitive_types::H256;
/// Tests for Replace
use x_core::Error;

// TODO(jaupe) mock crate wrappers

fn request_replace(
    origin: AccountId,
    vault_id: AccountId,
    amount: Balance,
    timeout: u64,
    collateral: Balance,
) -> Result<H256, Error> {
    Replace::_request_replace(origin, vault_id, amount, timeout, collateral)
}

fn store_banned_vault() {
    <vault_registry::Module<Test>>::_insert_vault(&BOB, vault_registry::Vault::default());
}

fn authorised_vault() -> vault_registry::Vault<u64, u64, u64> {
    vault_registry::Vault::default()
}

fn store_authorised_vault() {
    <vault_registry::Module<Test>>::_insert_vault(&BOB, authorised_vault());
}

#[test]
fn test_request_replace_invalid_amount() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let vault_id = 1;
        assert_noop!(
            request_replace(ALICE, vault_id, 0, 0, BOB),
            Error::InvalidAmount
        );
    })
}

#[test]
fn test_request_replace_invalid_timeout() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let amount = 1;
        let vault_id = 1;
        assert_noop!(
            request_replace(ALICE, vault_id, amount, 0, BOB),
            Error::InvalidTimeout
        );
    })
}

#[test]
fn test_request_replace_invalid_vault_id() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let amount = 1;
        let timeout = 1;
        let vault_id = 1;
        assert_noop!(
            request_replace(ALICE, vault_id, amount, timeout, BOB),
            Error::InvalidVaultID
        );
    })
}

#[test]
fn test_request_replace_vault_banned() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let amount = 1;
        let timeout = 1;
        store_banned_vault();
        let vault_id = 1;
        assert_noop!(
            request_replace(BOB, vault_id, amount, timeout, BOB),
            Error::VaultBanned
        );
    })
}

#[test]
fn test_request_replace_insufficient_griefing_amount_err() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        Replace::set_issue_griefing_collateral(1);
        let amount = 1;
        let timeout = 1;
        store_authorised_vault();
        let vault_id = 1;
        assert_noop!(
            request_replace(BOB, vault_id, amount, timeout, 1),
            Error::InsufficientCollateral
        );
    })
}

#[test]
fn test_request_replace_ok() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let amount = 100;
        let timeout = 1;
        let collateral = 0;
        let vault_id = BOB;
        store_authorised_vault();
        //TODO(jaupe) test key is correctly hashed
        assert!(request_replace(BOB, vault_id, amount, timeout, collateral).is_ok());
    })
}
 */
*/
