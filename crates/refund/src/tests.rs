use crate::ext;
use crate::mock::*;
use crate::RawEvent;
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::assert_ok;
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;

#[test]
fn test_refund_succeeds() {
    run_test(|| {
        ext::fee::get_refund_fee_from_total::<Test>.mock_safe(|_| MockResult::Return(Ok(5)));
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 995))));

        let issue_id = H256::zero();
        assert_ok!(Refund::request_refund(
            1000,
            VAULT,
            USER,
            BtcAddress::P2SH(H160::zero()),
            issue_id
        ));

        // check the emitted event
        let captured_event = System::events()
            .iter()
            .find_map(|x| match &x.event {
                TestEvent::test_events(ref event) => Some(event.clone()),
                _ => None,
            })
            .unwrap();
        let refund_id = match captured_event {
            RawEvent::RequestRefund(refund_id, issuer, 995, vault, _btc_address, issue)
                if issuer == USER && vault == VAULT && issue == issue_id =>
            {
                Some(refund_id)
            }
            _ => None,
        }
        .unwrap();

        assert_ok!(Refund::_execute_refund(
            refund_id,
            H256Le::zero(),
            vec![0u8; 100],
            vec![0u8; 100],
        ));
    })
}
