use crate::{
    setup::{assert_eq, issue_utils::assert_issue_request_event, redeem_utils::assert_redeem_request_event, *},
    utils::{loans_utils::activate_lending_and_mint, redeem_utils::get_punishment_fee},
};
use currency::Amount;
use issue::DefaultIssueRequest;
use redeem::DefaultRedeemRequest;
use sp_core::H256;

type IssueCall = issue::Call<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;

const USER: [u8; 32] = ALICE;
const OLD_VAULT: [u8; 32] = BOB;
const NEW_VAULT: [u8; 32] = CAROL;

fn test_with<R>(execute: impl Fn(VaultId, VaultId) -> R) {
    let test_with = |old_vault_currency, new_vault_currency, wrapped_currency, extra_vault_currency| {
        ExtBuilder::build().execute_with(|| {
            for currency_id in iter_collateral_currencies().filter(|c| !c.is_lend_token()) {
                assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            }
            if wrapped_currency != DEFAULT_WRAPPED_CURRENCY {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_currency, FixedU128::one()));
            }
            activate_lending_and_mint(Token(DOT), LendToken(1));
            set_default_thresholds();
            UserData::force_to(USER, default_user_state());
            let old_vault_id = VaultId::new(account_of(OLD_VAULT), old_vault_currency, wrapped_currency);
            let new_vault_id = VaultId::new(account_of(NEW_VAULT), new_vault_currency, wrapped_currency);
            CoreVaultData::force_to(&old_vault_id, default_vault_state(&old_vault_id));
            CoreVaultData::force_to(&new_vault_id, default_vault_state(&new_vault_id));
            LiquidationVaultData::force_to(default_liquidation_vault_state(&old_vault_id.currencies));

            if let Some(other_currency) = extra_vault_currency {
                assert_ok!(OraclePallet::_set_exchange_rate(other_currency, FixedU128::one()));
                // check that having other vault with the same account id does not influence tests
                let other_old_vault_id = VaultId::new(
                    old_vault_id.account_id.clone(),
                    other_currency,
                    old_vault_id.wrapped_currency(),
                );
                CoreVaultData::force_to(&other_old_vault_id, default_vault_state(&other_old_vault_id));
                let other_new_vault_id = VaultId::new(
                    new_vault_id.account_id.clone(),
                    other_currency,
                    new_vault_id.wrapped_currency(),
                );
                CoreVaultData::force_to(&other_new_vault_id, default_vault_state(&other_new_vault_id));
            }
            VaultRegistryPallet::collateral_integrity_check();

            execute(old_vault_id, new_vault_id)
        })
    };
    test_with(Token(DOT), Token(KSM), Token(KBTC), None);
    test_with(Token(DOT), Token(DOT), Token(IBTC), None);
    test_with(Token(DOT), Token(DOT), Token(IBTC), Some(Token(KSM)));
    test_with(Token(DOT), Token(KSM), Token(IBTC), None);
    test_with(Token(KSM), Token(DOT), Token(IBTC), None);
    test_with(ForeignAsset(1), Token(DOT), Token(IBTC), None);
    test_with(Token(KSM), ForeignAsset(1), Token(IBTC), None);
    test_with(LendToken(1), ForeignAsset(1), Token(IBTC), None);
    test_with(Token(KSM), LendToken(1), Token(IBTC), None);
}

fn test_without_initialization<R>(execute: impl Fn(CurrencyId) -> R) {
    ExtBuilder::build().execute_with(|| execute(Token(DOT)));
    ExtBuilder::build().execute_with(|| execute(Token(KSM)));
}

#[cfg(test)]
mod request_replace_tests {
    use super::{assert_eq, *};
    use crate::{setup::issue_utils::assert_issue_request_event, utils::redeem_utils::assert_redeem_request_event};

    fn assert_state_after_request_replace_correct(
        old_vault_id: &VaultId,
        new_vault_id: &VaultId,
        issue: &IssueRequest<AccountId32, BlockNumber, Balance, CurrencyId>,
        redeem: &RedeemRequest<AccountId32, BlockNumber, Balance, CurrencyId>,
    ) {
        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(
                |old_vault, new_vault, _| {
                    *old_vault.free_balance.get_mut(&issue.griefing_currency).unwrap() -=
                        Amount::new(issue.griefing_collateral, issue.griefing_currency);

                    old_vault.to_be_redeemed += redeem.amount_btc() - redeem.fee();
                    new_vault.to_be_issued += redeem.amount_btc();
                }
            )
        );
    }

    #[test]
    fn integration_test_replace_request_replace_at_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = new_vault_id.wrapped(DEFAULT_VAULT_TO_BE_REPLACED.amount());
            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: replace_amount.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));

            let redeem_id = assert_redeem_request_event();
            let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_eq!(redeem.amount_btc, replace_amount.amount());
            assert_eq!(redeem.issue_id.is_some(), true);

            let issue_id = assert_issue_request_event();
            let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

            assert_eq!(issue.griefing_collateral(), griefing(0));
            assert_state_after_request_replace_correct(&old_vault_id, &new_vault_id, &issue, &redeem);
        });
    }

    #[test]
    fn integration_test_replace_request_replace_below_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            // accept only 25%

            let replace_amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let replace_amount = old_vault_id.wrapped(replace_amount.amount());

            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: replace_amount.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));

            let redeem_id = assert_redeem_request_event();
            let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_eq!(redeem.amount_btc, replace_amount.amount());
            assert_eq!(redeem.issue_id.is_some(), true);

            let issue_id = assert_issue_request_event();
            let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

            assert_eq!(issue.griefing_collateral(), griefing(0));
            assert_state_after_request_replace_correct(&old_vault_id, &new_vault_id, &issue, &redeem);
        });
    }

    #[test]
    fn integration_test_request_replace_above_capacity_fails() {
        test_with(|old_vault_id, new_vault_id| {
            // try to accept 400%

            let replace_amount = DEFAULT_VAULT_TO_BE_REPLACED * 4;
            let replace_amount = old_vault_id.wrapped(replace_amount.amount());

            assert_err!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: replace_amount.amount(),
                    new_vault_id: new_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                VaultRegistryError::InsufficientTokensCommitted
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_by_vault_that_does_not_accept_issues_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            assert_ok!(RuntimeCall::VaultRegistry(VaultRegistryCall::accept_new_issues {
                currency_pair: new_vault_id.currencies.clone(),
                accept_new_issues: false
            })
            .dispatch(origin_of(new_vault_id.account_id.clone())));

            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: 1000,
                    new_vault_id: new_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                IssueError::VaultNotAcceptingNewIssues
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_below_dust_fails() {
        test_with(|old_vault_id, new_vault_id| {
            // if the new_vault _asks_ for an amount below below DUST, it gets rejected

            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: 1,
                    new_vault_id: new_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                IssueError::AmountBelowDustAmount
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_self_fails() {
        test_with(|old_vault_id, _new_vault_id| {
            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: 10_000,
                    new_vault_id: old_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                RedeemError::ReplaceSelfNotAllowed
            );
        });
    }

    #[test]
    fn integration_test_replace_other_wrapped_currency_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let other_currency = if let Token(IBTC) = old_vault_id.wrapped_currency() {
                Token(KBTC)
            } else {
                Token(IBTC)
            };
            assert_ok!(OraclePallet::_set_exchange_rate(other_currency, FixedU128::one()));

            let new_vault_id = VaultId::new(
                account_of(NEW_VAULT),
                new_vault_id.collateral_currency(),
                other_currency,
            );
            CoreVaultData::force_to(&new_vault_id, default_vault_state(&new_vault_id));

            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: 10000,
                    new_vault_id: new_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                RedeemError::InvalidWrappedCurrency
            );
        })
    }
}

mod expiry_test {
    use super::{assert_eq, *};
    use crate::{setup::redeem_utils::assert_redeem_request_event, utils::issue_utils::assert_issue_request_event};

    /// test replace created by accept
    fn test_with(initial_period: u32, execute: impl Fn((H256, H256))) {
        let amount_btc = wrapped(5_000);
        super::test_with(|old_vault_id, new_vault_id| {
            set_replace_period(initial_period);
            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: amount_btc.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));
            let redeem_id = assert_redeem_request_event();
            let issue_id = assert_issue_request_event();

            execute((redeem_id, issue_id));
        });
    }

    fn set_replace_period(period: u32) {
        assert_ok!(RuntimeCall::Redeem(RedeemCall::set_redeem_period { period }).dispatch(root()));
    }

    fn cancel_replace(redeem_id: H256) -> DispatchResultWithPostInfo {
        RuntimeCall::Redeem(RedeemCall::cancel_redeem {
            redeem_id: redeem_id,
            reimburse: true,
        })
        .dispatch(origin_of(account_of(NEW_VAULT)))
    }

    #[test]
    fn integration_test_replace_expiry_only_parachain_blocks_expired() {
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(1);
            SecurityPallet::set_active_block_number(1500);

            assert_noop!(cancel_replace(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_replace(redeem_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_only_bitcoin_blocks_expired() {
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(500);

            assert_noop!(cancel_replace(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_replace(redeem_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_pre_expiry() {
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(7);
            SecurityPallet::set_active_block_number(750);

            assert_noop!(cancel_replace(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_replace(redeem_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_post_expiry() {
        // can still execute after expiry
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(100);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(execute_replace(redeem_id));
        });

        // but new-vault can also cancel.. whoever is first wins
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(100);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(cancel_replace(redeem_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_decrease() {
        test_with(2000, |(redeem_id, _issue_id)| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(1000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_err!(cancel_replace(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_replace(redeem_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_increase() {
        test_with(1000, |(redeem_id, _issue_id)| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(2000);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_err!(cancel_replace(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_replace(redeem_id));
        });
    }
}

mod execute_replace_payment_limits {
    use super::{assert_eq, *};
    use crate::utils::redeem_utils::assert_redeem_request_event;

    #[test]
    fn integration_test_execute_replace_with_exact_amount_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = old_vault_id.wrapped(10000);

            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: replace_amount.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));
            let redeem_id = assert_redeem_request_event();

            let replace = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_ok!(execute_replace_with_amount(
                redeem_id,
                Amount::new(replace.amount_btc, replace.vault.currencies.wrapped)
            ));
        });
    }

    #[test]
    fn integration_test_execute_replace_with_overpayment_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = old_vault_id.wrapped(10000);

            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: replace_amount.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));

            let redeem_id = assert_redeem_request_event();

            let replace = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_err!(
                execute_replace_with_amount(
                    redeem_id,
                    Amount::new(replace.amount_btc, replace.vault.currencies.wrapped).with_amount(|x| x + 1)
                ),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
    #[test]
    fn integration_test_execute_replace_with_underpayment_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = old_vault_id.wrapped(10000);

            assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: replace_amount.amount(),
                new_vault_id: new_vault_id.clone(),
                griefing_currency: DEFAULT_GRIEFING_CURRENCY
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())));

            let redeem_id = assert_redeem_request_event();

            let replace = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_err!(
                execute_replace_with_amount(
                    redeem_id,
                    Amount::new(replace.amount_btc, replace.vault.currencies.wrapped).with_amount(|x| x - 1)
                ),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

fn setup_replace(
    replace_amount: Amount<Runtime>,
    old_vault_id: &VaultId,
    new_vault_id: &VaultId,
) -> (H256, DefaultRedeemRequest<Runtime>, H256, DefaultIssueRequest<Runtime>) {
    assert_ok!(RuntimeCall::Redeem(RedeemCall::request_replace {
        currency_pair: old_vault_id.currencies.clone(),
        amount: replace_amount.amount(),
        new_vault_id: new_vault_id.clone(),
        griefing_currency: DEFAULT_GRIEFING_CURRENCY
    })
    .dispatch(origin_of(old_vault_id.account_id.clone())));
    let redeem_id = assert_redeem_request_event();
    let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

    let issue_id = assert_issue_request_event();
    let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

    (redeem_id, redeem, issue_id, issue)
}

#[test]
fn integration_test_replace_cancel_replace() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, _, _, _) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);
        // set block height
        // new_vault cancels replacement
        mine_blocks(2);
        SecurityPallet::set_active_block_number(30);
        assert_ok!(RuntimeCall::Redeem(RedeemCall::cancel_redeem {
            redeem_id: redeem_id,
            reimburse: true,
        })
        .dispatch(origin_of(new_vault_id.account_id.clone())));
    });
}

// liquidation tests

fn execute_replace(redeem_id: H256) -> DispatchResultWithPostInfo {
    let replace = RedeemPallet::get_open_redeem_request_from_id(&redeem_id)?;
    execute_replace_with_amount(
        redeem_id,
        Amount::new(replace.amount_btc, replace.vault.currencies.wrapped),
    )
}

fn execute_replace_with_amount(redeem_id: H256, amount: Amount<Runtime>) -> DispatchResultWithPostInfo {
    let replace = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

    // send the btc from the old_vault to the new_vault
    let (_tx_id, _tx_block_height, transaction) = generate_transaction_and_mine(
        Default::default(),
        vec![],
        vec![(replace.btc_address, amount)],
        vec![redeem_id],
    );

    SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

    RuntimeCall::Redeem(RedeemCall::execute_redeem {
        redeem_id,
        unchecked_transaction: transaction,
    })
    .dispatch(origin_of(account_of(OLD_VAULT)))
}

fn cancel_replace(redeem_id: H256) {
    // set block height
    mine_blocks(2);
    SecurityPallet::set_active_block_number(30);
    assert_ok!(RuntimeCall::Redeem(RedeemCall::cancel_redeem {
        redeem_id: redeem_id,
        reimburse: true,
    })
    .dispatch(origin_of(account_of(NEW_VAULT))));
}

#[test]
fn integration_test_replace_execute_replace_success() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);

        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let pre_execute_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(redeem_id));
        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                old_vault.issued -= replace_amount - redeem.fee;
                old_vault.to_be_redeemed -= replace_amount - redeem.fee;

                new_vault.issued += replace_amount;
                new_vault.to_be_issued -= replace_amount;
            })
        );
    });
}

fn calculate_replace_collateral(
    vault_data: &CoreVaultData,
    replace_amount: Amount<Runtime>,
    currency_id: CurrencyId,
) -> Amount<Runtime> {
    Amount::new(
        (vault_data.backing_collateral.amount() * replace_amount.amount())
            / (vault_data.issued + vault_data.to_be_issued).amount(),
        currency_id,
    )
}

#[test]
fn integration_test_replace_execute_replace_old_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);

        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);

        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(redeem_id));

        let collateral_for_replace = calculate_replace_collateral(
            &old,
            redeem.amount_btc().checked_sub(&redeem.fee()).unwrap(),
            old_vault_id.collateral_currency(),
        );

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);

                liquidation_vault.issued -= replace_amount - redeem.fee;
                liquidation_vault.to_be_redeemed -= replace_amount - redeem.fee;

                new_vault.to_be_issued -= replace_amount;
                new_vault.issued += replace_amount;

                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.liquidated_collateral -= collateral_for_replace;
                *old_vault
                    .free_balance
                    .get_mut(&old_vault_id.collateral_currency())
                    .unwrap() += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_new_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);

        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        liquidate_vault(&new_vault_id);

        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(redeem_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);

                liquidation_vault.issued += replace_amount;
                liquidation_vault.to_be_issued -= replace_amount;

                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.issued -= replace_amount - redeem.fee;
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_both_vaults_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);

        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);
        liquidate_vault(&new_vault_id);

        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(redeem_id));

        let collateral_for_replace = calculate_replace_collateral(
            &old,
            redeem.amount_btc().checked_sub(&redeem.fee()).unwrap(),
            old_vault_id.collateral_currency(),
        );

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let old_liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);
                old_liquidation_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_liquidation_vault.issued -= replace_amount - redeem.fee;

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= replace_amount;
                new_liquidation_vault.issued += replace_amount;

                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.liquidated_collateral -= collateral_for_replace;
                *old_vault
                    .free_balance
                    .get_mut(&old_vault_id.collateral_currency())
                    .unwrap() += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_with_cancelled() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, _redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);
        cancel_replace(redeem_id);
        assert_err!(execute_replace(redeem_id), RedeemError::RedeemCancelled);
    });
}

#[test]
fn integration_test_replace_cancel_replace_success() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);
        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);
        cancel_replace(redeem_id);

        let punishment_fee = get_punishment_fee();
        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= replace_amount;
                old_vault.backing_collateral -= punishment_fee;
                *new_vault.free_balance.get_mut(&punishment_fee.currency()).unwrap() += punishment_fee;

                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_old_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(redeem_id);

        let collateral_for_replace = calculate_replace_collateral(
            &old,
            redeem.amount_btc().checked_sub(&redeem.fee()).unwrap(),
            old_vault_id.collateral_currency(),
        );

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.liquidated_collateral -= collateral_for_replace;

                new_vault.to_be_issued -= replace_amount;

                let liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);

                liquidation_vault.to_be_redeemed -= replace_amount - redeem.fee;
                liquidation_vault.collateral += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_new_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        liquidate_vault(&new_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(redeem_id);
        let punishment_fee = get_punishment_fee();
        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.backing_collateral -= punishment_fee;
                *new_vault.free_balance.get_mut(&punishment_fee.currency()).unwrap() += punishment_fee;

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= replace_amount;
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_both_vaults_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, redeem, _issue_id, issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);
        liquidate_vault(&new_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(redeem_id);

        let collateral_for_replace = calculate_replace_collateral(
            &old,
            redeem.amount_btc().checked_sub(&redeem.fee()).unwrap(),
            old_vault_id.collateral_currency(),
        );

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.liquidated_collateral -= collateral_for_replace;

                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += issue.griefing_collateral();

                let old_liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);
                old_liquidation_vault.to_be_redeemed -= replace_amount;
                old_liquidation_vault.collateral += collateral_for_replace;

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= replace_amount;
                let new_liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);
                new_liquidation_vault.to_be_redeemed += redeem.fee(); //KBTC
            })
        );
    });
}

#[test]
fn integration_test_replace_vault_with_different_currency_succeeds() {
    test_without_initialization(|currency_id| {
        for currency_id in iter_collateral_currencies().filter(|c| !c.is_lend_token()) {
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
        }
        set_default_thresholds();
        SecurityPallet::set_active_block_number(1);

        let other_currency = if let Token(DOT) = currency_id {
            Token(KSM)
        } else {
            Token(DOT)
        };

        let old_vault_id = vault_id_of(OLD_VAULT, currency_id);
        let new_vault_id = vault_id_of(NEW_VAULT, other_currency);

        // Mint lendTokens so that force-setting vault state doesn't fail
        activate_lending_and_mint(Token(DOT), LendToken(1));
        CoreVaultData::force_to(&old_vault_id, default_vault_state(&old_vault_id));
        CoreVaultData::force_to(&new_vault_id, default_vault_state(&new_vault_id));

        let replace_amount = new_vault_id.wrapped(10000);
        let (redeem_id, redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

        let pre_execute_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(redeem_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= replace_amount;
                new_vault.issued += replace_amount;
                old_vault.to_be_redeemed -= replace_amount - redeem.fee;
                old_vault.issued -= replace_amount - redeem.fee;
            })
        );
    });
}

mod oracle_down {
    use super::{assert_eq, *};

    #[test]
    fn no_oracle_execute_replace_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = new_vault_id.wrapped(10000);
            let (redeem_id, _redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

            OraclePallet::expire_all();

            assert_ok!(execute_replace(redeem_id));
        });
    }

    #[test]
    fn no_oracle_request_replace_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
            OraclePallet::expire_all();

            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: amount.amount(),
                    new_vault_id: new_vault_id.clone(),
                    griefing_currency: DEFAULT_GRIEFING_CURRENCY
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                OracleError::MissingExchangeRate
            );
        })
    }

    #[test]
    fn no_oracle_cancel_replace_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = new_vault_id.wrapped(10000);
            let (redeem_id, _redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

            OraclePallet::expire_all();

            mine_blocks(2);
            SecurityPallet::set_active_block_number(30);

            assert_noop!(
                RuntimeCall::Redeem(RedeemCall::cancel_redeem {
                    redeem_id: redeem_id,
                    reimburse: true,
                })
                .dispatch(origin_of(account_of(NEW_VAULT))),
                OracleError::MissingExchangeRate
            );
        })
    }

    #[test]
    fn no_oracle_execute_cancelled_replace_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let replace_amount = new_vault_id.wrapped(10000);
            let (redeem_id, _redeem, _issue_id, _issue) = setup_replace(replace_amount, &old_vault_id, &new_vault_id);

            OraclePallet::expire_all();
            assert_ok!(execute_replace(redeem_id));
        })
    }
}
