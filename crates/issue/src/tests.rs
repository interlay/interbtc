use crate::{ext, mock::*, Event};

use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::Amount;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use sp_arithmetic::FixedU128;
use sp_core::{H160, H256};
use sp_runtime::traits::One;
use vault_registry::{DefaultVault, DefaultVaultId, Vault, VaultStatus, Wallet};

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

fn griefing(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_NATIVE_CURRENCY)
}

fn wrapped(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_WRAPPED_CURRENCY)
}

fn request_issue(origin: AccountId, amount: Balance, vault: DefaultVaultId<Test>) -> Result<H256, DispatchError> {
    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    ext::vault_registry::register_deposit_address::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    Issue::_request_issue(origin, amount, vault)
}

fn request_issue_ok(origin: AccountId, amount: Balance, vault: DefaultVaultId<Test>) -> H256 {
    ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

    ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(get_dummy_request_id()));

    ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    ext::vault_registry::get_bitcoin_public_key::<Test>.mock_safe(|_| MockResult::Return(Ok(BtcPublicKey::default())));
    ext::vault_registry::register_deposit_address::<Test>
        .mock_safe(|_, _| MockResult::Return(Ok(BtcAddress::default())));

    Issue::_request_issue(origin, amount, vault).unwrap()
}

fn execute_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    Issue::_execute_issue(origin, *issue_id, vec![0u8; 100], vec![0u8; 100])
}

fn cancel_issue(origin: AccountId, issue_id: &H256) -> Result<(), DispatchError> {
    Issue::_cancel_issue(origin, *issue_id)
}

fn init_zero_vault(id: DefaultVaultId<Test>) -> DefaultVault<Test> {
    Vault::new(id)
}

fn get_dummy_request_id() -> H256 {
    H256::zero()
}

#[test]
fn test_request_issue_banned_fails() {
    run_test(|| {
        assert_ok!(<oracle::Pallet<Test>>::_set_exchange_rate(
            DEFAULT_COLLATERAL_CURRENCY,
            FixedU128::one()
        ));
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                active_replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: Some(1),
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );
        assert_noop!(request_issue(USER, 3, VAULT), VaultRegistryError::VaultBanned);
    })
}

#[test]
fn test_request_issue_succeeds() {
    run_test(|| {
        let origin = USER;
        let vault = VAULT;
        let amount: Balance = 3;
        let issue_fee = 1;
        let issue_griefing_collateral = 20;

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));

        ext::fee::get_issue_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(wrapped(issue_fee))));

        ext::fee::get_issue_griefing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(griefing(issue_griefing_collateral))));

        let issue_id = request_issue_ok(origin, amount, vault.clone());

        let request_issue_event = TestEvent::Issue(Event::RequestIssue {
            issue_id,
            requester: origin,
            amount: amount - issue_fee,
            fee: issue_fee,
            griefing_collateral: issue_griefing_collateral,
            vault_id: vault,
            vault_address: BtcAddress::default(),
            vault_public_key: BtcPublicKey::default(),
        });
        assert!(System::events().iter().any(|a| a.event == request_issue_event));
    })
}

#[test]
fn test_execute_issue_not_found_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));
        assert_noop!(execute_issue(USER, &H256([0; 32])), TestError::IssueIdNotFound);
    })
}

#[test]
fn test_execute_issue_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        ext::fee::get_issue_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(wrapped(1))));

        let issue_id = request_issue_ok(USER, 3, VAULT);
        <security::Pallet<Test>>::set_active_block_number(5);

        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::get_and_verify_issue_payment::<Test, Balance>
            .mock_safe(|_, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 3))));

        assert_ok!(execute_issue(USER, &issue_id));

        let execute_issue_event = TestEvent::Issue(Event::ExecuteIssue {
            issue_id,
            requester: USER,
            vault_id: VAULT,
            amount: 3,
            fee: 1,
        });
        assert!(System::events().iter().any(|a| a.event == execute_issue_event));

        assert_noop!(cancel_issue(USER, &issue_id), TestError::IssueCompleted);
    })
}

#[test]
fn test_execute_issue_overpayment_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(USER, 3, VAULT);
        <security::Pallet<Test>>::set_active_block_number(5);

        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::get_and_verify_issue_payment::<Test, Balance>
            .mock_safe(|_, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 5))));

        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));

        unsafe {
            let mut increase_tokens_called = false;
            let mut refund_called = false;

            ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_raw(|_, amount| {
                increase_tokens_called = true;
                assert_eq!(amount, &wrapped(2));
                MockResult::Return(Ok(()))
            });

            // check that request_refund is not called..
            ext::refund::request_refund::<Test>.mock_raw(|_, _, _, _, _| {
                refund_called = true;
                MockResult::Return(Ok(None))
            });

            assert_ok!(execute_issue(USER, &issue_id));
            assert_eq!(increase_tokens_called, true);
            assert_eq!(refund_called, false);
        }
    })
}

#[test]
fn test_execute_issue_refund_succeeds() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let issue_id = request_issue_ok(USER, 3, VAULT);
        <security::Pallet<Test>>::set_active_block_number(5);

        // pay 103 instead of the expected 3
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::get_and_verify_issue_payment::<Test, Balance>
            .mock_safe(|_, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 103))));

        // return some arbitrary error
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, amount| {
            assert_eq!(amount, &wrapped(100));
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
                assert_eq!(amount, &wrapped(100));
                MockResult::Return(Ok(None))
            });
            assert_ok!(execute_issue(USER, &issue_id));
            assert_eq!(refund_called, true);
        }
    })
}
#[test]
fn test_cancel_issue_not_found_fails() {
    run_test(|| {
        assert_noop!(cancel_issue(USER, &H256([0; 32])), TestError::IssueIdNotFound,);
    })
}

#[test]
fn test_cancel_issue_not_expired_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(init_zero_vault(VAULT))));

        let issue_id = request_issue_ok(USER, 3, VAULT);
        // issue period is 10, we issued at block 1, so at block 5 the cancel should fail
        <security::Pallet<Test>>::set_active_block_number(5);
        assert_noop!(cancel_issue(USER, &issue_id), TestError::TimeNotExpired,);
    })
}

#[test]
fn test_set_issue_period_only_root() {
    run_test(|| {
        assert_noop!(
            Issue::set_issue_period(Origin::signed(USER), 1),
            DispatchError::BadOrigin
        );
        assert_ok!(Issue::set_issue_period(Origin::root(), 1));
    })
}
