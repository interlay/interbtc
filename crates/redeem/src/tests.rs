use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use primitive_types::H256;
use sp_core::H160;
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
fn test_request_redeem_banned_fails() {
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
            request_redeem(ALICE, 0, H160::from_slice(&[0; 20]), BOB),
            Error::VaultBanned
        );
    })
}
