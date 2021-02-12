mod mock;

use mock::*;

use primitive_types::H256;

type ReplaceCall = replace::Call<Runtime>;
type ReplaceEvent = replace::Event<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;

// asserts request event happen and extracts its id for further testing
fn assert_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::RequestReplace(id, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

// asserts auction event happen and extracts its id for further testing
fn assert_auction_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::AuctionReplace(id, _, _, _, _, _, _, _, _)) => {
                Some(id.clone())
            }
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

        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(BOB))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_replace_request_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        let collateral = amount * 2;
        let griefing_collateral = 200;

        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
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
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let griefing_collateral = 2000;
        let amount = 50_000;
        let collateral = amount * 2;

        let bob = origin_of(account_of(BOB));

        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
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
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        let griefing_collateral = 500;
        let collateral = amount * 2;

        // alice creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            amount,
            dummy_public_key(),
        ))
        .dispatch(origin_of(account_of(ALICE))));
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        let replace_id = assert_request_event();
        // alice accept bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(ALICE))));
    });
}

#[test]
fn integration_test_replace_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            polkabtc,
            replace_collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        let final_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // take auction fee from old vault collateral
        let replace_amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let auction_fee = FeeModule::get_auction_redeem_fee(replace_amount_dot).unwrap();
        assert_eq!(
            final_old_vault_collateral,
            initial_old_vault_collateral - auction_fee
        );
    });
}

#[test]
fn integration_test_replace_execute_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let griefing_collateral = 500;
        let collateral = 4_000;
        let polkabtc = 1_000;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(polkabtc, griefing_collateral))
                .dispatch(origin_of(account_of(old_vault)))
        );

        let replace_id = assert_request_event();

        // alice accepts bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // send the btc from the old_vault to the new_vault
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(new_vault_btc_address, polkabtc, Some(replace_id));

        SystemModule::set_block_number(1 + CONFIRMATIONS);
        let r = Call::Replace(ReplaceCall::execute_replace(
            replace_id,
            tx_id,
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
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        //FIXME: get this from storage
        let griefing_collateral = 200;
        let collateral = amount * 2;

        // alice creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            amount,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(ALICE))));
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // alice accepts bob's request
        let replace_id = assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(BOB))));
        // set block height
        // alice cancels replacement
        SystemModule::set_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
            .dispatch(origin_of(account_of(BOB))));
    });
}

#[test]
fn integration_test_replace_cancel_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_new_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault));
        let initial_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            polkabtc,
            replace_collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // check old vault collateral
        let replace_amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let auction_fee = FeeModule::get_auction_redeem_fee(replace_amount_dot).unwrap();
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault)),
            initial_old_vault_collateral - auction_fee
        );
        // check new vault collateral
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault)),
            initial_new_vault_collateral + auction_fee + replace_collateral
        );

        let replace_id = assert_auction_event();

        SystemModule::set_block_number(30);

        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
            .dispatch(origin_of(account_of(BOB))));

        // check old vault collateral
        let amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let griefing_collateral = FeeModule::get_replace_griefing_collateral(amount_dot).unwrap();
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault)),
            initial_old_vault_collateral - auction_fee - griefing_collateral
        );

        // check new vault collateral. It should have received auction fee, griefing collateral and
        // the collateral that was reserved for this replace should have been released
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault)),
            initial_new_vault_collateral + auction_fee + griefing_collateral
        );
    });
}

#[test]
fn integration_test_replace_cancel_repeatedly_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address1 = BtcAddress::P2PKH(H160([2; 20]));
        let new_vault_btc_address2 = BtcAddress::P2PKH(H160([3; 20]));
        let new_vault_btc_address3 = BtcAddress::P2PKH(H160([4; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        // let initial_new_vault_collateral =
        //     collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault));
        // let initial_old_vault_collateral =
        //     collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            750,
            replace_collateral,
            new_vault_btc_address1
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            200,
            replace_collateral,
            new_vault_btc_address2
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // old_vault at this point only has 50 satoshi left, so this should fail
        // TODO: change back to assert_noop
        assert_noop!(
            Call::Replace(ReplaceCall::auction_replace(
                account_of(old_vault),
                200,
                replace_collateral,
                new_vault_btc_address3
            ))
            .dispatch(origin_of(account_of(new_vault))),
            VaultRegistryError::InsufficientTokensCommitted
        );
    });
}
