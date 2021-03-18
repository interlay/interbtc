mod mock;

use frame_support::assert_err;
use mock::issue_testing_utils::*;
use mock::*;

#[test]
fn integration_test_issue_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, account_of(BOB), 0))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );

        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                H256([0; 32]),
                H256Le::zero(),
                vec![0u8; 32],
                vec![0u8; 32]
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let user = ALICE;
        let vault = BOB;
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault_proof_submitter))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(
            Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(vault_proof_submitter)))
        );

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // check the sla increase
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(amount_btc, total_amount_btc).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            expected_sla_increase
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral reimbursed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc minted
        assert_eq!(final_btc_balance, initial_btc_balance + amount_btc);

        // vault should have 0 to-be-issued tokens
        assert_eq!(
            VaultRegistryModule::get_vault_from_id(&account_of(vault))
                .unwrap()
                .to_be_issued_tokens,
            0
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(vault))
        ))
        .dispatch(origin_of(account_of(vault))));
    });
}

#[test]
fn integration_test_withdraw_after_request_issue() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let vault = BOB;
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault_proof_submitter))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        // Should not be possible to request more, using the same collateral
        assert!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE)))
        .is_err());

        // should not be possible to withdraw the collateral now
        assert!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(collateral_vault))
                .dispatch(origin_of(account_of(vault)))
                .is_err()
        );
    });
}

/// Like integration_test_issue_polka_btc_execute, but here request only half of the amount - we
/// still transfer the same amount of bitcoin though. Check that it acts as if we requested the
/// full amount
#[test]
fn integration_test_issue_overpayment() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let user = ALICE;
        let vault = BOB;

        let amount_btc = 1000000;
        let overpayment_factor = 2;
        let requested_amount_btc = amount_btc / overpayment_factor;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            requested_amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;

        let fee_amount_btc = FeeModule::get_issue_fee(amount_btc).unwrap();
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(
            Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(user)))
        );

        // check the sla increase
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(amount_btc, total_amount_btc).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            expected_sla_increase
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral reimbursed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc minted
        assert_eq!(final_btc_balance, initial_btc_balance + amount_btc);

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(vault))
        ))
        .dispatch(origin_of(account_of(vault))));
    });
}

#[test]
fn integration_test_issue_refund() {
    ExtBuilder::build().execute_with(|| {
        let (issue_id, issue) = request_issue(1000);

        // verify that the vault has no spendable tokens at start
        assert_eq!(UserData::get(VAULT).free_tokens, 0);

        // make sure we don't have enough collateral to fulfil the overpayment
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: 2000,
                ..CoreVaultData::vault(VAULT)
            },
        );

        // overpay by a factor of 4
        ExecuteIssueBuilder::new(issue_id)
            .with_amount(4 * (issue.amount + issue.fee))
            .assert_execute();

        // perform the refund
        execute_refund(VAULT);

        // check that we the vault has issued the amount as usual, and 4 times the normal fee
        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: issue.amount + 4 * issue.fee,
                backing_collateral: 2000,
                ..Default::default()
            },
        );

        // check that fee was minted and is spendable by the vault
        // TODO: check that this is correct
        assert_eq!(UserData::get(VAULT).free_tokens, 3 * issue.fee);
    });
}

#[test]
fn integration_test_issue_underpayment_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let initial_user_balance = UserData::get(USER).free_balance;
        let (issue_id, issue) = request_issue(4_000);

        // only pay 25%
        ExecuteIssueBuilder::new(issue_id)
            .with_amount((issue.amount + issue.fee) / 4)
            .with_submitter(USER, false)
            .assert_execute();

        // check that we the vault has issued only 25%, and that it received 75% of griefing collateral
        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: (issue.amount + issue.fee) / 4,
                backing_collateral: DEFAULT_COLLATERAL + (issue.griefing_collateral * 3) / 4,
                ..Default::default()
            },
        );

        // check that the user lost 75% of griefing collateral, and received 25% of requested polkabtc
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: initial_user_balance - (issue.griefing_collateral * 3) / 4,
                free_tokens: issue.amount / 4,
                ..Default::default()
            }
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), issue.fee / 4);
    });
}

#[test]
fn integration_test_issue_underpayment_executed_by_third_party_fails() {
    ExtBuilder::build().execute_with(|| {
        let (issue_id, issue) = request_issue(4_000);

        // note: not doing assert_noop because the build does additional calls that change the storage
        assert_err!(
            ExecuteIssueBuilder::new(issue_id)
                .with_amount((issue.amount + issue.fee) / 4)
                .with_submitter(PROOF_SUBMITTER, true)
                .execute(),
            IssueError::InvalidExecutor
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_cancel() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 100000;
        let griefing_collateral = 100;
        let collateral_vault = 1000000;

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();

        // expire request without transferring btc
        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(vault))),
            IssueError::CommitPeriodExpired
        );

        // bob cancels issue request
        assert_ok!(
            Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault)))
        );

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral slashed
        assert_eq!(final_dot_balance, initial_dot_balance - griefing_collateral);

        // no polka_btc for alice
        assert_eq!(final_btc_balance, initial_btc_balance);
    });
}

#[test]
fn integration_test_issue_polka_btc_cancel_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 100000;
        let griefing_collateral = 100;
        let collateral_vault = 1000000;

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue = IssueModule::get_issue_request_from_id(&issue_id).unwrap();

        drop_exchange_rate_and_liquidate(vault);

        assert_eq!(
            VaultRegistryModule::get_liquidation_vault().to_be_issued_tokens,
            issue.amount + issue.fee
        );

        // expire request without transferring btc
        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(vault))),
            IssueError::CommitPeriodExpired
        );

        // bob cancels issue request
        assert_ok!(
            Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault)))
        );

        assert_eq!(
            VaultRegistryModule::get_liquidation_vault().to_be_issued_tokens,
            0
        );

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral is NOT slashed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // no polka_btc for alice
        assert_eq!(final_btc_balance, initial_btc_balance);
    });
}

#[test]
fn integration_test_issue_polka_btc_execute_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000;

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS,
            },
        );

        let (issue_id, issue) = request_issue(amount_btc);

        let fee_amount_btc = issue.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_issued: total_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);
        execute_issue(issue_id);

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // check that sla is zero for being liquidated
        assert_eq!(SlaModule::vault_sla(account_of(VAULT)), FixedI128::zero());

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        // vault should be empty
        assert_eq!(CoreVaultData::vault(VAULT), CoreVaultData::default());
        // liquidation vault took everything from vault
        assert_eq!(
            CoreVaultData::liquidation_vault(),
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                issued: amount_btc + fee_amount_btc,
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            }
        );
        // net effect is that user received free_tokens
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS + amount_btc,
            },
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT))
        ))
        .dispatch(origin_of(account_of(VAULT))));
        // should not have received fee
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(VAULT)),
            0
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_execute_not_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let vault_proof_submitter = CAROL;

        let amount_btc = 10_000;

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS,
            },
        );

        let (issue_id, issue) = request_issue(amount_btc);

        let fee_amount_btc = issue.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_issued: total_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );

        execute_issue(issue_id);

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: amount_btc + fee_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );
        // net effect is that user received free_tokens
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS + amount_btc,
            },
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT))
        ))
        .dispatch(origin_of(account_of(VAULT))));
        // check that a fee has been withdrawn
        assert!(TreasuryModule::get_balance_from_account(account_of(VAULT)) > 0);
    });
}
