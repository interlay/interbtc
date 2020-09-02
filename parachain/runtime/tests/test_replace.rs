mod mock;

use mock::*;

use primitive_types::H256;

type ReplaceCall = replace::Call<Runtime>;
type ReplaceEvent = replace::Event<Runtime>;

// asserts request event happen and extracts its id for further testing
fn assert_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::RequestReplace(_, _, id)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

#[test]
fn integration_test_replace_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_parachain_status(StatusCode::Shutdown);

        assert_err!(
            Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(BOB))),
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

        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(amount, H160([0; 20])))
                .dispatch(origin_of(account_of(BOB)))
        );
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // assert request event
        let _request_id = assert_request_event();
    });
}

#[test]
fn integration_test_replace_withdraw_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let griefing_collateral = 200;
        let collateral = 100_000;

        let bob = origin_of(account_of(BOB));
        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            H160([0; 20])
        ))
        .dispatch(origin_of(account_of(BOB))));
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(5000, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // bob withdraws his replace
        let replace_id = assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(replace_id)).dispatch(bob));
    });
}

#[test]
fn integration_test_replace_accept_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        let griefing_collateral = 500;
        let collateral = amount * 2;

        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(amount, H160([0; 20])))
                .dispatch(origin_of(account_of(ALICE)))
        );
        // alice creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(amount, H160([0; 20])))
                .dispatch(origin_of(account_of(BOB)))
        );
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        let replace_id = assert_request_event();
        // alice accept bob's request
        assert_ok!(
            Call::Replace(ReplaceCall::accept_replace(replace_id, collateral))
                .dispatch(origin_of(account_of(ALICE)))
        );
    });
}

#[test]
fn integration_test_replace_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let user = CLAIRE;
        let old_vault = ALICE;
        let new_vault = BOB;
        let collateral = 4_000;
        let polkabtc = 1_000;
        let vault_btc_address = H160([0u8; 20]);

        set_default_thresholds();
        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc, vault_btc_address);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(3))
            .dispatch(origin_of(account_of(BOB))));
        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            polkabtc,
            2 * collateral
        ))
        .dispatch(origin_of(account_of(new_vault))));
    });
}

#[test]
fn integration_test_replace_execute_replace() {
    ExtBuilder::build().execute_with(|| {
        let user = CLAIRE;
        let old_vault = ALICE;
        let new_vault = BOB;
        let griefing_collateral = 50;
        let collateral = 4_000;
        let polkabtc = 1_000;
        let vault_btc_address = H160([0u8; 20]);

        set_default_thresholds();
        SystemModule::set_block_number(1);

        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc, vault_btc_address);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(polkabtc, griefing_collateral))
                .dispatch(origin_of(account_of(old_vault)))
        );

        let replace_id = assert_request_event();

        // alice accepts bob's request
        assert_ok!(
            Call::Replace(ReplaceCall::accept_replace(replace_id, collateral))
                .dispatch(origin_of(account_of(new_vault)))
        );

        // send the btc from the old_vault to the new_vault
        let (tx_id, tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, polkabtc, replace_id);

        let r = Call::Replace(ReplaceCall::execute_replace(
            replace_id,
            tx_id,
            tx_block_height,
            merkle_proof,
            raw_tx,
        ))
        .dispatch(origin_of(account_of(old_vault)));
        assert_ok!(r);
    });
}

#[test]
fn integration_test_replace_cancel_replace() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        let amount = 1000;
        //FIXME: get this from storage
        let griefing_collateral = 200;
        let collateral = amount * 2;
        // peg spot rate
        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));
        // bob creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(amount, H160([0; 20])))
                .dispatch(origin_of(account_of(ALICE)))
        );
        // alice creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(10, H160([0; 20])))
                .dispatch(origin_of(account_of(BOB)))
        );
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // alice accepts bob's request
        let replace_id = assert_request_event();
        assert_ok!(
            Call::Replace(ReplaceCall::accept_replace(replace_id, collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // set block height
        // alice cancels replacement
        SystemModule::set_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
            .dispatch(origin_of(account_of(BOB))));
    });
}
