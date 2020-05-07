mod mock;

use mock::*;
use primitive_types::H256;

type ReplaceCall = replace::Call<Runtime>;
type ReplaceEvent = replace::Event<Runtime>;

#[test]
fn integration_test_replace_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_parachain_status(StatusCode::Shutdown);

        assert_err!(
            ReplaceCall::request_replace(0, 0).dispatch(origin_of(account_of(BOB))),
            Error::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_replace_request_replace() {
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

#[test]
fn integration_test_replace_withdraw_replace() {
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
        let replace_id = if let Event::replace(ReplaceEvent::RequestReplace(_, _, id)) =
            record.unwrap().event.clone()
        {
            id
        } else {
            panic!("request replace event not found")
        };

        assert_ok!(ReplaceCall::withdraw_replace(replace_id).dispatch(origin_of(account_of(BOB))));
        let event_found = events.iter().find(|record| match record.event {
            Event::replace(ReplaceEvent::WithdrawReplace(_, _)) => true,
            _ => false,
        });
        assert!(event_found.is_some());
    });
}

#[test]
fn integration_test_replace_accept_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let griefing_collateral = 500;
        let collateral = amount * 2;
        let replace_id = H256::zero();

        // peg spot rate
        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(ALICE))));
        // alice creates a vault
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));
        // bob requests a replace
        assert_ok!(ReplaceCall::request_replace(amount, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice accept bob's request
        assert_ok!(ReplaceCall::accept_replace(replace_id, collateral)
            .dispatch(origin_of(account_of(ALICE))));
    });
}

#[test]
fn integration_test_replace_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let griefing_collateral = 200;
        let collateral = amount * 2;

        // peg spot rate
        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(ALICE))));
        // alice creates a vault
        assert_ok!(VaultRegistryCall::register_vault(10, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));
        // bob requests a replace
        assert_ok!(ReplaceCall::request_replace(amount, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice auctions bob's vault
        assert_ok!(
            ReplaceCall::auction_replace(account_of(BOB), amount, collateral)
                .dispatch(origin_of(account_of(ALICE)))
        );
    });
}

#[test]
fn integration_test_replace_execute_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let griefing_collateral = 200;
        let replace_id = H256::zero();

        // peg spot rate
        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(ALICE))));
        // alice creates a vault
        assert_ok!(VaultRegistryCall::register_vault(10, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));
        // bob requests a replace
        assert_ok!(ReplaceCall::request_replace(amount, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice accepts bob's request
        assert_ok!(ReplaceCall::accept_replace(replace_id, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice excutes replacement of bob's vault
        // TODO(jaupe) populate the bitcoin data correctly
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let tx_block_height = 0;
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();
        let r =
            ReplaceCall::execute_replace(replace_id, tx_id, tx_block_height, merkle_proof, raw_tx)
                .dispatch(origin_of(account_of(BOB)));
        assert_ok!(r);
    });
}

#[test]
fn integration_test_replace_cancel_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let replace_id = H256::default();
        //FIXME: get this from storage
        let griefing_collateral = 200;
        let collateral = amount * 2;
        // peg spot rate
        assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
            .dispatch(origin_of(account_of(ALICE))));
        // alice creates a vault
        assert_ok!(VaultRegistryCall::register_vault(10, H160([0; 20]))
            .dispatch(origin_of(account_of(BOB))));
        // bob requests a replace
        assert_ok!(ReplaceCall::request_replace(amount, griefing_collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice accepts bob's request
        assert_ok!(ReplaceCall::accept_replace(replace_id, collateral)
            .dispatch(origin_of(account_of(BOB))));
        // alice cancels replacement
        assert_ok!(ReplaceCall::cancel_replace(replace_id).dispatch(origin_of(account_of(BOB))));
    });
}
