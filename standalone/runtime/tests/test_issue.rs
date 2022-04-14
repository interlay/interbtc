mod mock;

use currency::Amount;
use frame_support::assert_err;
use mock::{assert_eq, issue_testing_utils::*, *};

fn test_with<R>(execute: impl Fn(VaultId) -> R) {
    let test_with = |currency_id, wrapped_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies() {
                assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            }
            if wrapped_id != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_id, FixedU128::one()));
            }
            UserData::force_to(USER, default_user_state());
            let vault_id = PrimitiveVaultId::new(account_of(VAULT), currency_id, wrapped_id);
            LiquidationVaultData::force_to(default_liquidation_vault_state(&vault_id.currencies));

            execute(vault_id)
        });
    };
    test_with(Token(DOT), Token(KBTC));
    test_with(Token(KSM), Token(IBTC));
    test_with(Token(DOT), Token(IBTC));
}

fn test_with_initialized_vault<R>(execute: impl Fn(VaultId) -> R) {
    test_with(|vault_id| {
        CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));

        // register a second vault with another currency id
        let mut other_vault = vault_id.clone();
        other_vault.currencies.collateral = if let Token(DOT) = vault_id.collateral_currency() {
            Token(KSM)
        } else {
            Token(DOT)
        };
        CoreVaultData::force_to(&other_vault, default_vault_state(&other_vault));

        execute(vault_id)
    })
}

macro_rules! signed_fixed_point {
    ($amount:expr) => {
        sp_arithmetic::FixedI128::checked_from_integer($amount).unwrap()
    };
}

mod expiry_test {
    use super::{assert_eq, *};

    fn set_issue_period(period: u32) {
        assert_ok!(Call::Issue(IssueCall::set_issue_period { period }).dispatch(root()));
    }

    fn execute_issue(issue_id: H256) -> DispatchResultWithPostInfo {
        ExecuteIssueBuilder::new(issue_id).execute()
    }

    fn cancel_issue(issue_id: H256) -> DispatchResultWithPostInfo {
        Call::Issue(IssueCall::cancel_issue { issue_id: issue_id }).dispatch(origin_of(account_of(USER)))
    }

    #[test]
    fn integration_test_issue_expiry_only_parachain_blocks_expired() {
        test_with(|vault_id| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
            mine_blocks(1);

            // not expired until both parachain block and parachain block expired
            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_only_bitcoin_blocks_expired() {
        test_with(|vault_id| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
            SecurityPallet::set_active_block_number(750);
            mine_blocks(20);

            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_no_period_change_pre_expiry() {
        test_with(|vault_id| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
            SecurityPallet::set_active_block_number(750);
            mine_blocks(7);

            assert_noop!(cancel_issue(issue_id), IssueError::TimeNotExpired);
            assert_ok!(execute_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_no_period_change_post_expiry() {
        test_with(|vault_id| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
            SecurityPallet::set_active_block_number(1100);
            mine_blocks(11);

            assert_noop!(execute_issue(issue_id), IssueError::CommitPeriodExpired);
            assert_ok!(cancel_issue(issue_id));
        });
    }

    #[test]
    fn integration_test_issue_expiry_with_period_decrease() {
        test_with(|vault_id| {
            set_issue_period(2000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
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
        test_with(|vault_id| {
            set_issue_period(1000);
            let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
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
    test_with(|_currency_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Refund(RefundCall::execute_refund {
                refund_id: Default::default(),
                merkle_proof: Default::default(),
                raw_tx: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered,
        );
    });
}

mod request_issue_tests {
    use super::{assert_eq, *};

    /// Request fails if parachain is shutdown
    #[test]
    fn integration_test_issue_request_precond_not_shutdown() {
        test_with(|vault_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: 0,
                    vault_id: vault_id,
                })
                .dispatch(origin_of(account_of(ALICE))),
                SystemError::CallFiltered,
            );
        });
    }

    /// Request fails if relay is not initialized
    #[test]
    fn integration_test_issue_request_precond_relay_initialized() {
        ExtBuilder::build().execute_without_relay_init(|| {
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: 0,
                    vault_id: dummy_vault_id_of(),
                })
                .dispatch(origin_of(account_of(USER))),
                IssueError::WaitingForRelayerInitialization
            );
        });
        ExtBuilder::build().execute_without_relay_init(|| {
            // calls BTCRelay::initialize, but with insufficient confirmations
            let _ = TransactionGenerator::new().with_confirmations(3).mine();

            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: 0,
                    vault_id: dummy_vault_id_of(),
                })
                .dispatch(origin_of(account_of(USER))),
                IssueError::WaitingForRelayerInitialization
            );
        });
    }

    /// Request fails if attempted with an account that is not a registered vault
    #[test]
    fn integration_test_issue_request_precond_vault_registered() {
        test_with(|vault_id| {
            //test_with ...out_initialized_vault
            let amount = 1_000;
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: amount,
                    vault_id: vault_id,
                })
                .dispatch(origin_of(account_of(USER))),
                VaultRegistryError::VaultNotFound
            );
        });
    }

    /// Request fails if vault is not actively accepting new issues
    #[test]
    fn integration_test_issue_request_precond_vault_active() {
        test_with_initialized_vault(|vault_id| {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::accept_new_issues {
                currency_pair: vault_id.currencies.clone(),
                accept_new_issues: false
            })
            .dispatch(origin_of(account_of(VAULT))));
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: 1000,
                    vault_id: vault_id.clone(),
                })
                .dispatch(origin_of(account_of(USER))),
                IssueError::VaultNotAcceptingNewIssues
            );
        });
    }

    /// Request fails if requested amount is below the BTC dust value
    #[test]
    fn integration_test_issue_request_precond_amount_above_dust() {
        test_with_initialized_vault(|vault_id| {
            let amount = vault_id.wrapped(1); // dust is set to 2
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: amount.amount(),
                    vault_id: vault_id,
                })
                .dispatch(origin_of(account_of(USER))),
                IssueError::AmountBelowDustAmount
            );
        });
    }

    /// Request succeeds when issuing with a vault's entire capacity
    #[test]
    fn integration_test_issue_request_precond_succeeds_at_capacity() {
        test_with_initialized_vault(|vault_id| {
            let amount = VaultRegistryPallet::get_issuable_tokens_from_vault(&vault_id).unwrap();
            let (issue_id, _) = request_issue(&vault_id, amount);
            execute_issue(issue_id);
        });
    }

    /// Request fails when trying to issue above a vault's capacity
    #[test]
    fn integration_test_issue_request_precond_fails_above_capacity() {
        test_with_initialized_vault(|vault_id| {
            let amount = vault_id.wrapped(1) + VaultRegistryPallet::get_issuable_tokens_from_vault(&vault_id).unwrap();

            // give the vault a lot of griefing collateral, to check that this isn't available for backing new tokens
            let vault_id = vault_id;
            let mut vault_data = CoreVaultData::vault(vault_id.clone());
            vault_data.griefing_collateral += griefing(1000000);
            CoreVaultData::force_to(&vault_id, vault_data);

            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: amount.amount(),
                    vault_id: vault_id,
                })
                .dispatch(origin_of(account_of(USER))),
                VaultRegistryError::ExceedingVaultLimit
            );
        });
    }

    fn get_expected_griefing_collateral(amount_btc: Amount<Runtime>) -> Amount<Runtime> {
        let amount_collateral = amount_btc.convert_to(DEFAULT_GRIEFING_CURRENCY).unwrap();
        FeePallet::get_issue_griefing_collateral(&amount_collateral).unwrap()
    }

    /// Request fails if the user can't pay the griefing collateral
    #[test]
    fn integration_test_issue_request_precond_sufficient_funds_for_collateral() {
        test_with_initialized_vault(|vault_id| {
            let amount_btc = vault_id.wrapped(10_000);
            let expected_griefing_collateral = get_expected_griefing_collateral(amount_btc);
            let mut user_state = default_user_state();
            user_state.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap().free =
                expected_griefing_collateral - Amount::new(1, DEFAULT_GRIEFING_CURRENCY);
            UserData::force_to(USER, user_state);

            // succeeds when using entire balance but not exceeding
            assert_noop!(
                Call::Issue(IssueCall::request_issue {
                    amount: amount_btc.amount(),
                    vault_id: vault_id.clone(),
                })
                .dispatch(origin_of(account_of(USER))),
                TokensError::BalanceTooLow
            );
        });
    }

    #[test]
    fn integration_test_issue_request_postcond_succeeds() {
        test_with_initialized_vault(|vault_id| {
            let amount_btc = vault_id.wrapped(100_000);
            let current_block = 500;
            SecurityPallet::set_active_block_number(current_block);
            let (_issue_id, issue) = request_issue(&vault_id, amount_btc);

            // lock griefing collateral and increase to_be_issued
            assert_eq!(
                ParachainState::get(&vault_id),
                ParachainState::get_default(&vault_id).with_changes(|user, vault, _, _| {
                    vault.to_be_issued += amount_btc;
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked += issue.griefing_collateral();
                })
            );

            let issue_id = assert_issue_request_event();
            let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

            // created issue request has expected values in all the fields
            let expected_btc_address = VaultRegistryPallet::register_deposit_address(&vault_id, issue_id).unwrap();
            let expected_public_key = VaultRegistryPallet::get_bitcoin_public_key(&vault_id.account_id).unwrap();

            let expected_fee = FeePallet::get_issue_fee(&amount_btc).unwrap();
            let expected_height = BTCRelayPallet::get_best_block_height();
            let expected_griefing_collateral = get_expected_griefing_collateral(amount_btc);

            let expected_issue = IssueRequest {
                vault: vault_id,
                opentime: current_block,
                period: IssuePallet::issue_period(),
                griefing_collateral: expected_griefing_collateral.amount(),
                amount: (amount_btc - expected_fee).amount(),
                fee: expected_fee.amount(),
                requester: account_of(USER),
                btc_address: expected_btc_address,
                btc_public_key: expected_public_key,
                btc_height: expected_height,
                status: IssueRequestStatus::Pending,
            };

            assert_eq!(issue, expected_issue);
        });
    }

    #[test]
    fn integration_test_liquidating_one_collateral_currency_does_not_impact_other_currencies() {
        test_with_initialized_vault(|vault_id| {
            let amount_btc = vault_id.wrapped(10000);

            let different_collateral = match vault_id.currencies.collateral {
                Token(KSM) => Token(DOT),
                _ => Token(KSM),
            };
            let different_collateral_vault_id = PrimitiveVaultId::new(
                vault_id.account_id.clone(),
                different_collateral.clone(),
                vault_id.currencies.wrapped.clone(),
            );
            CoreVaultData::force_to(
                &different_collateral_vault_id,
                default_vault_state(&different_collateral_vault_id),
            );

            liquidate_vault(&vault_id);
            assert_ok!(Call::Issue(IssueCall::request_issue {
                amount: amount_btc.amount(),
                vault_id: different_collateral_vault_id.clone(),
            })
            .dispatch(origin_of(account_of(ALICE))));
        });
    }
}

#[test]
fn integration_test_issue_wrapped_execute_succeeds() {
    test_with(|vault_id| {
        let vault_proof_submitter = CAROL;
        let vault_id_proof_submitter = VaultId {
            account_id: account_of(vault_proof_submitter),
            ..vault_id.clone()
        };

        let amount_btc = vault_id.wrapped(1000000);
        let collateral_vault = required_collateral_for_issue(amount_btc, vault_id.collateral_currency());

        register_vault(&vault_id, collateral_vault);
        register_vault(&vault_id_proof_submitter, collateral_vault);

        // alice requests wrapped by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue {
            amount: amount_btc.amount(),
            vault_id: vault_id,
        })
        .dispatch(origin_of(account_of(USER))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee();
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (_tx_id, _height, proof, raw_tx, _) = generate_transaction_and_mine(
            Default::default(),
            vec![],
            vec![(vault_btc_address, total_amount_btc)],
            vec![],
        );

        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(Call::Issue(IssueCall::execute_issue {
            issue_id: issue_id,
            merkle_proof: proof,
            raw_tx: raw_tx
        })
        .dispatch(origin_of(account_of(vault_proof_submitter))));
    });
}

#[test]
fn integration_test_issue_wrapped_execute_bookkeeping() {
    test_with_initialized_vault(|vault_id| {
        let requested_btc = vault_id.wrapped(1000);
        let (issue_id, issue) = request_issue(&vault_id, requested_btc);

        assert_eq!(issue.fee() + issue.amount(), requested_btc);

        execute_issue(issue_id);

        assert_eq!(
            ParachainState::get(&vault_id),
            ParachainState::get_default(&vault_id).with_changes(|user, vault, _, fee_pool| {
                (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount();
                *fee_pool.rewards_for(&vault_id) += issue.fee();
                vault.issued += issue.fee() + issue.amount();
            }),
        );
    });
}

#[test]
fn integration_test_withdraw_after_request_issue() {
    test_with(|vault_id| {
        let vault = VAULT;
        let vault_proof_submitter = CAROL;
        let vault_id_proof_submitter = VaultId {
            account_id: account_of(vault_proof_submitter),
            ..vault_id.clone()
        };
        let amount_btc = vault_id.wrapped(1000000);
        let collateral_vault = required_collateral_for_issue(amount_btc, vault_id.collateral_currency());

        register_vault(&vault_id, collateral_vault);
        register_vault(&vault_id_proof_submitter, collateral_vault);

        // alice requests wrapped by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue {
            amount: amount_btc.amount(),
            vault_id: vault_id.clone(),
        })
        .dispatch(origin_of(account_of(ALICE))));

        // Should not be possible to request more, using the same collateral
        assert!(Call::Issue(IssueCall::request_issue {
            amount: amount_btc.amount(),
            vault_id: vault_id.clone(),
        })
        .dispatch(origin_of(account_of(ALICE)))
        .is_err());

        // should not be possible to withdraw the collateral now
        assert!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
            currency_pair: vault_id.currencies.clone(),
            amount: collateral_vault.amount()
        })
        .dispatch(origin_of(account_of(vault)))
        .is_err());
    });
}

#[test]
/// overpay by a factor of 4
fn integration_test_issue_refund() {
    test_with_initialized_vault(|vault_id| {
        let requested_btc = vault_id.wrapped(1000);

        // make sure we don't have enough collateral to fulfil the overpayment
        let current_minimum_collateral =
            VaultRegistryPallet::get_required_collateral_for_vault(vault_id.clone()).unwrap();
        CoreVaultData::force_to(
            &vault_id,
            CoreVaultData {
                backing_collateral: current_minimum_collateral
                    + requested_btc.convert_to(vault_id.collateral_currency()).unwrap() * 2,
                ..CoreVaultData::vault(vault_id.clone())
            },
        );
        let initial_state = ParachainState::get(&vault_id);

        let (issue_id, issue) = request_issue(&vault_id, requested_btc);
        let sent_btc = (issue.amount() + issue.fee()) * 4;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .assert_execute();

        // not enough collateral to back sent amount, so it's as if the user sent the correct amount
        let post_redeem_state = ParachainState::get(&vault_id);
        assert_eq!(
            post_redeem_state,
            initial_state.with_changes(|user, vault, _, fee_pool| {
                (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount();
                *fee_pool.rewards_for(&vault_id) += issue.fee();
                vault.issued += issue.fee() + issue.amount();
            })
        );

        // perform the refund
        execute_refund(VAULT);

        assert_eq!(
            ParachainState::get(&vault_id),
            post_redeem_state.with_changes(|_user, vault, _, _fee_pool| {
                let reward = FeePallet::get_refund_fee_from_total(&(sent_btc - requested_btc)).unwrap();
                *vault.free_balance.get_mut(&vault_id.wrapped_currency()).unwrap() += reward;
                vault.issued += reward;
            })
        );
    });
}

mod execute_refund_payment_limits {
    use super::{assert_eq, *};

    fn setup_refund(vault_id: &VaultId) -> (H256, Amount<Runtime>) {
        let requested_btc = vault_id.wrapped(1000);

        // make sure we don't have enough collateral to fulfil the overpayment
        let current_minimum_collateral =
            VaultRegistryPallet::get_required_collateral_for_vault(vault_id.clone()).unwrap();
        CoreVaultData::force_to(
            &vault_id,
            CoreVaultData {
                backing_collateral: current_minimum_collateral
                    + requested_btc.convert_to(vault_id.collateral_currency()).unwrap() * 2,
                ..CoreVaultData::vault(vault_id.clone())
            },
        );

        let (issue_id, issue) = request_issue(&vault_id, requested_btc);
        let sent_btc = (issue.amount() + issue.fee()) * 4;

        ExecuteIssueBuilder::new(issue_id)
            .with_amount(sent_btc)
            .assert_execute();

        let refund_id = assert_refund_request_event();
        let refund = RefundPallet::get_open_refund_request_from_id(&refund_id).unwrap();

        (refund_id, vault_id.wrapped(refund.amount_btc))
    }

    #[test]
    fn integration_test_execute_refund_with_exact_amount_succeeds() {
        test_with_initialized_vault(|vault_id| {
            let (_refund_id, amount) = setup_refund(&vault_id);
            assert_ok!(execute_refund_with_amount(VAULT, amount));
        });
    }
    #[test]
    fn integration_test_execute_refund_with_overpayment_fails() {
        test_with_initialized_vault(|vault_id| {
            let (_refund_id, amount) = setup_refund(&vault_id);
            assert_err!(
                execute_refund_with_amount(VAULT, Amount::new(amount.amount() + 1, amount.currency())),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
    #[test]
    fn integration_test_execute_refund_with_underpayment_fails() {
        test_with_initialized_vault(|vault_id| {
            let (_refund_id, amount) = setup_refund(&vault_id);
            assert_err!(
                execute_refund_with_amount(VAULT, Amount::new(amount.amount() - 1, amount.currency())),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

mod execute_issue_tests {
    use super::{assert_eq, *};
    /// Execute fails if parachain is shut down
    #[test]
    fn integration_test_issue_execute_precond_not_shutdown() {
        test_with(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Issue(IssueCall::execute_issue {
                    issue_id: Default::default(),
                    merkle_proof: Default::default(),
                    raw_tx: Default::default()
                })
                .dispatch(origin_of(account_of(ALICE))),
                SystemError::CallFiltered,
            );
        });
    }

    /// Execute fails if corresponding request doesn't exist
    #[test]
    fn integration_test_issue_execute_precond_issue_exists() {
        test_with(|vault_id| {
            let (issue_id, _issue) = request_issue(&vault_id, vault_id.wrapped(4_000));
            let nonexistent_issue_id = H256::zero();

            let mut executor = ExecuteIssueBuilder::new(issue_id);
            executor
                .with_submitter(account_of(PROOF_SUBMITTER), Some(vault_id.collateral_currency()))
                .with_issue_id(nonexistent_issue_id)
                .prepare_for_execution();

            assert_noop!(executor.execute_prepared(), IssueError::IssueIdNotFound);
        });
    }

    /// Execute fails if issue request has expired
    /// cf. also mod expiry_test
    #[test]
    fn integration_test_issue_execute_precond_not_expired() {
        test_with(|vault_id| {
            let (issue_id, issue) = request_issue(&vault_id, vault_id.wrapped(4_000));
            let mut executor = ExecuteIssueBuilder::new(issue_id);
            executor.prepare_for_execution();

            SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
            mine_blocks(issue.period + 99);

            assert_noop!(executor.execute_prepared(), IssueError::CommitPeriodExpired);
        });
    }

    /// Execute fails if the execution BTC tx isn't a valid payment
    #[test]
    fn integration_test_issue_execute_precond_rawtx_valid() {
        test_with_initialized_vault(|vault_id| {
            let (issue_id, issue) = request_issue(&vault_id, vault_id.wrapped(1000));
            let (_tx_id, _height, proof, _raw_tx, mut transaction) = TransactionGenerator::new()
                .with_outputs(vec![(issue.btc_address, wrapped(1000))])
                .mine();

            SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

            // send to wrong address
            let bogus_address = BtcAddress::P2WPKHv0(H160::zero());
            transaction.outputs[0] = TransactionOutput::payment(1000, &bogus_address);
            assert_noop!(
                Call::Issue(IssueCall::execute_issue {
                    issue_id: issue_id,
                    merkle_proof: proof,
                    raw_tx: transaction.format_with(true)
                })
                .dispatch(origin_of(account_of(CAROL))),
                BTCRelayError::InvalidTxid
            );
        })
    }

    /// Execute fails if provided merkle proof of payment is not valid
    #[test]
    fn integration_test_issue_execute_precond_proof_valid() {
        test_with_initialized_vault(|vault_id| {
            let (issue_id, issue) = request_issue(&vault_id, vault_id.wrapped(1000));
            let (_tx_id, _height, mut proof, _raw_tx, transaction) = TransactionGenerator::new()
                .with_outputs(vec![(issue.btc_address, wrapped(1))])
                .mine();

            SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

            // mangle block header in merkle proof
            proof[0] += 1;
            assert_noop!(
                Call::Issue(IssueCall::execute_issue {
                    issue_id: issue_id,
                    merkle_proof: proof,
                    raw_tx: transaction.format_with(true)
                })
                .dispatch(origin_of(account_of(CAROL))),
                BTCRelayError::BlockNotFound
            );
        })
    }

    /// Execute fails if the BTC transaction underpaid, and someone other than the user is trying
    /// to execute
    #[test]
    fn integration_test_issue_execute_precond_underpayment_executed_by_requester() {
        test_with(|vault_id| {
            let (issue_id, issue) = request_issue(&vault_id, vault_id.wrapped(4_000));

            let mut executor = ExecuteIssueBuilder::new(issue_id);
            executor
                .with_amount((issue.amount() + issue.fee()) / 4)
                .with_submitter(account_of(PROOF_SUBMITTER), Some(vault_id.collateral_currency()))
                .prepare_for_execution();

            assert_noop!(executor.execute_prepared(), IssueError::InvalidExecutor);
        });
    }

    /// Test Execute postconditions when BTC payment is for the exact requested amount
    #[test]
    fn integration_test_issue_execute_postcond_exact_payment() {
        test_with_initialized_vault(|vault_id| {
            let requested_btc = vault_id.wrapped(1000);
            let (issue_id, issue) = request_issue(&vault_id, requested_btc);
            let post_request_state = ParachainState::get(&vault_id);

            ExecuteIssueBuilder::new(issue_id).assert_execute();

            // user balances are updated, tokens are minted and fees paid
            assert_eq!(
                ParachainState::get(&vault_id),
                post_request_state.with_changes(|user, vault, _, fee_pool| {
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += issue.griefing_collateral();
                    (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount();

                    *fee_pool.rewards_for(&vault_id) += issue.fee();
                    vault.issued += requested_btc;
                    vault.to_be_issued -= requested_btc;
                })
            );

            // issue request is updated: status is complete
            assert_eq!(
                IssuePallet::issue_requests(issue_id).unwrap().status,
                IssueRequestStatus::Completed(None)
            );
        });
    }

    /// Test Execute postconditions when BTC payment is less than the requested amount
    #[test]
    fn integration_test_issue_execute_postcond_underpayment() {
        test_with_initialized_vault(|vault_id| {
            let requested_btc = vault_id.wrapped(400_000);
            let (issue_id, issue) = request_issue(&vault_id, requested_btc);
            let sent_btc = (issue.amount() + issue.fee()) / 4;

            let post_request_state = ParachainState::get(&vault_id);

            // need stake for rewards to deposit
            assert_ok!(VaultRewardsPallet::deposit_stake(&vault_id, signed_fixed_point!(1)));

            ExecuteIssueBuilder::new(issue_id)
                .with_amount(sent_btc)
                .with_submitter(account_of(USER), None)
                .assert_execute();

            let slashed_griefing_collateral = (issue.griefing_collateral() * 3) / 4;
            let returned_griefing_collateral = issue.griefing_collateral() - slashed_griefing_collateral;

            // user balances are updated, tokens are minted and fees paid
            assert_eq!(
                ParachainState::get(&vault_id),
                post_request_state.with_changes(|user, vault, _, fee_pool| {
                    // user loses 75% of griefing collateral for having only fulfilled 25%
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += returned_griefing_collateral;
                    *vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += slashed_griefing_collateral;

                    // token updating as if only 25% was requested
                    (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount() / 4;
                    *fee_pool.rewards_for(&vault_id) += issue.fee() / 4;
                    vault.issued += (issue.fee() + issue.amount()) / 4;
                    vault.to_be_issued -= issue.fee() + issue.amount(); // decrease to sent_btc and then decrease to
                                                                        // zero
                                                                        // happens within execute_issue and adds up to
                                                                        // full
                                                                        // amount
                })
            );

            assert_issue_amount_change_event(
                issue_id,
                issue.amount() / 4,
                issue.fee() / 4,
                slashed_griefing_collateral,
            );

            // issue request is updated: status is complete, amounts have been adjusted
            let mut completed_issue = issue;
            completed_issue.amount /= 4;
            completed_issue.fee /= 4;
            completed_issue.status = IssueRequestStatus::Completed(None);

            assert_eq!(
                IssuePallet::issue_requests(issue_id).unwrap().status,
                IssueRequestStatus::Completed(None)
            );
            let user_issues = IssuePallet::get_issue_requests_for_account(account_of(USER));
            let onchain_issue = user_issues
                .into_iter()
                .find(|id| id == &issue_id)
                .map(|id| IssuePallet::issue_requests(id).unwrap())
                .unwrap();
            assert_eq!(onchain_issue, completed_issue);
        });
    }

    /// Test Execute postconditions when BTC payment is greater than the requested amount, and
    /// vault can execute the greater amount
    #[test]
    fn integration_test_issue_execute_postcond_overpayment_succeeds() {
        test_with_initialized_vault(|vault_id| {
            // 2000 so that 0.15% is a nice round number, otherwise we get rounding errors below. E.g. with 1000,
            // 0.15% would be 1.5, which is rounded to 2. Then when we double the sent amount, the fee is 3 instead
            // of 4, so we wouldn't quite get exactly double the expected amount.
            let requested_btc = vault_id.wrapped(2000);
            let (issue_id, issue) = request_issue(&vault_id, requested_btc);
            let sent_btc = (issue.amount() + issue.fee()) * 2;
            let post_request_state = ParachainState::get(&vault_id);

            ExecuteIssueBuilder::new(issue_id)
                .with_amount(sent_btc)
                .assert_execute();

            // user balances are updated, tokens are minted and fees paid
            assert_eq!(
                ParachainState::get(&vault_id),
                post_request_state.with_changes(|user, vault, _, fee_pool| {
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += issue.griefing_collateral();
                    (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount() * 2;

                    *fee_pool.rewards_for(&vault_id) += issue.fee() * 2;
                    vault.issued += sent_btc;
                    vault.to_be_issued -= requested_btc; // increase to sent_btc and decrease back to zero happens
                                                         // within execute_issue and cancels out
                })
            );

            assert_issue_amount_change_event(issue_id, issue.amount() * 2, issue.fee() * 2, griefing(0));

            // issue request is updated: status is complete, amounts have been adjusted
            let mut completed_issue = issue;
            completed_issue.amount *= 2;
            completed_issue.fee *= 2;
            completed_issue.status = IssueRequestStatus::Completed(None);

            let user_issues = IssuePallet::get_issue_requests_for_account(account_of(USER));
            let onchain_issue = user_issues
                .into_iter()
                .find(|id| id == &issue_id)
                .map(|id| IssuePallet::issue_requests(id).unwrap())
                .unwrap();
            assert_eq!(onchain_issue, completed_issue);
        });
    }

    /// Test Execute postconditions when BTC payment is greater than the requested amount, and
    /// vault can not execute the greater amount
    #[test]
    fn integration_test_issue_execute_postcond_overpayment_creates_refund() {
        test_with_initialized_vault(|vault_id| {
            let requested_btc = vault_id.wrapped(1000);

            // make sure we don't have enough collateral to fulfil the overpayment
            let current_minimum_collateral =
                VaultRegistryPallet::get_required_collateral_for_vault(vault_id.clone()).unwrap();
            CoreVaultData::force_to(
                &vault_id,
                CoreVaultData {
                    backing_collateral: current_minimum_collateral
                        + requested_btc.convert_to(vault_id.collateral_currency()).unwrap() * 2,
                    ..CoreVaultData::vault(vault_id.clone())
                },
            );

            let (issue_id, issue) = request_issue(&vault_id, requested_btc);
            let sent_btc = (issue.amount() + issue.fee()) * 4;
            let post_request_state = ParachainState::get(&vault_id);

            ExecuteIssueBuilder::new(issue_id)
                .with_amount(sent_btc)
                .assert_execute();

            // user balances are updated, tokens are minted and fees paid
            // not enough collateral to back sent amount, so it's as if the user sent the correct amount
            assert_eq!(
                ParachainState::get(&vault_id),
                post_request_state.with_changes(|user, vault, _, fee_pool| {
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += issue.griefing_collateral();

                    (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount();
                    *fee_pool.rewards_for(&vault_id) += issue.fee();

                    vault.issued += issue.fee() + issue.amount();
                    vault.to_be_issued -= issue.fee() + issue.amount();
                })
            );

            // refund requests exists for issue
            let refund_id = assert_refund_request_event();
            let refund = RefundPallet::get_open_refund_request_from_id(&refund_id).unwrap();
            assert_eq!(refund.issue_id, issue_id);

            // issue request is updated: status is complete and references refund request
            let user_issues = IssuePallet::get_issue_requests_for_account(account_of(USER));
            let onchain_issue = user_issues
                .into_iter()
                .find(|id| id == &issue_id)
                .map(|id| IssuePallet::issue_requests(id).unwrap())
                .unwrap();
            assert_eq!(onchain_issue.status, IssueRequestStatus::Completed(Some(refund_id)));
        });
    }

    /// Test Execute postconditions when vault has been liquidated
    #[test]
    fn integration_test_issue_execute_postcond_liquidated() {
        test_with_initialized_vault(|vault_id| {
            let (issue_id, issue) = RequestIssueBuilder::new(&vault_id, vault_id.wrapped(10_000)).request();

            liquidate_vault(&vault_id);
            let post_liquidation_status = ParachainState::get(&vault_id);

            execute_issue(issue_id);

            // user balances are updated, tokens are minted and fees paid
            assert_eq!(
                ParachainState::get(&vault_id),
                post_liquidation_status.with_changes(|user, _vault, liquidation_vault, fee_pool| {
                    (*user.balances.get_mut(&vault_id.wrapped_currency()).unwrap()).free += issue.amount();

                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();

                    let liquidation_vault = liquidation_vault.with_currency(&vault_id.currencies);
                    liquidation_vault.to_be_issued -= issue.amount() + issue.fee();
                    liquidation_vault.issued += issue.amount() + issue.fee();

                    *fee_pool.rewards_for(&vault_id) += issue.fee();
                })
            );
        });
    }
}

mod cancel_issue_tests {
    use super::{assert_eq, *};

    /// Cancel fails when parachain is shutdown
    #[test]
    fn integration_test_issue_cancel_precond_not_shutdown() {
        test_with(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);
            assert_noop!(
                Call::Issue(IssueCall::cancel_issue {
                    issue_id: H256([0; 32]),
                })
                .dispatch(origin_of(account_of(ALICE))),
                SystemError::CallFiltered,
            );
        });
    }

    /// Cancel fails if issue request does not exist
    #[test]
    fn integration_test_issue_cancel_precond_issue_exists() {
        test_with(|vault_id| {
            request_issue(&vault_id, vault_id.wrapped(4_000));
            let nonexistent_issue_id = H256::zero();
            SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
            mine_blocks((IssuePallet::issue_period() + 99) / 100 + 1);

            assert_noop!(
                Call::Issue(IssueCall::cancel_issue {
                    issue_id: nonexistent_issue_id
                })
                .dispatch(origin_of(account_of(VAULT))),
                IssueError::IssueIdNotFound
            );
        });
    }

    /// Cancel fails if issue request is not yet expired
    #[test]
    fn integration_test_issue_cancel_precond_issue_expired() {
        test_with(|vault_id| {
            let (issue_id, _issue) = request_issue(&vault_id, vault_id.wrapped(4_000));
            assert_noop!(
                Call::Issue(IssueCall::cancel_issue { issue_id: issue_id }).dispatch(origin_of(account_of(VAULT))),
                IssueError::TimeNotExpired
            );
        });
    }

    /// Test Cancel preconditions for a non-liquidated vault
    #[test]
    fn integration_test_issue_cancel_postcond_vault_not_liquidated() {
        test_with_initialized_vault(|vault_id| {
            let (issue_id, issue) = RequestIssueBuilder::new(&vault_id, vault_id.wrapped(10_000)).request();

            SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
            mine_blocks((IssuePallet::issue_period() + 99) / 100 + 1);

            let post_request_state = ParachainState::get(&vault_id);

            // bob cancels issue request
            assert_ok!(
                Call::Issue(IssueCall::cancel_issue { issue_id: issue_id }).dispatch(origin_of(account_of(VAULT)))
            );

            // balances and collaterals are updated
            assert_eq!(
                ParachainState::get(&vault_id),
                post_request_state.with_changes(|user, vault, _, _| {
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    *vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += issue.griefing_collateral();
                    vault.to_be_issued -= issue.amount() + issue.fee();
                })
            );

            // issue request status is set to cancelled
            let user_issues = IssuePallet::get_issue_requests_for_account(account_of(USER));
            let onchain_issue = user_issues
                .into_iter()
                .find(|id| id == &issue_id)
                .map(|id| IssuePallet::issue_requests(id).unwrap())
                .unwrap();
            assert_eq!(onchain_issue.status, IssueRequestStatus::Cancelled);
        });
    }

    /// Test cancel preconditions in the case that the vault was liquidated
    #[test]
    fn integration_test_issue_cancel_postcond_vault_liquidated() {
        test_with_initialized_vault(|vault_id| {
            let (issue_id, issue) = RequestIssueBuilder::new(&vault_id, vault_id.wrapped(10_000)).request();

            SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);
            mine_blocks((IssuePallet::issue_period() + 99) / 100 + 1);

            liquidate_vault(&vault_id);
            let post_liquidation_status = ParachainState::get(&vault_id);

            // bob cancels issue request
            assert_ok!(
                Call::Issue(IssueCall::cancel_issue { issue_id: issue_id }).dispatch(origin_of(account_of(VAULT)))
            );

            // grieifing collateral released back to the user
            assert_eq!(
                ParachainState::get(&vault_id),
                post_liquidation_status.with_changes(|user, _vault, liquidation_vault, _fee_pool| {
                    // griefing collateral released instead of slashed
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).locked -= issue.griefing_collateral();
                    (*user.balances.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap()).free += issue.griefing_collateral();

                    let liquidation_vault = liquidation_vault.with_currency(&vault_id.currencies);
                    liquidation_vault.to_be_issued -= issue.amount() + issue.fee();
                })
            );

            // issue request status is set to cancelled
            let user_issues = IssuePallet::get_issue_requests_for_account(account_of(USER));
            let onchain_issue = user_issues
                .into_iter()
                .find(|id| id == &issue_id)
                .map(|id| IssuePallet::issue_requests(id).unwrap())
                .unwrap();
            assert_eq!(onchain_issue.status, IssueRequestStatus::Cancelled);
        });
    }
}
