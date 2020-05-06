mod mock;

use mock::*;

type IssueCall = issue::Call<Runtime>;
type IssueEvent = issue::Event<Runtime>;

#[test]
fn issue_polka_btc() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);

        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));

        assert_ok!(VaultRegistryCall::register_vault(1000, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));

        assert_ok!(IssueCall::request_issue(1000, account_of(BOB), 100)
            .dispatch(origin_of(account_of(ALICE))));

        let events = SystemModule::events();
        let record = events.iter().find(|record| match record.event {
            Event::issue(IssueEvent::RequestIssue(_, _, _, _, _)) => true,
            _ => false,
        });
        let id =
            if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _)) = record.unwrap().event {
                id
            } else {
                panic!("request issue event not found")
            };

        SystemModule::set_block_number(5);

        // btc_relay::Module::<Runtime>::_verify_transaction_inclusion
        //     .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

        // btc_relay::Module::<Runtime>::_validate_transaction
        //     .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        // assert_ok!(
        //     IssueCall::execute_issue(id, H256Le::zero(), 0, vec![0u8; 32], vec![0u8; 32])
        //         .dispatch(origin_of(account_of(ALICE)))
        // );
    });
}
