mod mock;

use frame_support::assert_err;
use mock::{issue_testing_utils::*, reward_testing_utils::vault_rewards, *};

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
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

macro_rules! signed_fixed_point {
    ($amount:expr) => {
        sp_arithmetic::FixedI128::checked_from_integer($amount).unwrap()
    };
}

mod expiry_test {
    use super::*;

    fn set_issue_period(period: u32) {
        assert_ok!(Call::Issue(IssueCall::set_issue_period(period)).dispatch(root()));
    }

    fn execute_issue(issue_id: H256) -> DispatchResultWithPostInfo {
        ExecuteIssueBuilder::new(issue_id).execute()
    }

    fn cancel_issue(issue_id: H256) -> DispatchResultWithPostInfo {
        Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(USER)))
    }

    #[test]
    fn integration_test_issue_expiry_only_parachain_blocks_expired() {
        test_with(|| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(4_000);
            mine_blocks(1);

            // not expired until both parachain block and parachain block expired
            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_only_bitcoin_blocks_expired() {
        test_with(|| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(4_000);
            SecurityPallet::set_active_block_number(750);
            mine_blocks(20);

            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_no_period_change_pre_expiry() {
        test_with(|| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(4_000);
            SecurityPallet::set_active_block_number(750);
            mine_blocks(7);

            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_no_period_change_post_expiry() {
        test_with(|| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(4_000);
            SecurityPallet::set_active_block_number(1100);
            mine_blocks(11);

            assert_noop!(execute_issue(issue_id), IssueError::CommitPeriodExpired);
            assert_ok!(cancel_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_with_period_decrease() {
        test_with(|| {
            set_issue_period(2000);
            let (issue_id, _) = request_issue(4_000);
            SecurityPallet::set_active_block_number(1100);
            mine_blocks(11);
            set_issue_period(1000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_with_period_increase() {
        test_with(|| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(4_000);
            SecurityPallet::set_active_block_number(1100);
            mine_blocks(11);
            set_issue_period(2000);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }
}

#[test]
fn integration_test_issue_with_parachain_shutdown_fails() {
    test_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, account_of(BOB), 0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );

        assert_noop!(
            Call::Issue(IssueCall::cancel_issue(H256([0; 32]),)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );

        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                Default::default(),
                Default::default(),
                Default::default()
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );

        assert_noop!(
            Call::Refund(RefundCall::execute_refund(
                Default::default(),
                Default::default(),
                Default::default()
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );
    });
}

mod request_issue_tests {
    use super::*;

    #[test]
    fn integration_test_request_issue_at_capacity_succeeds() {
        test_with_initialized_vault(|| {
            let amount = VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
            let (issue_id, _) = request_issue(amount);
            execute_issue(issue_id);
        });
    }

    #[test]
    fn integration_test_request_issue_above_capacity_fails() {
        test_with_initialized_vault(|| {
            let amount = 1 + VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
            assert_noop!(
                Call::Issue(IssueCall::request_issue(
                    amount,
                    account_of(VAULT),
                    DEFAULT_GRIEFING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                VaultRegistryError::ExceedingVaultLimit
            );
        });
    }

    #[test]
    fn integration_test_request_with_griefing_collateral_at_minimum_succeeds() {
        test_with_initialized_vault(|| {
            let amount = 10_000;
            let amount_in_collateral = ExchangeRateOraclePallet::wrapped_to_collateral(amount).unwrap();
            let griefing_collateral = FeePallet::get_issue_griefing_collateral(amount_in_collateral).unwrap();
            assert_ok!(
                Call::Issue(IssueCall::request_issue(amount, account_of(VAULT), griefing_collateral))
                    .dispatch(origin_of(account_of(USER)))
            );
        });
    }

    #[test]
    fn integration_test_request_with_griefing_collateral_below_minimum_fails() {
        test_with_initialized_vault(|| {
            assert_ok!(
                Call::VaultRegistry(VaultRegistryCall::accept_new_issues(false)).dispatch(origin_of(account_of(VAULT)))
            );
            assert_noop!(
                Call::Issue(IssueCall::request_issue(
                    1000,
                    account_of(VAULT),
                    DEFAULT_GRIEFING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                IssueError::VaultNotAcceptingNewIssues
            );
        });
    }

    #[test]
    fn integration_test_request_not_accepting_new_issues_fails() {
        test_with_initialized_vault(|| {
            let amount = 10_000;
            let amount_in_collateral = ExchangeRateOraclePallet::wrapped_to_collateral(amount).unwrap();
            let griefing_collateral = FeePallet::get_issue_griefing_collateral(amount_in_collateral).unwrap() - 1;
            assert_noop!(
                Call::Issue(IssueCall::request_issue(amount, account_of(VAULT), griefing_collateral))
                    .dispatch(origin_of(account_of(USER))),
                IssueError::InsufficientCollateral
            );
        });
    }
}

#[test]
fn integration_test_issue_fails_with_uninitialized_relay() {
    ExtBuilder::build().execute_without_relay_init(|| {
        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, Default::default(), 0)).dispatch(origin_of(account_of(USER))),
            IssueError::WaitingForRelayerInitialization
        );
    });
    ExtBuilder::build().execute_without_relay_init(|| {
        // calls BTCRelay::initialize, but with insufficient confirmations
        let _ = TransactionGenerator::new().with_confirmations(3).mine();

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, Default::default(), 0)).dispatch(origin_of(account_of(USER))),
            IssueError::WaitingForRelayerInitialization
        );
    });
}

#[test]
fn integration_test_issue_wrapped_execute_succeeds() {
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

        // alice requests wrapped by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(VAULT),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(USER))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (_tx_id, _height, proof, raw_tx) = generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(Call::Issue(IssueCall::execute_issue(issue_id, proof, raw_tx))
            .dispatch(origin_of(account_of(vault_proof_submitter))));
    });
}

#[test]
fn integration_test_issue_wrapped_execute_bookkeeping() {
    test_with_initialized_vault(|| {
        let requested_btc = 1000;
        let (issue_id, issue) = request_issue(requested_btc);

        assert_eq!(issue.fee + issue.amount, requested_btc);

        execute_issue(issue_id);

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                user.free_tokens += issue.amount;
                fee_pool.vault_rewards += vault_rewards(issue.fee);
                vault.issued += issue.fee + issue.amount;
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

        // alice requests wrapped by locking btc with bob
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
                user.free_tokens += 2 * issue.amount;
                fee_pool.vault_rewards += 2 * vault_rewards(issue.fee);
                vault.issued += sent_btc;
            })
        );

        assert_issue_amount_change_event(issue_id, 2 * issue.amount, 2 * issue.fee, 0);
    });
}

#[test]
/// overpay by a factor of 4
fn integration_test_issue_refund() {
    test_with_initialized_vault(|| {
        let requested_btc = 1000;

        // make sure we don't have enough collateral to fulfil the overpayment
        let current_minimum_collateral =
            VaultRegistryPallet::get_required_collateral_for_vault(account_of(VAULT)).unwrap();
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
                user.free_tokens += issue.amount;
                fee_pool.vault_rewards += vault_rewards(issue.fee);
                vault.issued += issue.fee + issue.amount;
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

mod execute_refund_payment_limits {
    use super::*;

    fn setup_refund() -> (H256, u128) {
        let requested_btc = 1000;

        // make sure we don't have enough collateral to fulfil the overpayment
        let current_minimum_collateral =
            VaultRegistryPallet::get_required_collateral_for_vault(account_of(VAULT)).unwrap();
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: current_minimum_collateral + requested_btc * 2,
                ..CoreVaultData::vault(VAULT)
            },
        );

        let (issue_id, issue) = request_issue(requested_btc);
        let sent_btc = (issue.amount + issue.fee) * 4;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .assert_execute();

        let refund_id = assert_refund_request_event();
        let refund = RefundPallet::get_open_refund_request_from_id(&refund_id).unwrap();

        (refund_id, refund.amount_wrapped)
    }

    #[test]
    fn integration_test_execute_refund_with_exact_amount_succeeds() {
        test_with_initialized_vault(|| {
            let (_refund_id, amount) = setup_refund();
            assert_ok!(execute_refund_with_amount(VAULT, amount));
        });
    }
    #[test]
    fn integration_test_execute_refund_with_overpayment_fails() {
        test_with_initialized_vault(|| {
            let (_refund_id, amount) = setup_refund();
            assert_err!(
                execute_refund_with_amount(VAULT, amount + 1),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
    #[test]
    fn integration_test_execute_refund_with_underpayment_fails() {
        test_with_initialized_vault(|| {
            let (_refund_id, amount) = setup_refund();
            assert_err!(
                execute_refund_with_amount(VAULT, amount - 1),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

#[test]
fn integration_test_issue_underpayment_succeeds() {
    test_with_initialized_vault(|| {
        let requested_btc = 4000;
        let (issue_id, issue) = request_issue(requested_btc);
        let sent_btc = (issue.amount + issue.fee) / 4;

        // need stake for rewards to deposit
        assert_ok!(VaultRewardsPallet::deposit_stake(
            DOT,
            &account_of(VAULT),
            signed_fixed_point!(1)
        ));

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .with_submitter(USER, false)
            .assert_execute();

        let slashed_griefing_collateral = (issue.griefing_collateral * 3) / 4;

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                // user loses 75% of griefing collateral for having only fulfilled 25%
                user.free_balance -= slashed_griefing_collateral;
                vault.free_balance += slashed_griefing_collateral;

                // token updating as if only 25% was requested
                user.free_tokens += issue.amount / 4;
                fee_pool.vault_rewards += vault_rewards(issue.fee / 4);
                vault.issued += (issue.fee + issue.amount) / 4;
            })
        );

        assert_issue_amount_change_event(issue_id, issue.amount / 4, issue.fee / 4, slashed_griefing_collateral);
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
fn integration_test_issue_wrapped_cancel() {
    test_with_initialized_vault(|| {
        // random non-zero starting state
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
        mine_blocks((IssuePallet::issue_period() + 99) / 100 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(issue_id, vec![], vec![])).dispatch(origin_of(account_of(VAULT))),
            IssueError::CommitPeriodExpired
        );

        // need stake for rewards to deposit
        assert_ok!(VaultRewardsPallet::deposit_stake(
            DOT,
            &account_of(VAULT),
            signed_fixed_point!(1)
        ));

        // bob cancels issue request
        assert_ok!(Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(VAULT))));

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, _| {
                user.free_balance -= issue.griefing_collateral;
                vault.free_balance += issue.griefing_collateral;
            })
        );
    });
}

#[test]
fn integration_test_issue_wrapped_cancel_liquidated() {
    test_with_initialized_vault(|| {
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
        mine_blocks((IssuePallet::issue_period() + 99) / 100 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(issue_id, vec![], vec![])).dispatch(origin_of(account_of(VAULT))),
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
fn integration_test_issue_wrapped_execute_liquidated() {
    test_with_initialized_vault(|| {
        let (issue_id, issue) = RequestIssueBuilder::new(10_000).request();

        drop_exchange_rate_and_liquidate(VAULT);
        let post_liquidation_status = ParachainState::get();

        execute_issue(issue_id);

        assert_eq!(
            ParachainState::get(),
            post_liquidation_status.with_changes(|user, _vault, liquidation_vault, fee_pool| {
                user.free_tokens += issue.amount;
                fee_pool.vault_rewards += vault_rewards(issue.fee);

                user.free_balance += issue.griefing_collateral;
                user.locked_balance -= issue.griefing_collateral;

                liquidation_vault.to_be_issued -= issue.amount + issue.fee;
                liquidation_vault.issued += issue.amount + issue.fee;
            })
        );
    });
}

#[test]
fn integration_test_issue_with_unrelated_rawtx_and_txid_fails() {
    test_with_initialized_vault(|| {
        let (issue_id, issue) = request_issue(1000);
        let (_tx_id, _height, proof, raw_tx, mut transaction) = TransactionGenerator::new()
            .with_address(issue.btc_address)
            .with_amount(1)
            .with_op_return(None)
            .mine();

        SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

        // fail due to insufficient amount
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(issue_id, proof.clone(), raw_tx))
                .dispatch(origin_of(account_of(CAROL))),
            IssueError::InvalidExecutor
        );

        // increase the amount in the raw_tx, but not in the blockchain. This should definitely fail
        transaction.outputs[0].value = 1000;
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(issue_id, proof, transaction.format_with(true)))
                .dispatch(origin_of(account_of(CAROL))),
            BTCRelayError::InvalidTxid
        );
    })
}
