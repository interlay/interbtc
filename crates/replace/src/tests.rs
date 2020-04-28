use crate::mock::*;
use frame_support::assert_noop;
use primitive_types::H256;
use sp_core::H160;
/// Tests for Replace
use x_core::Error;

fn request_replace(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, Error> {
    Replace::_request_replace(origin, amount, vault, collateral)
}

fn store_banned_vault() {
    <vault_registry::Module<Test>>::insert_vault(
        BOB,
        vault_registry::Vault {
            vault: BOB,
            to_be_issued_tokens: 0,
            issued_tokens: 0,
            to_be_redeemed_tokens: 0,
            collateral: 0,
            btc_address: H160([0; 20]),
            banned_until: Some(1),
        },
    );
}

fn authorised_vault() -> vault_registry::Vault<u64, u64, u64, u64> {
    vault_registry::Vault {
        vault: BOB,
        to_be_issued_tokens: 0,
        issued_tokens: 0,
        to_be_redeemed_tokens: 0,
        collateral: 0,
        btc_address: H160([0; 20]),
        banned_until: None,
    }
}

fn store_authorised_vault() {
    <vault_registry::Module<Test>>::insert_vault(BOB, authorised_vault());
}

#[test]
fn test_request_replace_invalid_amount() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        assert_noop!(request_replace(ALICE, 0, 0, BOB), Error::InvalidAmount);
    })
}

#[test]
fn test_request_replace_invalid_timeout() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        let amount = 1;
        assert_noop!(
            request_replace(ALICE, amount, 0, BOB),
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
        assert_noop!(
            request_replace(ALICE, amount, timeout, BOB),
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
        assert_noop!(
            request_replace(BOB, amount, timeout, BOB),
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
        assert_noop!(
            request_replace(BOB, amount, timeout, 1),
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
        store_authorised_vault();
        //TODO(jaupe) test key is correctly hashed
        assert!(request_replace(BOB, amount, timeout, collateral).is_ok());
    })
}
