use crate::{ext, mock::*, ReplaceRequest, ReplaceRequestStatus};

use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::BtcAddress;
use currency::Amount;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use sp_core::H256;

type Event = crate::Event<Test>;

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

macro_rules! assert_event_matches {
    ($( $pattern:pat )|+ $( if $guard: expr )? $(,)?) => {

        assert!(System::events().iter().any(|a| {
            match a.event {
                TestEvent::Replace( $( $pattern )|+ ) $( if $guard )? => true,
                _ => false
            }
        }));
    }
}
fn test_request() -> ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId> {
    ReplaceRequest {
        period: 0,
        new_vault: NEW_VAULT,
        old_vault: OLD_VAULT,
        accept_time: 1,
        amount: 10,
        griefing_collateral: 0,
        btc_address: BtcAddress::default(),
        collateral: 20,
        btc_height: 0,
        status: ReplaceRequestStatus::Pending,
    }
}

fn griefing(amount: u128) -> Amount<Test> {
    Amount::new(amount, GRIEFING_CURRENCY)
}
fn wrapped(amount: u128) -> Amount<Test> {
    Amount::new(amount, INTERBTC)
}

mod request_replace_tests {
    use super::*;

    fn setup_mocks() {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        ext::vault_registry::requestable_to_be_replaced_tokens::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(wrapped(1000000))));
        ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok((wrapped(2), griefing(20)))));
        ext::fee::get_replace_griefing_collateral::<Test>.mock_safe(move |_| MockResult::Return(Ok(griefing(20))));
    }

    #[test]
    fn test_request_replace_total_to_be_replace_above_dust_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_request_replace(OLD_VAULT, 1, 10));
            assert_event_matches!(Event::RequestReplace(OLD_VAULT, 1, 10));
        })
    }

    #[test]
    fn test_request_replace_above_requestable_succeeds() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::requestable_to_be_replaced_tokens::<Test>
                .mock_safe(move |_| MockResult::Return(Ok(wrapped(5))));
            assert_ok!(Replace::_request_replace(OLD_VAULT, 10, 20));
            assert_event_matches!(Event::RequestReplace(OLD_VAULT, 5, 10));
        })
    }

    #[test]
    fn test_request_replace_total_to_be_replace_below_dust_fails() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
                .mock_safe(|_, _, _| MockResult::Return(Ok((wrapped(1), griefing(10)))));
            assert_err!(
                Replace::_request_replace(OLD_VAULT, 1, 10),
                TestError::AmountBelowDustAmount
            );
        })
    }

    #[test]
    fn test_request_replace_with_insufficient_griefing_collateral_fails() {
        run_test(|| {
            setup_mocks();
            ext::fee::get_replace_griefing_collateral::<Test>.mock_safe(move |_| MockResult::Return(Ok(griefing(25))));
            assert_err!(
                Replace::_request_replace(OLD_VAULT, 1, 10),
                TestError::InsufficientCollateral
            );
        })
    }
}

mod accept_replace_tests {
    use super::*;

    fn setup_mocks() {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        ext::vault_registry::insert_vault_deposit_address::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok((wrapped(5), griefing(10)))));
        ext::vault_registry::try_deposit_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    }

    #[test]
    fn test_accept_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_accept_replace(
                OLD_VAULT,
                NEW_VAULT,
                5,
                10,
                BtcAddress::default()
            ));
            assert_event_matches!(Event::AcceptReplace(_, OLD_VAULT, NEW_VAULT, 5, 10, addr) if addr == BtcAddress::default());
        })
    }

    #[test]
    fn test_accept_replace_partial_accept_succeeds() {
        run_test(|| {
            // call to replace (5, 10), when there is only (4, 8) actually used
            setup_mocks();
            ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
                .mock_safe(|_, _| MockResult::Return(Ok((wrapped(4), griefing(8)))));

            assert_ok!(Replace::_accept_replace(
                OLD_VAULT,
                NEW_VAULT,
                5,
                10,
                BtcAddress::default()
            ));
            assert_event_matches!(Event::AcceptReplace(_, OLD_VAULT, NEW_VAULT, 4, 8, addr) if addr == BtcAddress::default());
        })
    }

    #[test]
    fn test_accept_replace_below_dust_fails() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
                .mock_safe(|_, _| MockResult::Return(Ok((wrapped(1), griefing(10)))));
            assert_err!(
                Replace::_accept_replace(OLD_VAULT, NEW_VAULT, 5, 10, BtcAddress::default()),
                TestError::AmountBelowDustAmount
            );
        })
    }
}

mod execute_replace_test {
    use currency::Amount;

    use super::*;

    fn setup_mocks() {
        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = OLD_VAULT;
            replace.new_vault = NEW_VAULT;
            MockResult::Return(Ok(replace))
        });

        Replace::replace_period.mock_safe(|| MockResult::Return(20));
        ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(false)));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::replace_tokens::<Test>.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        Amount::<Test>::unlock_on.mock_safe(|_, _| MockResult::Return(Ok(())));
    }

    #[test]
    fn test_execute_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_execute_replace(H256::zero(), Vec::new(), Vec::new()));
            assert_event_matches!(Event::ExecuteReplace(_, OLD_VAULT, NEW_VAULT));
        })
    }
}

mod cancel_replace_tests {
    use super::*;

    fn setup_mocks() {
        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = OLD_VAULT;
            replace.new_vault = NEW_VAULT;
            MockResult::Return(Ok(replace))
        });

        Replace::replace_period.mock_safe(|| MockResult::Return(20));
        ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(true)));
        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));
        ext::vault_registry::cancel_replace_tokens::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::transfer_funds::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_allowed_to_withdraw_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(false)));
    }

    #[test]
    fn test_cancel_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_cancel_replace(NEW_VAULT.account_id, H256::zero(),));
            assert_event_matches!(Event::CancelReplace(_, NEW_VAULT, OLD_VAULT, _));
        })
    }

    #[test]
    fn test_cancel_replace_invalid_caller_fails() {
        run_test(|| {
            setup_mocks();

            assert_err!(
                Replace::_cancel_replace(OLD_VAULT.account_id, H256::zero(),),
                TestError::UnauthorizedVault
            );
        })
    }
}
