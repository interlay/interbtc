mod mock;

use mock::*;

use primitive_types::H256;

type RedeemCall = redeem::Call<Runtime>;
type RedeemModule = redeem::Module<Runtime>;
type RedeemEvent = redeem::Event<Runtime>;
type RedeemError = redeem::Error<Runtime>;

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

        assert_noop!(
            Call::Redeem(RedeemCall::request_redeem(
                1000,
                BtcAddress::P2PKH(H160([0u8; 20])),
                account_of(BOB),
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_redeem_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let collateral_vault = 1_000_000;
        let total_polka_btc = 1_000_000;
        let polka_btc = 1_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_rational(1, 100_000).unwrap()
        ));

        set_default_thresholds();

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, total_polka_btc);

        // alice requests to redeem polka_btc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polka_btc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();

        // send the btc from the vault to the user
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(user_btc_address, polka_btc, Some(redeem_id));

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        assert_ok!(Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            tx_id,
            merkle_proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(vault))));

        // TODO: check redeem rewards update
    });
}
fn setup_cancelable_redeem(user: [u8; 32], vault: [u8; 32], polka_btc: u128) -> H256 {
    let collateral_vault = 100500 * 2;
    let total_polka_btc = 100_000;

    let user_btc_address = BtcAddress::P2PKH(H160([2u8; 20]));

    SystemModule::set_block_number(1);

    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::checked_from_rational(1, 100_000).unwrap()
    ));

    set_default_thresholds();

    // create tokens for the vault and user
    force_issue_tokens(user, vault, collateral_vault, total_polka_btc);

    // alice requests to redeem polka_btc from Bob
    assert_ok!(Call::Redeem(RedeemCall::request_redeem(
        polka_btc,
        user_btc_address,
        account_of(vault)
    ))
    .dispatch(origin_of(account_of(user))));

    // assert that request happened and extract the id
    let redeem_id = assert_redeem_request_event();

    // expire request without transferring btc
    SystemModule::set_block_number(RedeemModule::redeem_period() + 1 + 1);

    // bob cannot execute past expiry
    assert_noop!(
        Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            H256Le::from_bytes_le(&[0; 32]),
            vec![],
            vec![],
        ))
        .dispatch(origin_of(account_of(vault))),
        RedeemError::CommitPeriodExpired,
    );

    redeem_id
}
#[test]
fn integration_test_redeem_polka_btc_cancel_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 1000;

        let initial_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let amount_dot = ExchangeRateOracleModule::btc_to_dots(amount_btc).unwrap();
        let punishment_fee = FeeModule::get_punishment_fee(amount_dot).unwrap();

        let redeem_id = setup_cancelable_redeem(user, vault, amount_btc);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_rational(1, 100_000).unwrap()
        ));

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(account_of(vault), sla_score_before);

        // alice cancels redeem request and chooses to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true))
            .dispatch(origin_of(account_of(user))));

        // balance should have increased by punishment_fee compared to the pre-request balance
        assert_eq!(
            CollateralModule::get_balance_from_account(&account_of(user)),
            initial_balance + punishment_fee
        );

        // bob's SLA is reduced by redeem failure amount
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change()
        );
    });
}

#[test]
fn integration_test_redeem_polka_btc_cancel_no_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 1000;

        let amount_dot = ExchangeRateOracleModule::btc_to_dots(amount_btc).unwrap();
        let punishment_fee = FeeModule::get_punishment_fee(amount_dot).unwrap();

        let redeem_id = setup_cancelable_redeem(user, vault, amount_btc);

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(account_of(vault), sla_score_before);

        let pre_cancel_balance = CollateralModule::get_balance_from_account(&account_of(user));

        // alice cancels redeem request, but does not reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, false))
            .dispatch(origin_of(account_of(user))));

        // balance should have increased by punishment_fee compared to the balance pre-cancellation
        assert_eq!(
            CollateralModule::get_balance_from_account(&account_of(user)),
            pre_cancel_balance + punishment_fee
        );

        // bob's SLA is reduced by redeem failure amount
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change()
        );
    });
}
