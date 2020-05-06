mod mock;

use mock::*;

type RedeemCall = redeem::Call<Runtime>;
type RedeemEvent = redeem::Event<Runtime>;

#[test]
fn redeem_polka_btc() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);

        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));

        assert_ok!(VaultRegistryCall::register_vault(10000, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));

        assert_ok!(
            vault_registry::Module::<Runtime>::_increase_to_be_issued_tokens(
                &account_of(BOB),
                1000,
            ),
            H160([0; 20])
        );

        assert_ok!(vault_registry::Module::<Runtime>::_issue_tokens(
            &account_of(BOB),
            1000
        ));

        assert_ok!(
            RedeemCall::request_redeem(1000, H160([0; 20]), account_of(BOB))
                .dispatch(origin_of(account_of(ALICE)))
        );

        let events = SystemModule::events();
        let record = events.iter().find(|record| match record.event {
            Event::redeem(RedeemEvent::RequestRedeem(_, _, _, _, _)) => true,
            _ => false,
        });
        let _id = if let Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _)) =
            record.unwrap().event
        {
            id
        } else {
            panic!("request redeem event not found")
        };
    });
}
