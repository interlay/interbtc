mod mock;

use mock::*;
use primitive_types::H256;

type IssueCall = issue::Call<Runtime>;
type IssueEvent = issue::Event<Runtime>;

fn assert_issue_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().find(|record| match record.event {
        Event::issue(IssueEvent::RequestIssue(_, _, _, _, _)) => true,
        _ => false,
    });
    let id = if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _)) = record.unwrap().event {
        id
    } else {
        panic!("request issue event not found")
    };
    id
}

#[test]
fn integration_test_issue_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_parachain_status(StatusCode::Shutdown);

        assert_err!(
            IssueCall::request_issue(0, account_of(BOB), 0).dispatch(origin_of(account_of(ALICE))),
            Error::ParachainNotRunning,
        );

        assert_err!(
            IssueCall::execute_issue(
                H256([0; 32]),
                H256Le::zero(),
                0,
                vec![0u8; 32],
                vec![0u8; 32]
            )
            .dispatch(origin_of(account_of(ALICE))),
            Error::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_issue_polka_btc() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);

        let address = H160::from_slice(
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85")
                .unwrap()
                .as_slice(),
        );
        let amount = 100;

        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        assert_ok!(VaultRegistryCall::register_vault(1000000, address.clone())
            .dispatch(origin_of(account_of(BOB))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(IssueCall::request_issue(amount, account_of(BOB), 100)
            .dispatch(origin_of(account_of(ALICE))));

        let id = assert_issue_request_event();

        // send the btc from the user to the vault
        let (tx_id, height, proof, raw_tx) = generate_transaction_and_mine(address, amount, id);

        SystemModule::set_block_number(5);

        assert_ok!(IssueCall::execute_issue(id, tx_id, height, proof, raw_tx)
            .dispatch(origin_of(account_of(ALICE))));
    });
}
