mod mock;

use mock::redeem_testing_utils::*;
use mock::*;

use vault_registry::types::RichVault;
use vault_registry::types::UpdatableVault;

#[test]
fn integration_test_redeem_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

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
        let collateral_vault = 1_000_000_000_000;
        let polka_btc = 1_000_000_000_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_rational(1, 100_000).unwrap()
        ));

        set_default_thresholds();

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, polka_btc);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let initial_btc_issuance = TreasuryModule::get_total_supply();
        assert_eq!(polka_btc, initial_btc_issuance);

        // alice requests to redeem polka_btc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polka_btc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();

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

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let final_btc_issuance = TreasuryModule::get_total_supply();

        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc burned from user, including fee
        assert_eq!(final_btc_balance, initial_btc_balance - polka_btc);
        // polka_btc burned from issuance
        assert_eq!(final_btc_issuance, initial_btc_issuance - redeem.amount_btc);

        assert_eq!(FeeModule::epoch_rewards_polka_btc(), redeem.fee);
    });
}

#[test]
fn integration_test_premium_redeem_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let polka_btc = 1_000_000_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        set_default_thresholds();

        let collateral_vault = required_collateral_for_issue(polka_btc);

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, polka_btc);

        // suddenly require twice as much DOT; we are definitely below premium redeem threshold now
        // (also below liquidation threshold, but as long as we don't call liquidate that's ok)
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let initial_vault_collateral =
            CollateralModule::get_collateral_from_account(&account_of(vault));
        let initial_btc_issuance = TreasuryModule::get_total_supply();
        assert_eq!(polka_btc, initial_btc_issuance);

        // alice requests to redeem polka_btc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polka_btc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();

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

        assert_eq!(FeeModule::epoch_rewards_polka_btc(), redeem.fee);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let final_btc_issuance = TreasuryModule::get_total_supply();

        // user should have received some premium (DOT)
        assert!(final_dot_balance > initial_dot_balance);

        // it should be a zero-sum game; the user's gain is equal to the vault's loss
        assert_eq!(
            initial_vault_collateral + initial_dot_balance,
            CollateralModule::get_collateral_from_account(&account_of(vault)) + final_dot_balance
        );

        // polka_btc burned from user, including fee
        assert_eq!(final_btc_balance, initial_btc_balance - polka_btc);
        // polka_btc burned from issuance
        assert_eq!(final_btc_issuance, initial_btc_issuance - redeem.amount_btc);

        // TODO: check redeem rewards update
    });
}

#[test]
fn integration_test_redeem_polka_btc_liquidation_redeem() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        set_default_thresholds();
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let issued = 400;
        let to_be_issued = 100;
        let to_be_redeemed = 50;

        UserData::force_to(
            USER,
            UserData {
                free_tokens: 1000,
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued,
                to_be_issued,
                to_be_redeemed,
                backing_collateral: 10_000,
                ..Default::default()
            },
        );

        // create tokens for the vault and user
        drop_exchange_rate_and_liquidate(VAULT);

        let slashed_collateral = 10_000 - (10000 * to_be_redeemed) / (issued + to_be_issued);

        assert_eq!(
            CoreVaultData::liquidation_vault(),
            CoreVaultData {
                issued,
                to_be_issued,
                to_be_redeemed,
                backing_collateral: slashed_collateral,
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            },
        );

        assert_noop!(
            Call::Redeem(RedeemCall::liquidation_redeem(351)).dispatch(origin_of(account_of(USER))),
            VaultRegistryError::InsufficientTokensCommitted
        );

        assert_ok!(
            Call::Redeem(RedeemCall::liquidation_redeem(325)).dispatch(origin_of(account_of(USER)))
        );

        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: (slashed_collateral * 325) / (issued + to_be_issued),
                free_tokens: 1000 - 325,
                ..Default::default()
            },
        );
    });
}

#[test]
fn integration_test_redeem_polka_btc_cancel_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 100000;

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(10).unwrap() // 10 planck/satoshi
        ));

        let initial_balance_dot = CollateralModule::get_balance_from_account(&account_of(user));

        let redeem_id = setup_cancelable_redeem(user, vault, 100000000, amount_btc);
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_dot =
            ExchangeRateOracleModule::btc_to_dots(redeem.amount_btc).unwrap();

        let punishment_fee = FeeModule::get_punishment_fee(amount_without_fee_dot).unwrap();
        assert!(punishment_fee > 0);

        // get initial balance - the setup call above will have minted and locked polkabtc
        let initial_balance_btc = TreasuryModule::get_balance_from_account(account_of(user))
            + TreasuryModule::get_locked_balance_from_account(account_of(user));

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(&account_of(vault), sla_score_before);

        // alice cancels redeem request and chooses to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true))
            .dispatch(origin_of(account_of(user))));

        let new_balance = CollateralModule::get_balance_from_account(&account_of(user));

        // balance should have increased by punishment_fee plus amount_without_fee_dot
        assert_eq!(
            new_balance,
            initial_balance_dot + amount_without_fee_dot + punishment_fee
        );

        // user gets fee back, but loses the rest of the requested btc
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(user)),
            initial_balance_btc - (amount_btc - redeem.fee)
        );

        // vault's SLA is reduced by redeem failure amount
        let expected_sla = FixedI128::max(
            FixedI128::zero(),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change(),
        );
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);
        assert!(FeeModule::epoch_rewards_dot() > 0);
    });
}

#[test]
fn integration_test_redeem_polka_btc_cancel_no_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 1000;

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(10).unwrap() // 10 planck/satoshi
        ));

        let initial_balance_dot = CollateralModule::get_balance_from_account(&account_of(user));

        let redeem_id = setup_cancelable_redeem(user, vault, 100000000, amount_btc);
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let punishment_fee = FeeModule::get_punishment_fee(
            ExchangeRateOracleModule::btc_to_dots(redeem.amount_btc).unwrap(),
        )
        .unwrap();
        assert!(punishment_fee > 0);

        // get initial balance - the setup call above will have minted and locked polkabtc
        let initial_balance_btc = TreasuryModule::get_balance_from_account(account_of(user))
            + TreasuryModule::get_locked_balance_from_account(account_of(user));

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(&account_of(vault), sla_score_before);

        // alice cancels redeem request, but does not reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, false))
            .dispatch(origin_of(account_of(user))));

        // dot-balance should have increased by punishment_fee
        assert_eq!(
            CollateralModule::get_balance_from_account(&account_of(user)),
            initial_balance_dot + punishment_fee
        );

        // polkabtc balance should not have changed
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(user)),
            initial_balance_btc
        );

        // vault's SLA is reduced by redeem failure amount
        let expected_sla = FixedI128::max(
            FixedI128::zero(),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change(),
        );
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);
        assert!(FeeModule::epoch_rewards_dot() > 0);
    });
}

fn test_cancel_liquidated(reimburse: bool) {
    let user = ALICE;
    let vault = BOB;
    let amount_btc = 100000;

    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::checked_from_integer(10).unwrap() // 10 planck/satoshi
    ));

    let initial_balance_dot = CollateralModule::get_balance_from_account(&account_of(user));

    let redeem_id = setup_cancelable_redeem(user, vault, 1999500000, amount_btc);
    let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
    let amount_without_fee_dot = ExchangeRateOracleModule::btc_to_dots(redeem.amount_btc).unwrap();

    let punishment_fee = FeeModule::get_punishment_fee(amount_without_fee_dot).unwrap();
    assert!(punishment_fee > 0);

    // get initial balance - the setup call above will have minted and locked polkabtc
    let initial_balance_btc = TreasuryModule::get_balance_from_account(account_of(user))
        + TreasuryModule::get_locked_balance_from_account(account_of(user));
    let initial_collateral = CollateralModule::get_collateral_from_account(&account_of(vault));
    let sla_score_before = FixedI128::from(60);
    SlaModule::set_vault_sla(&account_of(vault), sla_score_before);

    let mut rich_vault: RichVault<Runtime> =
        VaultRegistryModule::get_vault_from_id(&account_of(vault))
            .unwrap()
            .into();
    assert_ok!(rich_vault.force_increase_to_be_redeemed(redeem.amount_btc));
    assert_ok!(rich_vault.force_issue_tokens(redeem.amount_btc));

    let vault_data_before = VaultRegistryModule::get_vault_from_id(&account_of(vault)).unwrap();
    assert_eq!(
        vault_data_before.to_be_redeemed_tokens,
        2 * redeem.amount_btc
    ); // sanity check

    drop_exchange_rate_and_liquidate(vault);

    assert_ok!(
        Call::Redeem(RedeemCall::cancel_redeem(redeem_id, reimburse))
            .dispatch(origin_of(account_of(user)))
    );

    // Check vault data
    let vault_data_after = VaultRegistryModule::get_vault_from_id(&account_of(vault)).unwrap();
    assert_eq!(vault_data_after.issued_tokens, 0);
    assert_eq!(vault_data_after.to_be_issued_tokens, 0);
    // vault started with (2*redeem.amount_btc) - it should now have redeem.amount_btc left
    assert_eq!(vault_data_after.to_be_redeemed_tokens, redeem.amount_btc);
    assert_eq!(
        CollateralModule::get_collateral_from_account(&account_of(vault)),
        (redeem.amount_btc * initial_collateral) / vault_data_before.issued_tokens
    );
    assert_eq!(
        CollateralModule::get_collateral_from_account(&account_of(vault)),
        (redeem.amount_btc * initial_collateral) / vault_data_before.issued_tokens
    );

    // Check user data
    // user balance should have remained the same -- no fees or punishments reiumbursed
    assert_eq!(
        CollateralModule::get_balance_from_account(&account_of(user)),
        initial_balance_dot
    );
    // user keeps all polkabtc
    assert_eq!(
        TreasuryModule::get_balance_from_account(account_of(user)),
        initial_balance_btc
    );
}
#[test]
fn integration_test_redeem_polka_btc_cancel_liquidated_reimburse() {
    ExtBuilder::build().execute_with(|| test_cancel_liquidated(true));
}

#[test]
fn integration_test_redeem_polka_btc_cancel_liquidated_no_reimburse() {
    ExtBuilder::build().execute_with(|| test_cancel_liquidated(false));
}

#[test]
fn integration_test_redeem_polka_btc_execute_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let polka_btc = 1_000;
        let fee = FeeModule::get_redeem_fee(polka_btc).unwrap();
        let collateral_vault = 1_000_000;
        let amount_without_fee = polka_btc - fee;

        let redeem_id = setup_redeem(polka_btc, USER, VAULT, collateral_vault);
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: polka_btc + 1234,
                free_tokens: 50,
            },
        );

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: amount_without_fee, // assuming fee
                to_be_redeemed: amount_without_fee,
                backing_collateral: collateral_vault,
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: amount_without_fee * 4,
                to_be_redeemed: amount_without_fee * 4,
                backing_collateral: collateral_vault,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);

        execute_redeem(polka_btc, redeem_id);

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_redeemed: amount_without_fee * 3,
                backing_collateral: (collateral_vault * 3) / 4,
                free_balance: collateral_vault / 4,
                ..Default::default()
            },
        );

        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: 1234, // most important: check that polka_btc has been burned
                free_tokens: 50,
            },
        );

        assert_eq!(FeeModule::epoch_rewards_polka_btc(), redeem.fee);
    });
}

#[test]
fn integration_test_redeem_banning() {
    ExtBuilder::build().execute_with(|| {
        let new_vault = CAROL;

        let redeem_id = setup_cancelable_redeem(USER, VAULT, 10_000, 1_000);

        // make sure the vault & user have funds after the cancel_redeem
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: 1000000,
                backing_collateral: 10000000,
                free_balance: 100, // to be used for griefing collateral
                ..CoreVaultData::vault(VAULT)
            },
        );
        UserData::force_to(
            USER,
            UserData {
                free_balance: 1000000,
                free_tokens: 10000000,
                ..UserData::get(USER)
            },
        );
        CoreVaultData::force_to(
            new_vault,
            CoreVaultData {
                issued: 1000000,
                backing_collateral: 10000000,
                ..Default::default()
            },
        );

        // can still make a replace request now
        assert_ok!(Call::Replace(ReplaceCall::request_replace(100, 100))
            .dispatch(origin_of(account_of(VAULT))));
        let replace_id = SystemModule::events()
            .iter()
            .find_map(|r| match r.event {
                Event::replace(ReplaceEvent::RequestReplace(id, _, _, _)) => Some(id.clone()),
                _ => None,
            })
            .unwrap();

        // cancel the redeem, this should ban the vault
        cancel_redeem(redeem_id, USER, true);

        // can not redeem with vault while banned
        assert_noop!(
            Call::Redeem(RedeemCall::request_redeem(
                50,
                BtcAddress::P2PKH(H160([0u8; 20])),
                account_of(VAULT),
            ))
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::VaultBanned,
        );

        // can not issue with vault while banned
        assert_noop!(
            Call::Issue(IssueCall::request_issue(50, account_of(VAULT), 50))
                .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::VaultBanned,
        );

        // can not request replace while banned
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, 0))
                .dispatch(origin_of(account_of(VAULT))),
            VaultRegistryError::VaultBanned,
        );

        // can not accept replace of banned vault
        assert_noop!(
            Call::Replace(ReplaceCall::accept_replace(
                replace_id,
                1000,
                BtcAddress::default()
            ))
            .dispatch(origin_of(account_of(VAULT))),
            VaultRegistryError::VaultBanned,
        );

        // check that the ban is not permanent
        SystemModule::set_block_number(100000000);
        assert_ok!(
            Call::Issue(IssueCall::request_issue(50, account_of(VAULT), 50))
                .dispatch(origin_of(account_of(USER)))
        );
    })
}
