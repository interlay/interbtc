use crate::{ext, mock::*, DefaultRefundRequest, Event};
use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::BtcAddress;
use currency::Amount;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use primitives::refund::RefundRequest;
use sp_core::{H160, H256};

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

fn wrapped(amount: u128) -> Amount<Test> {
    Amount::new(amount, Token(IBTC))
}

#[test]
fn test_refund_succeeds() {
    run_test(|| {
        ext::fee::get_refund_fee_from_total::<Test>.mock_safe(|_| MockResult::Return(Ok(wrapped(5))));
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));
        Refund::get_current_inclusion_fee.mock_safe(|_| MockResult::Return(Ok(wrapped(1))));
        let issue_id = H256::zero();
        assert_ok!(Refund::request_refund(
            &wrapped(1000),
            VAULT,
            USER,
            BtcAddress::P2SH(H160::zero()),
            issue_id
        ));

        // check the emitted event
        let captured_event = System::events()
            .iter()
            .find_map(|x| match &x.event {
                TestEvent::Refund(ref event) => Some(event.clone()),
                _ => None,
            })
            .unwrap();
        let refund_id = match captured_event {
            Event::<Test>::RequestRefund {
                refund_id,
                issuer,
                amount: 994,
                vault_id,
                btc_address: _,
                issue_id: issue,
                fee: 5,
                transfer_fee: 1,
            } if issuer == USER && vault_id == VAULT && issue == issue_id => Some(refund_id),
            _ => None,
        }
        .unwrap();

        assert_ok!(Refund::_execute_refund(refund_id, vec![0u8; 100], vec![0u8; 100],));
    })
}

fn default_refund_request() -> DefaultRefundRequest<Test> {
    RefundRequest {
        completed: Default::default(),
        amount_btc: Default::default(),
        btc_address: Default::default(),
        fee: Default::default(),
        transfer_fee_btc: Default::default(),
        issue_id: Default::default(),
        issuer: Default::default(),
        vault: VAULT,
    }
}
mod spec_based_tests {
    use crate::{RefundRequest, RefundRequests};

    use super::*;

    #[test]
    fn precondition_refund_must_exist() {
        run_test(|| {
            assert_err!(
                Refund::_execute_refund(H256::random(), vec![0u8; 100], vec![0u8; 100]),
                TestError::RefundIdNotFound,
            );
        })
    }

    #[test]
    fn precondition_refund_must_not_be_completed() {
        run_test(|| {
            let redeem_id = H256::random();
            <RefundRequests<Test>>::insert(
                redeem_id,
                RefundRequest {
                    completed: true,
                    ..default_refund_request()
                },
            );
            assert_err!(
                Refund::_execute_refund(redeem_id, vec![0u8; 100], vec![0u8; 100]),
                TestError::RefundCompleted,
            );
        })
    }

    #[test]
    fn postcondition_refund_must_be_completed() {
        run_test(|| {
            ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
            ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
            ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
                .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));
            ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
            ext::vault_registry::issue_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

            let redeem_id = H256::random();
            <RefundRequests<Test>>::insert(redeem_id, default_refund_request());

            assert_ok!(Refund::_execute_refund(redeem_id, vec![0u8; 100], vec![0u8; 100]));

            assert!(
                matches!(<RefundRequests<Test>>::get(redeem_id), Some(refund) if refund.completed),
                "refund request MUST be completed"
            )
        })
    }
}
