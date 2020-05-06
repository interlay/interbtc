mod mock;

use mock::*;

type ReplaceCall = replace::Call<Runtime>;
type ReplaceEvent = replace::Event<Runtime>;

#[test]
fn replace_request() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let griefing_collateral = 200;

        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));
        assert_ok!(ReplaceCall::request_replace(amount, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));

        let events = SystemModule::events();
        let record = events.iter().find(|record| match record.event {
            Event::replace(ReplaceEvent::RequestReplace(_, _, _)) => true,
            _ => false,
        });
        let _id = if let Event::replace(ReplaceEvent::RequestReplace(id, _, _)) =
            record.unwrap().event.clone()
        {
            id
        } else {
            panic!("request replace event not found")
        };
    });
}
