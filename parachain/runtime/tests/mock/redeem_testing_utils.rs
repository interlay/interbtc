use crate::*;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const USER_BTC_ADDRESS: BtcAddress = BtcAddress::P2PKH(H160([2u8; 20]));

pub fn setup_cancelable_redeem(user: [u8; 32], vault: [u8; 32], collateral: u128, polka_btc: u128) -> H256 {
    let redeem_id = setup_redeem(polka_btc, user, vault, collateral);

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

pub fn setup_redeem(polka_btc: u128, user: [u8; 32], vault: [u8; 32], collateral: u128) -> H256 {
    SystemModule::set_block_number(1);

    set_default_thresholds();

    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));

    let fee = FeeModule::get_redeem_fee(polka_btc).unwrap();

    // burn surplus free balance to make checking easier
    CollateralModule::transfer(
        account_of(vault),
        account_of(FAUCET),
        CollateralModule::get_balance_from_account(&account_of(vault)) - collateral,
    )
    .unwrap();

    // create tokens for the vault and user
    force_issue_tokens(user, vault, collateral, polka_btc - fee);

    // mint tokens to the user such that he can afford the fee
    TreasuryModule::mint(user.into(), fee);

    // alice requests to redeem polka_btc from Bob
    assert_ok!(Call::Redeem(RedeemCall::request_redeem(
        polka_btc,
        USER_BTC_ADDRESS,
        account_of(vault)
    ))
    .dispatch(origin_of(account_of(user))));

    // assert that request happened and extract the id
    assert_redeem_request_event()
}

// asserts redeem event happen and extracts its id for further testing
pub fn assert_redeem_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

pub fn execute_redeem(polka_btc: u128, redeem_id: H256) {
    // send the btc from the vault to the user
    let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(USER_BTC_ADDRESS, polka_btc, Some(redeem_id));

    SystemModule::set_block_number(1 + CONFIRMATIONS);

    assert_ok!(
        Call::Redeem(RedeemCall::execute_redeem(redeem_id, tx_id, merkle_proof, raw_tx))
            .dispatch(origin_of(account_of(VAULT)))
    );
}

pub fn cancel_redeem(redeem_id: H256, redeemer: [u8; 32], reimburse: bool) {
    assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, reimburse)).dispatch(origin_of(account_of(redeemer))));
}
