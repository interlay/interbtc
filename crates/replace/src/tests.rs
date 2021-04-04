use crate::{ext, mock::*, ReplaceRequest, ReplaceRequestStatus};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;

type Event = crate::Event<Test>;

// // use macro to avoid messing up stack trace
// macro_rules! assert_emitted {
//     ($event:expr) => {
//         let test_event = TestEvent::replace($event);
//         assert!(System::events().iter().any(|a| a.event == test_event));
//     };
//     ($event:expr, $times:expr) => {
//         let test_event = TestEvent::replace($event);
//         assert_eq!(
//             System::events().iter().filter(|a| a.event == test_event).count(),
//             $times
//         );
//     };
// }
macro_rules! assert_event_matches {
    ($( $pattern:pat )|+ $( if $guard: expr )? $(,)?) => {

        assert!(System::events().iter().any(|a| {
            match a.event {
                TestEvent::replace( $( $pattern )|+ ) $( if $guard )? => true,
                _ => false
            }
        }));
    }
}
fn test_request() -> ReplaceRequest<u64, u64, u64, u64> {
    ReplaceRequest {
        period: 0,
        new_vault: NEW_VAULT,
        old_vault: OLD_VAULT,
        accept_time: 1,
        amount: 10,
        griefing_collateral: 0,
        btc_address: BtcAddress::default(),
        collateral: 20,
        open_bitcoin_height: 0,
        status: ReplaceRequestStatus::Pending,
    }
}

mod request_replace_tests {
    use super::*;

    fn setup_mocks() {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        ext::vault_registry::requestable_to_be_replaced_tokens::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(1000000)));
        ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok((2, 20))));
        ext::fee::get_replace_griefing_collateral::<Test>.mock_safe(move |_| MockResult::Return(Ok(20)));
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
                .mock_safe(move |_| MockResult::Return(Ok(5)));

            assert_ok!(Replace::_request_replace(OLD_VAULT, 10, 20));
            assert_event_matches!(Event::RequestReplace(OLD_VAULT, 5, 10));
        })
    }

    #[test]
    fn test_request_replace_total_to_be_replace_below_dust_fails() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
                .mock_safe(|_, _, _| MockResult::Return(Ok((1, 10))));
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
            ext::fee::get_replace_griefing_collateral::<Test>.mock_safe(move |_| MockResult::Return(Ok(25)));
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
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok((5, 10))));
        ext::vault_registry::try_lock_additional_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
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
                .mock_safe(|_, _| MockResult::Return(Ok((4, 8))));

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
                .mock_safe(|_, _| MockResult::Return(Ok((1, 10))));
            assert_err!(
                Replace::_accept_replace(OLD_VAULT, NEW_VAULT, 5, 10, BtcAddress::default()),
                TestError::AmountBelowDustAmount
            );
        })
    }
}

mod auction_replace_tests {
    use super::*;

    fn setup_mocks() {
        ext::vault_registry::is_vault_below_auction_threshold::<Test>.mock_safe(|_| MockResult::Return(Ok(true)));
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        ext::vault_registry::insert_vault_deposit_address::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok((5, 10))));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::get_auctionable_tokens::<Test>.mock_safe(|_| MockResult::Return(Ok(100)));
        ext::vault_registry::try_lock_additional_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::slash_collateral::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
    }

    #[test]
    fn test_auction_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_auction_replace(
                OLD_VAULT,
                NEW_VAULT,
                5,
                10,
                BtcAddress::default()
            ));
            assert_event_matches!(Event::AuctionReplace(_, OLD_VAULT, NEW_VAULT, 5, 10, _, _, addr) if addr == BtcAddress::default());
        })
    }

    #[test]
    fn test_auction_partial_replace_succeeds() {
        run_test(|| {
            // call to replace (5, 10), when there is only (4, 8) actually used
            setup_mocks();
            ext::vault_registry::get_auctionable_tokens::<Test>.mock_safe(|_| MockResult::Return(Ok(4)));
            assert_ok!(Replace::_auction_replace(
                OLD_VAULT,
                NEW_VAULT,
                5,
                10,
                BtcAddress::default()
            ));
            assert_event_matches!(Event::AuctionReplace(_, OLD_VAULT, NEW_VAULT, 4, 8, _, _, addr) if addr == BtcAddress::default());
        })
    }

    #[test]
    fn test_auction_replace_above_auction_threshold_fails() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::is_vault_below_auction_threshold::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));
            assert_err!(
                Replace::_auction_replace(OLD_VAULT, NEW_VAULT, 1, 10, BtcAddress::default()),
                TestError::VaultOverAuctionThreshold
            );
        })
    }

    #[test]
    fn test_auction_replace_below_dust_fails() {
        run_test(|| {
            setup_mocks();
            assert_err!(
                Replace::_auction_replace(OLD_VAULT, NEW_VAULT, 1, 10, BtcAddress::default()),
                TestError::AmountBelowDustAmount
            );
        })
    }

    #[test]
    fn test_auction_replace_auctionable_tokens_below_dust_fails() {
        run_test(|| {
            setup_mocks();
            ext::vault_registry::get_auctionable_tokens::<Test>.mock_safe(|_| MockResult::Return(Ok(1)));
            assert_err!(
                Replace::_auction_replace(OLD_VAULT, NEW_VAULT, 5, 10, BtcAddress::default()),
                TestError::AmountBelowDustAmount
            );
        })
    }
}

mod execute_replace_test {
    use super::*;

    fn setup_mocks() {
        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = OLD_VAULT;
            replace.new_vault = NEW_VAULT;
            MockResult::Return(Ok(replace))
        });

        Replace::replace_period.mock_safe(|| MockResult::Return(20));
        ext::security::has_expired::<Test>.mock_safe(|_, _| MockResult::Return(Ok(false)));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 0))));
        ext::vault_registry::replace_tokens::<Test>.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
    }

    #[test]
    fn test_execute_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_execute_replace(
                H256::zero(),
                H256Le::zero(),
                Vec::new(),
                Vec::new()
            ));
            assert_event_matches!(Event::ExecuteReplace(_, OLD_VAULT, NEW_VAULT));
        })
    }

    #[test]
    fn test_execute_replace_after_expiry_fails() {
        run_test(|| {
            setup_mocks();
            ext::security::has_expired::<Test>.mock_safe(|_, _| MockResult::Return(Ok(true)));

            assert_err!(
                Replace::_execute_replace(H256::zero(), H256Le::zero(), Vec::new(), Vec::new()),
                TestError::ReplacePeriodExpired
            );
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
        ext::security::has_expired::<Test>.mock_safe(|_, _| MockResult::Return(Ok(true)));
        ext::vault_registry::is_vault_liquidated::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));
        ext::vault_registry::cancel_replace_tokens::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::slash_collateral::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_allowed_to_withdraw_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(false)));
    }

    #[test]
    fn test_cancel_replace_succeeds() {
        run_test(|| {
            setup_mocks();
            assert_ok!(Replace::_cancel_replace(NEW_VAULT, H256::zero(),));
            assert_event_matches!(Event::CancelReplace(_, NEW_VAULT, OLD_VAULT, _));
        })
    }

    #[test]
    fn test_cancel_replace_invalid_caller_fails() {
        run_test(|| {
            setup_mocks();

            assert_err!(
                Replace::_cancel_replace(OLD_VAULT, H256::zero(),),
                TestError::UnauthorizedVault
            );
        })
    }
}
