mod mock;

use currency::Amount;
use mock::{assert_eq, replace_testing_utils::*, *};

use sp_core::H256;
use vault_registry::DefaultVaultId;

type IssueCall = issue::Call<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;
pub type ReplaceError = replace::Error<Runtime>;

const USER: [u8; 32] = ALICE;
const OLD_VAULT: [u8; 32] = BOB;
const NEW_VAULT: [u8; 32] = CAROL;

fn test_with<R>(execute: impl Fn(VaultId, VaultId) -> R) {
    let test_with = |old_vault_currency, new_vault_currency, wrapped_currency, extra_vault_currency| {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(OraclePallet::_set_exchange_rate(old_vault_currency, FixedU128::one()));
            assert_ok!(OraclePallet::_set_exchange_rate(new_vault_currency, FixedU128::one()));

            if wrapped_currency != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_currency, FixedU128::one()));
            }
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
            execute(old_vault_id, new_vault_id)
        })
    };
    test_with(Token(DOT), Token(KSM), Token(KBTC), None);
    test_with(Token(DOT), Token(DOT), Token(IBTC), None);
    test_with(Token(DOT), Token(DOT), Token(IBTC), Some(Token(KSM)));
    test_with(Token(DOT), Token(KSM), Token(IBTC), None);
    test_with(Token(KSM), Token(DOT), Token(IBTC), None);
}

fn test_without_initialization<R>(execute: impl Fn(CurrencyId) -> R) {
    ExtBuilder::build().execute_with(|| execute(Token(DOT)));
    ExtBuilder::build().execute_with(|| execute(Token(KSM)));
}

pub fn withdraw_replace(old_vault_id: &VaultId, amount: Amount<Runtime>) -> DispatchResultWithPostInfo {
    Call::Replace(ReplaceCall::withdraw_replace {
        currency_pair: old_vault_id.currencies.clone(),
        amount: amount.amount(),
    })
    .dispatch(origin_of(old_vault_id.account_id.clone()))
}

pub fn assert_replace_request_event() {
    let events = SystemPallet::events();
    let ids = events.iter().filter_map(|r| match r.event {
        Event::Replace(ReplaceEvent::RequestReplace { .. }) => Some(()),
        _ => None,
    });
    assert_eq!(ids.count(), 1);
}

#[cfg(test)]
mod accept_replace_tests {
    use super::{assert_eq, *};

    fn assert_state_after_accept_replace_correct(
        old_vault_id: &VaultId,
        new_vault_id: &VaultId,
        replace: &ReplaceRequest<AccountId32, BlockNumber, Balance, CurrencyId>,
    ) {
        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(
                |old_vault, new_vault, _| {
                    *new_vault
                        .free_balance
                        .get_mut(&new_vault_id.collateral_currency())
                        .unwrap() -= replace.collateral().unwrap();
                    new_vault.backing_collateral += replace.collateral().unwrap();

                    old_vault.replace_collateral -= griefing(
                        (old_vault.replace_collateral.amount() * replace.amount) / old_vault.to_be_replaced.amount(),
                    );
                    old_vault.to_be_replaced -= replace.amount();

                    old_vault.to_be_redeemed += replace.amount();
                    new_vault.to_be_issued += replace.amount();
                }
            )
        );
    }

    #[test]
    fn integration_test_replace_accept_replace_at_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let accept_amount = new_vault_id.wrapped(DEFAULT_VAULT_TO_BE_REPLACED.amount());
            let new_vault_additional_collateral = Amount::new(10_000, new_vault_id.collateral_currency());

            let (_, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                accept_amount,
                new_vault_additional_collateral,
                Default::default(),
            )
            .unwrap();

            assert_eq!(replace.amount(), accept_amount);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(&old_vault_id, &new_vault_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            // accept only 25%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let accept_amount = old_vault_id.wrapped(accept_amount.amount());
            let new_vault_additional_collateral = Amount::new(10_000, new_vault_id.collateral_currency());

            let (_, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                accept_amount,
                new_vault_additional_collateral,
                Default::default(),
            )
            .unwrap();

            assert_eq!(replace.amount(), accept_amount);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL / 4);

            assert_state_after_accept_replace_correct(&old_vault_id, &new_vault_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_above_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            // try to accept 400%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED * 4;
            let accept_amount = old_vault_id.wrapped(accept_amount.amount());
            let new_vault_additional_collateral = Amount::new(10_000, new_vault_id.collateral_currency());

            let (_, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                accept_amount,
                new_vault_additional_collateral,
                Default::default(),
            )
            .unwrap();

            assert_eq!(replace.amount(), accept_amount / 4);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral / 4);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(&old_vault_id, &new_vault_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_by_vault_that_does_not_accept_issues_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::accept_new_issues {
                currency_pair: new_vault_id.currencies.clone(),
                accept_new_issues: false
            })
            .dispatch(origin_of(new_vault_id.account_id.clone())));

            let (_, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                old_vault_id.wrapped(1000),
                griefing(1000),
                Default::default(),
            )
            .unwrap();

            assert_state_after_accept_replace_correct(&old_vault_id, &new_vault_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_dust_fails() {
        test_with(|old_vault_id, new_vault_id| {
            // if the new_vault _asks_ for an amount below below DUST, it gets rejected

            assert_noop!(
                accept_replace(
                    &old_vault_id,
                    &new_vault_id,
                    old_vault_id.wrapped(1),
                    griefing(10_000),
                    BtcAddress::P2PKH(H160([1; 20]))
                ),
                ReplaceError::AmountBelowDustAmount
            );

            // if the old_vault does not have sufficient to-be-replaced tokens, it gets rejected
            CoreVaultData::force_to(
                &old_vault_id,
                CoreVaultData {
                    to_be_replaced: old_vault_id.wrapped(1),
                    ..default_vault_state(&old_vault_id)
                },
            );
            assert_noop!(
                accept_replace(
                    &old_vault_id,
                    &new_vault_id,
                    old_vault_id.wrapped(1),
                    griefing(10_000),
                    BtcAddress::P2PKH(H160([1; 20]))
                ),
                ReplaceError::AmountBelowDustAmount
            );
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_self_fails() {
        test_with(|old_vault_id, _new_vault_id| {
            assert_noop!(
                accept_replace(
                    &old_vault_id,
                    &old_vault_id,
                    DEFAULT_VAULT_TO_BE_REPLACED,
                    griefing(10_000),
                    BtcAddress::P2PKH(H160([1; 20]))
                ),
                ReplaceError::ReplaceSelfNotAllowed
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
                accept_replace(
                    &old_vault_id,
                    &new_vault_id,
                    old_vault_id.wrapped(10000),
                    griefing(10_000),
                    BtcAddress::P2PKH(H160([1; 20]))
                ),
                ReplaceError::InvalidWrappedCurrency
            );
        })
    }
}

mod request_replace_tests {
    use primitives::VaultCurrencyPair;

    use super::{assert_eq, *};
    #[test]
    fn integration_test_replace_should_fail_if_not_running() {
        test_without_initialization(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Replace(ReplaceCall::request_replace {
                    currency_pair: VaultCurrencyPair {
                        collateral: Token(DOT),
                        wrapped: Token(DOT),
                    },
                    amount: 0,
                })
                .dispatch(origin_of(account_of(OLD_VAULT))),
                SystemError::CallFiltered,
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_at_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
            let amount = old_vault_id.wrapped(amount.amount());
            let griefing_collateral = request_replace(&old_vault_id, amount);
            // assert request event
            assert_replace_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_above_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) * 2;
            let amount = old_vault_id.wrapped(amount.amount());

            let griefing_collateral = request_replace(&old_vault_id, amount);

            // assert request event
            assert_replace_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount / 2;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_below_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) / 2;
            let amount = old_vault_id.wrapped(amount.amount());
            let griefing_collateral = request_replace(&old_vault_id, amount);

            // assert request event
            let _request_id = assert_replace_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_zero_btc_fails() {
        test_with(|old_vault_id, _new_vault_id| {
            assert_noop!(
                Call::Replace(ReplaceCall::request_replace {
                    currency_pair: old_vault_id.currencies.clone(),
                    amount: 0,
                })
                .dispatch(origin_of(old_vault_id.account_id.clone())),
                ReplaceError::ReplaceAmountZero
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_insufficient_collateral() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = old_vault_id.wrapped(1000);

            CoreVaultData::force_to(
                &old_vault_id,
                CoreVaultData {
                    to_be_replaced: old_vault_id.wrapped(5_000),
                    replace_collateral: griefing(1),
                    ..default_vault_state(&old_vault_id)
                },
            );

            let pre_request_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

            // check that by locking sufficient collateral we can recover
            let griefing_collateral = request_replace(&old_vault_id, amount);

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                pre_request_state.with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }
}

mod withdraw_replace_tests {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_replace_withdraw_replace_at_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let amount = DEFAULT_VAULT_TO_BE_REPLACED;
            let amount = old_vault_id.wrapped(amount.amount());

            assert_ok!(withdraw_replace(&old_vault_id, amount));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_below_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            // withdraw 25%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let amount = old_vault_id.wrapped(amount.amount());

            assert_ok!(withdraw_replace(&old_vault_id, amount));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL / 4;

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_above_capacity_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            // withdraw 200% - should just be capped to 100%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED * 2;
            let amount = old_vault_id.wrapped(amount.amount());

            assert_ok!(withdraw_replace(&old_vault_id, amount));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
                ParachainTwoVaultState::get_default(&old_vault_id, &new_vault_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount / 2;

                    *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }
    #[test]
    fn integration_test_replace_withdraw_replace_with_zero_to_be_replaced_tokens_fails() {
        test_with(|old_vault_id, _new_vault_id| {
            CoreVaultData::force_to(
                &old_vault_id,
                CoreVaultData {
                    to_be_replaced: old_vault_id.wrapped(0),
                    ..default_vault_state(&old_vault_id)
                },
            );

            assert_noop!(
                withdraw_replace(&old_vault_id, old_vault_id.wrapped(1000)),
                ReplaceError::NoPendingRequest
            );
        });
    }
}

mod expiry_test {
    use super::{assert_eq, *};

    /// test replace created by accept
    fn test_with(initial_period: u32, execute: impl Fn(H256)) {
        let amount_btc = wrapped(5_000);
        let griefing_collateral = griefing(1000);
        super::test_with(|old_vault_id, new_vault_id| {
            set_replace_period(initial_period);
            let (replace_id, _replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                amount_btc,
                griefing_collateral,
                BtcAddress::P2PKH(H160([1; 20])),
            )
            .unwrap();
            execute(replace_id);
        });
    }

    fn set_replace_period(period: u32) {
        assert_ok!(Call::Replace(ReplaceCall::set_replace_period { period }).dispatch(root()));
    }

    fn cancel_replace(replace_id: H256) -> DispatchResultWithPostInfo {
        Call::Replace(ReplaceCall::cancel_replace { replace_id: replace_id }).dispatch(origin_of(account_of(NEW_VAULT)))
    }

    #[test]
    fn integration_test_replace_expiry_only_parachain_blocks_expired() {
        test_with(1000, |replace_id| {
            mine_blocks(1);
            SecurityPallet::set_active_block_number(1500);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_only_bitcoin_blocks_expired() {
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(500);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_pre_expiry() {
        test_with(1000, |replace_id| {
            mine_blocks(7);
            SecurityPallet::set_active_block_number(750);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_post_expiry() {
        // can still execute after expiry
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(execute_replace(replace_id));
        });

        // but new-vault can also cancel.. whoever is first wins
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(cancel_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_decrease() {
        test_with(2000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(1000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_increase() {
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(2000);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }
}

mod execute_replace_payment_limits {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_execute_replace_with_exact_amount_succeeds() {
        test_with(|old_vault_id, new_vault_id| {
            let (replace_id, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                old_vault_id.wrapped(1000000),
                DEFAULT_GRIEFING_COLLATERAL,
                Default::default(),
            )
            .unwrap();
            assert_ok!(execute_replace_with_amount(replace_id, replace.amount()));
        });
    }
    #[test]
    fn integration_test_execute_replace_with_overpayment_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let (replace_id, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                old_vault_id.wrapped(1000000),
                DEFAULT_GRIEFING_COLLATERAL,
                Default::default(),
            )
            .unwrap();
            assert_err!(
                execute_replace_with_amount(replace_id, replace.amount().with_amount(|x| x + 1)),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
    #[test]
    fn integration_test_execute_replace_with_underpayment_fails() {
        test_with(|old_vault_id, new_vault_id| {
            let (replace_id, replace) = accept_replace(
                &old_vault_id,
                &new_vault_id,
                old_vault_id.wrapped(1000000),
                DEFAULT_GRIEFING_COLLATERAL,
                Default::default(),
            )
            .unwrap();
            assert_err!(
                execute_replace_with_amount(replace_id, replace.amount().with_amount(|x| x - 1)),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

#[test]
fn integration_test_replace_with_parachain_shutdown_fails() {
    test_with(|old_vault_id, new_vault_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Replace(ReplaceCall::request_replace {
                currency_pair: old_vault_id.currencies.clone(),
                amount: old_vault_id.wrapped(0).amount(),
            })
            .dispatch(origin_of(old_vault_id.account_id.clone())),
            SystemError::CallFiltered,
        );
        assert_noop!(
            withdraw_replace(&old_vault_id, old_vault_id.wrapped(0)),
            SystemError::CallFiltered
        );
        assert_noop!(
            accept_replace(
                &old_vault_id,
                &new_vault_id,
                old_vault_id.wrapped(0),
                griefing(0),
                Default::default()
            ),
            SystemError::CallFiltered
        );

        assert_noop!(
            Call::Replace(ReplaceCall::execute_replace {
                replace_id: Default::default(),
                merkle_proof: Default::default(),
                raw_tx: Default::default()
            })
            .dispatch(origin_of(account_of(OLD_VAULT))),
            SystemError::CallFiltered
        );

        assert_noop!(
            Call::Replace(ReplaceCall::cancel_replace {
                replace_id: Default::default()
            })
            .dispatch(origin_of(account_of(OLD_VAULT))),
            SystemError::CallFiltered
        );
    })
}

#[test]
fn integration_test_replace_cancel_replace() {
    test_with(|old_vault_id, new_vault_id| {
        let (_, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        // set block height
        // new_vault cancels replacement
        mine_blocks(2);
        SecurityPallet::set_active_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace { replace_id: replace_id })
            .dispatch(origin_of(new_vault_id.account_id.clone())));
    });
}

// liquidation tests

fn execute_replace(replace_id: H256) -> DispatchResultWithPostInfo {
    let replace = ReplacePallet::get_open_or_cancelled_replace_request(&replace_id).unwrap();
    execute_replace_with_amount(replace_id, replace.amount())
}

fn execute_replace_with_amount(replace_id: H256, amount: Amount<Runtime>) -> DispatchResultWithPostInfo {
    let replace = ReplacePallet::get_open_or_cancelled_replace_request(&replace_id).unwrap();

    // send the btc from the old_vault to the new_vault
    let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = generate_transaction_and_mine(
        Default::default(),
        vec![],
        vec![(replace.btc_address, amount)],
        vec![replace_id],
    );

    SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

    Call::Replace(ReplaceCall::execute_replace {
        replace_id: replace_id,
        merkle_proof: merkle_proof,
        raw_tx: raw_tx,
    })
    .dispatch(origin_of(account_of(OLD_VAULT)))
}

fn cancel_replace(replace_id: H256) {
    // set block height
    mine_blocks(2);
    SecurityPallet::set_active_block_number(30);
    assert_ok!(Call::Replace(ReplaceCall::cancel_replace { replace_id: replace_id })
        .dispatch(origin_of(account_of(NEW_VAULT))));
}

#[test]
fn integration_test_replace_execute_replace_success() {
    test_with(|old_vault_id, new_vault_id| {
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        let pre_execute_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_vault.issued += old_vault_id.wrapped(1000);
                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.issued -= old_vault_id.wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
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
        let replace_tokens = old_vault_id.wrapped(1000);
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, replace_tokens);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);

        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(replace_id));

        let collateral_for_replace =
            calculate_replace_collateral(&old, replace.amount(), old_vault_id.collateral_currency());

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);

                liquidation_vault.issued -= old_vault_id.wrapped(1000);
                liquidation_vault.to_be_redeemed -= old_vault_id.wrapped(1000);

                new_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_vault.issued += old_vault_id.wrapped(1000);

                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.liquidated_collateral -= collateral_for_replace;
                old_vault.backing_collateral += collateral_for_replace; // TODO: probably should be free
                *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_new_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_tokens = old_vault_id.wrapped(1000);
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, replace_tokens);

        liquidate_vault(&new_vault_id);
        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.issued += old_vault_id.wrapped(1000);
                new_liquidation_vault.to_be_issued -= old_vault_id.wrapped(1000);

                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.issued -= old_vault_id.wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_both_vaults_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let replace_tokens = old_vault_id.wrapped(1000);
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, replace_tokens);

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);
        liquidate_vault(&new_vault_id);

        let pre_execution_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(replace_id));

        let collateral_for_replace =
            calculate_replace_collateral(&old, replace.amount(), old_vault_id.collateral_currency());

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let old_liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);
                old_liquidation_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_liquidation_vault.issued -= old_vault_id.wrapped(1000);

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_liquidation_vault.issued += old_vault_id.wrapped(1000);

                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.liquidated_collateral -= collateral_for_replace;
                old_vault.backing_collateral += collateral_for_replace; // TODO: probably should be free
                *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_with_cancelled() {
    test_with(|old_vault_id, new_vault_id| {
        let issued_tokens = old_vault_id.wrapped(1000);
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, issued_tokens);

        let before = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);
        cancel_replace(replace_id);
        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            before.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= issued_tokens;
                new_vault.issued += issued_tokens;
                old_vault.to_be_redeemed -= issued_tokens;
                old_vault.issued -= issued_tokens;

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_with_additional_and_cancelled() {
    test_with(|old_vault_id, new_vault_id| {
        let issued_tokens = old_vault_id.wrapped(1000);
        let collateral = old_vault_id.collateral(100000);
        let (replace, replace_id) =
            setup_replace_with_collateral(&old_vault_id, &new_vault_id, issued_tokens, collateral);

        let before = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);
        cancel_replace(replace_id);
        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            before.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= issued_tokens;
                new_vault.issued += issued_tokens;
                old_vault.to_be_redeemed -= issued_tokens;
                old_vault.issued -= issued_tokens;

                // griefing collateral is slashed
                old_vault.griefing_collateral -= replace.griefing_collateral();
                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                // backing collateral is returned
                new_vault.backing_collateral -= replace.collateral().unwrap();
                *new_vault
                    .free_balance
                    .get_mut(&new_vault_id.collateral_currency())
                    .unwrap() += replace.collateral().unwrap();
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_success() {
    test_with(|old_vault_id, new_vault_id| {
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(replace_id);

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_vault.backing_collateral -= replace.collateral().unwrap();
                *new_vault
                    .free_balance
                    .get_mut(&new_vault_id.collateral_currency())
                    .unwrap() += replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_old_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(replace_id);

        let collateral_for_replace =
            calculate_replace_collateral(&old, replace.amount(), old_vault_id.collateral_currency());

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);

                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();
                old_vault.liquidated_collateral -= collateral_for_replace;

                new_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_vault.backing_collateral -= replace.collateral().unwrap();
                *new_vault
                    .free_balance
                    .get_mut(&new_vault_id.collateral_currency())
                    .unwrap() += replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                liquidation_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                liquidation_vault.collateral += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_new_vault_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        liquidate_vault(&new_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(replace_id);

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();

                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= old_vault_id.wrapped(1000);
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_both_vaults_liquidated() {
    test_with(|old_vault_id, new_vault_id| {
        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        let old = CoreVaultData::vault(old_vault_id.clone());

        liquidate_vault(&old_vault_id);
        liquidate_vault(&new_vault_id);

        let pre_cancellation_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        cancel_replace(replace_id);

        let collateral_for_replace =
            calculate_replace_collateral(&old, replace.amount(), old_vault_id.collateral_currency());

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();
                old_vault.liquidated_collateral -= collateral_for_replace;

                *new_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                let old_liquidation_vault = liquidation_vault.with_currency(&old_vault_id.currencies);
                old_liquidation_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_liquidation_vault.collateral += collateral_for_replace;

                let new_liquidation_vault = liquidation_vault.with_currency(&new_vault_id.currencies);
                new_liquidation_vault.to_be_issued -= old_vault_id.wrapped(1000);
            })
        );
    });
}

#[test]
fn integration_test_replace_vault_with_different_currency_succeeds() {
    test_without_initialization(|currency_id| {
        for currency_id in iter_collateral_currencies() {
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

        CoreVaultData::force_to(&old_vault_id, default_vault_state(&old_vault_id));
        CoreVaultData::force_to(&new_vault_id, default_vault_state(&new_vault_id));

        let (replace, replace_id) = setup_replace(&old_vault_id, &new_vault_id, old_vault_id.wrapped(1000));

        let pre_execute_state = ParachainTwoVaultState::get(&old_vault_id, &new_vault_id);

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(&old_vault_id, &new_vault_id),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= old_vault_id.wrapped(1000);
                new_vault.issued += old_vault_id.wrapped(1000);
                old_vault.to_be_redeemed -= old_vault_id.wrapped(1000);
                old_vault.issued -= old_vault_id.wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&DEFAULT_GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}
