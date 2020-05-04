use crate::mock::*;

//use crate::RawEvent;
use crate::ext;
use crate::DOT;
//use bitcoin::types::H256Le;
use crate::Replace as R;
use frame_support::assert_noop;
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use vault_registry::Vault;
use x_core::Error;

fn request_replace(
    origin: AccountId,
    vault: AccountId,
    amount: Balance,
    timeout: BlockNumber,
    griefing_collateral: DOT<Test>,
) -> Result<H256, Error> {
    Replace::_request_replace(origin, vault, amount, timeout, griefing_collateral)
}

fn withdraw_replace(vault_id: AccountId, request_id: H256) -> Result<(), Error> {
    Replace::_withdraw_replace_request(vault_id, request_id)
}

#[test]
fn test_request_replace_transfer_zero() {
    run_test(|| {
        assert_noop!(request_replace(0, BOB, 0, 0, 0), Error::InvalidAmount);
    })
}

#[test]
fn test_request_replace_timeout_zero() {
    run_test(|| {
        assert_noop!(request_replace(0, BOB, 1, 0, 0), Error::InvalidTimeout);
    })
}

#[test]
fn test_request_replace_vault_not_found() {
    run_test(|| {
        assert_noop!(request_replace(0, 10_000, 1, 1, 0), Error::VaultNotFound);
    })
}

#[test]
fn test_request_replace_vault_banned() {
    run_test(|| {
        //TODO(jaupe) work out why this is not mocking correctly
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
            Replace::_request_replace(ALICE, BOB, 1, 1, 0),
            Error::VaultBanned
        );
    })
}

#[test]
fn test_request_replace_insufficient_collateral() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            }))
        });
        ext::collateral::get_collateral_from_account::<Test>.mock_safe(|_| MockResult::Return(1));
        assert_noop!(
            Replace::_request_replace(ALICE, BOB, 1, 1, 0),
            Error::InsufficientCollateral
        );
    })
}

#[test]
fn test_withdraw_replace_request_invalid_request_id() {
    run_test(|| {
        Replace::get_replace_request
            .mock_safe(|_| MockResult::Return(Err(Error::InvalidReplaceID)));
        assert_noop!(
            Replace::_withdraw_replace_request(ALICE, H256([0u8; 32])),
            Error::InvalidReplaceID
        );
    })
}

fn test_request() -> R<u64, u64, u64, u64> {
    R {
        new_vault: None,
        old_vault: ALICE,
        open_time: 0,
        accept_time: None,
        amount: 10,
        griefing_collateral: 0,
        btc_address: H160([0; 20]),
        collateral: 20,
    }
}

#[test]
fn test_withdraw_replace_request_invalid_vault_id() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Err(Error::VaultNotFound)));
        assert_noop!(
            withdraw_replace(ALICE, H256([0u8; 32])),
            Error::VaultNotFound
        );
    })
}
