use crate::ext;
use crate::mock::*;
use crate::RawEvent;
use bitcoin::types::H256Le;
use frame_support::{assert_noop, assert_ok};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use vault_registry::Vault;
use x_core::Error;

fn request_redeem(
    origin: AccountId,
    amount: Balance,
    btc_address: H160,
    vault: AccountId,
) -> Result<H256, Error> {
    Redeem::_request_redeem(origin, amount, btc_address, vault)
}

#[test]
fn test_request_redeem_err_banned_fails() {
    run_test(|| {
        <exchange_rate_oracle::Module<Test>>::get_exchange_rate
            .mock_safe(|| MockResult::Return(Ok(1)));
        <system::Module<Test>>::block_number.mock_safe(|| MockResult::Return(0));

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: Some(1),
            }))
        });

        assert_noop!(
            request_redeem(ALICE, 0, H160::from_slice(&[0; 20]), BOB),
            Error::VaultBanned
        );
    })
}

#[test]
fn test_request_redeem_err_vault_not_found() {
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
        assert_noop!(
            request_redeem(ALICE, 0, H160::from_slice(&[0; 20]), 3),
            Error::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_amount_err_exceeds_user_balance() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            },
        );
        <treasury::Module<Test>>::mint(ALICE, 2);
        let amount = 10_000_000;
        assert_noop!(
            request_redeem(ALICE, amount, H160::from_slice(&[0; 20]), BOB),
            Error::AmountExceedsUserBalance
        );
    })
}

#[test]
fn test_request_redeem_amount_err_exceeds_vault_balance() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            },
        );
        <treasury::Module<Test>>::mint(ALICE, 2);
        let amount = 11;
        assert_noop!(
            request_redeem(ALICE, amount, H160::from_slice(&[0; 20]), BOB),
            Error::AmountExceedsVaultBalance
        );
    })
}

#[test]
fn test_request_redeem_amount_ok() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        let vault = vault_registry::Vault {
            id: BOB,
            to_be_issued_tokens: 0,
            issued_tokens: 10,
            to_be_redeemed_tokens: 0,
            btc_address: H160([0; 20]),
            banned_until: None,
        };
        <vault_registry::Module<Test>>::_insert_vault(&BOB, vault.clone());
        let amount = 9;
        request_redeem(ALICE, amount, H160::from_slice(&[0; 20]), BOB).unwrap();
        let key = Redeem::gen_redeem_key(ALICE);
        let redeemer = ALICE;
        let request_redeem_event = TestEvent::test_events(RawEvent::RequestRedeem(
            key,
            redeemer,
            amount,
            BOB,
            H160([0; 20]),
        ));
        assert!(System::events()
            .iter()
            .any(|a| a.event == request_redeem_event));
        //TODO(jaupe) test that there is a mapping
    })
}

#[test]
fn test_execute_redeem_err_id_not_found() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            },
        );
        <treasury::Module<Test>>::mint(ALICE, 2);
        let vault_id = 1000;
        let redeemer = ALICE;
        let redeem_id = H256([0u8; 32]);
        let tx_id = H256Le::zero();
        let tx_block_height = 0;
        let merkle_proof = Vec::default();
        let raw_tx = Vec::default();
        assert_noop!(
            Redeem::_execute_redeem(
                vault_id,
                redeemer,
                redeem_id,
                tx_id,
                tx_block_height,
                merkle_proof,
                raw_tx
            ),
            Error::RedeemIdNotFound
        );
    })
}

#[test]
fn test_execute_redeem_err_vault_not_found() {
    run_test(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(1));
        <system::Module<Test>>::set_block_number(0);
        <vault_registry::Module<Test>>::_insert_vault(
            &BOB,
            vault_registry::Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            },
        );
        Redeem::get_redeem_request_from_id
            .mock_safe(|_| MockResult::Return(Ok(Redeem::default()));
        let vault_id = 1000;
        let redeemer = ALICE;
        let redeem_id = H256([0u8; 32]);
        let tx_id = H256Le::zero();
        let tx_block_height = 0;
        let merkle_proof = Vec::default();
        let raw_tx = Vec::default();
        assert_noop!(
            Redeem::_execute_redeem(
                vault_id,
                redeemer,
                redeem_id,
                tx_id,
                tx_block_height,
                merkle_proof,
                raw_tx
            ),
            Error::RedeemIdNotFound
        );
    })
}
