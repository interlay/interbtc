mod mock;

use mock::*;

use primitive_types::H256;

type RedeemCall = redeem::Call<Runtime>;
type RedeemEvent = redeem::Event<Runtime>;

// asserts redeem event happen and extracts its id for further testing
fn assert_redeem_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

#[test]
fn integration_test_redeem_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_parachain_status(StatusCode::Shutdown);

        assert_err!(
            Call::Redeem(RedeemCall::request_redeem(
                1000,
                H160([0; 20]),
                account_of(BOB)
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_redeem_polka_btc() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let collateral_vault = 10_000;
        let polkabtc = 1_000;
        let vault_btc_address = H160([0u8; 20]);
        let user_btc_address = H160([0u8; 20]);

        SystemModule::set_block_number(1);

        assert_ok!(Call::ExchangeRateOracle(OracleCall::set_exchange_rate(1))
            .dispatch(origin_of(account_of(BOB))));

        set_default_thresholds();

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, polkabtc, vault_btc_address);

        // Alice requests to redeem polkabtc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polkabtc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // send the btc from the vault to the user
        let (tx_id, tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(user_btc_address, polkabtc, redeem_id);

        assert_ok!(Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            tx_id,
            tx_block_height,
            merkle_proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(vault))));
    });
}
