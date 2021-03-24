mod mock;

use frame_support::assert_err;
use mock::{issue_testing_utils::*, *};

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        UserData::force_to(USER, default_user_state());
        execute()
    })
}

fn test_with_initialized_vault<R>(execute: impl FnOnce() -> R) -> R {
    test_with(|| {
        CoreVaultData::force_to(VAULT, default_vault_state());
        execute()
    })
}
#[test]
fn integration_test_issue_should_fail_if_not_running() {
    test_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, account_of(BOB), 0)).dispatch(origin_of(account_of(ALICE))),
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
fn integration_test_issue_polka_btc_execute_succeeds() {
    test_with(|| {
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(VAULT)))
        );
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(vault_proof_submitter)))
        );

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(VAULT),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(USER))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) = generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
            .dispatch(origin_of(account_of(vault_proof_submitter))));
    });
}

#[test]
fn integration_test_issue_polka_btc_execute_bookkeeping() {
    test_with_initialized_vault(|| {
        let requested_btc = 1000;
        let (issue_id, issue) = request_issue(requested_btc);
        execute_issue(issue_id);

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                user.free_tokens += requested_btc;
                fee_pool.tokens += issue.fee;
                vault.issued += issue.fee + requested_btc;
            })
        );
    });
}

#[test]
fn integration_test_withdraw_after_request_issue() {
    test_with(|| {
        let vault = BOB;
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(vault)))
        );
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(vault_proof_submitter)))
        );

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

#[test]
fn integration_test_issue_overpayment() {
    test_with_initialized_vault(|| {
        let requested_btc = 1000;
        let (issue_id, issue) = request_issue(requested_btc);
        let sent_btc = (issue.amount + issue.fee) * 2;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .assert_execute();

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                user.free_tokens += 2 * requested_btc;
                fee_pool.tokens += 2 * issue.fee;
                vault.issued += 2 * (issue.fee + requested_btc);
            })
        );
    });
}

#[test]
/// overpay by a factor of 4
fn integration_test_issue_refund() {
    test_with_initialized_vault(|| {
        let requested_btc = 1000;

        // make sure we don't have enough collateral to fulfil the overpayment
        let current_minimum_collateral =
            VaultRegistryModule::get_required_collateral_for_vault(account_of(VAULT)).unwrap();
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: current_minimum_collateral + requested_btc * 2,
                ..CoreVaultData::vault(VAULT)
            },
        );
        let initial_state = ParachainState::get();

        let (issue_id, issue) = request_issue(requested_btc);
        let sent_btc = (issue.amount + issue.fee) * 4;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .assert_execute();

        // not enough collateral to back sent amount, so it's as if the user sent the correct amount
        let post_redeem_state = ParachainState::get();
        assert_eq!(
            post_redeem_state,
            initial_state.with_changes(|user, vault, _, fee_pool| {
                user.free_tokens += requested_btc;
                fee_pool.tokens += issue.fee;
                vault.issued += issue.fee + requested_btc;
            })
        );

        // perform the refund
        execute_refund(VAULT);

        assert_eq!(
            ParachainState::get(),
            post_redeem_state.with_changes(|_user, vault, _, _fee_pool| {
                vault.free_tokens += issue.fee * 3;
                vault.issued += issue.fee * 3;
            })
        );
    });
}

#[test]
fn integration_test_issue_underpayment_succeeds() {
    test_with_initialized_vault(|| {
        let requested_btc = 4000;
        let (issue_id, issue) = request_issue(requested_btc);
        let sent_btc = (issue.amount + issue.fee) / 4;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .with_submitter(USER, false)
            .assert_execute();

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                // user loses 75% of griefing collateral for having only fulfilled 25%
                let slashed_griefing_collateral = (issue.griefing_collateral * 3) / 4;
                user.free_balance -= slashed_griefing_collateral;
                fee_pool.balance += slashed_griefing_collateral;

                // token updating as if only 25% was requested
                user.free_tokens += requested_btc / 4;
                fee_pool.tokens += issue.fee / 4;
                vault.issued += (issue.fee + requested_btc) / 4;
            })
        );
    });
}

#[test]
fn integration_test_issue_underpayment_executed_by_third_party_fails() {
    test_with(|| {
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
    test_with_initialized_vault(|| {
        // random non-zero starting state
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(VAULT))),
            IssueError::CommitPeriodExpired
        );

        // bob cancels issue request
        assert_ok!(Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(VAULT))));

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, _vault, _, fee_pool| {
                user.free_balance -= issue.griefing_collateral;
                fee_pool.balance += issue.griefing_collateral;
            })
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_cancel_liquidated() {
    test_with_initialized_vault(|| {
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(VAULT))),
            IssueError::CommitPeriodExpired
        );

        drop_exchange_rate_and_liquidate(VAULT);
        let post_liquidation_status = ParachainState::get();

        // bob cancels issue request
        assert_ok!(Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(VAULT))));

        assert_eq!(
            ParachainState::get(),
            post_liquidation_status.with_changes(|user, _vault, liquidation_vault, _fee_pool| {
                // griefing collateral released instead of slashed
                user.locked_balance -= issue.griefing_collateral;
                user.free_balance += issue.griefing_collateral;

                liquidation_vault.to_be_issued -= issue.amount + issue.fee;
            })
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_execute_liquidated() {
    test_with_initialized_vault(|| {
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        drop_exchange_rate_and_liquidate(VAULT);
        let post_liquidation_status = ParachainState::get();

        execute_issue(issue_id);

        assert_eq!(
            ParachainState::get(),
            post_liquidation_status.with_changes(|user, _vault, liquidation_vault, fee_pool| {
                user.free_tokens += issue.amount;
                fee_pool.tokens += issue.fee;

                user.free_balance += issue.griefing_collateral;
                user.locked_balance -= issue.griefing_collateral;

                liquidation_vault.to_be_issued -= issue.amount + issue.fee;
                liquidation_vault.issued += issue.amount + issue.fee;
            })
        );
    });
}
