mod mock;

use currency::Amount;
use mock::{assert_eq, *};

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

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
            CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));

            execute(vault_id)
        });
    };
    test_with(Token(DOT), Token(KBTC));
    test_with(Token(KSM), Token(IBTC));
    test_with(Token(DOT), Token(IBTC));
}

mod deposit_collateral_test {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_vault_registry_deposit_collateral_below_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = Amount::new(1_000, currency_id);

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: amount.amount(),
            })
            .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(&vault_id),
                ParachainState::get_default(&vault_id).with_changes(|_, vault, _, _| {
                    vault.backing_collateral += amount;
                    *vault.free_balance.get_mut(&vault.collateral_currency()).unwrap() -= amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_at_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = default_vault_free_balance(currency_id);

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: amount.amount()
            })
            .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(&vault_id),
                ParachainState::get_default(&vault_id).with_changes(|_, vault, _, _| {
                    vault.backing_collateral += amount;
                    *vault.free_balance.get_mut(&vault.collateral_currency()).unwrap() -= amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_above_capacity_fails() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = default_vault_free_balance(currency_id).amount() + 1;

            assert_noop!(
                Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                    currency_pair: vault_id.currencies.clone(),
                    amount: amount
                })
                .dispatch(origin_of(account_of(VAULT))),
                TokensError::BalanceTooLow
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_respects_fund_limit() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let mut vault_data = CoreVaultData::vault(vault_id.clone());
            *vault_data.free_balance.get_mut(&currency_id).unwrap() = Amount::new(FUND_LIMIT_CEILING, currency_id);

            CoreVaultData::force_to(&vault_id, vault_data);
            let current = VaultRegistryPallet::get_total_user_vault_collateral(&vault_id.currencies).unwrap();
            let remaining = FUND_LIMIT_CEILING - current.amount();

            assert_noop!(
                Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                    currency_pair: vault_id.currencies.clone(),
                    amount: remaining + 1
                })
                .dispatch(origin_of(account_of(VAULT))),
                VaultRegistryError::CurrencyCeilingExceeded
            );

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: remaining,
            })
            .dispatch(origin_of(account_of(VAULT))));
        });
    }
}
mod withdraw_collateral_test {
    use super::{assert_eq, *};

    fn required_collateral(vault_id: VaultId) -> Amount<Runtime> {
        VaultRegistryPallet::get_required_collateral_for_vault(vault_id).unwrap()
    }

    #[test]
    fn integration_test_vault_registry_withdraw_collateral_below_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = Amount::new(1_000, currency_id);

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: amount.amount()
            })
            .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(&vault_id),
                ParachainState::get_default(&vault_id).with_changes(|_, vault, _, _| {
                    vault.backing_collateral -= amount;
                    *vault.free_balance.get_mut(&vault.collateral_currency()).unwrap() += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_withdraw_at_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = default_vault_backing_collateral(currency_id) - required_collateral(vault_id.clone());

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: amount.amount()
            })
            .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(&vault_id),
                ParachainState::get_default(&vault_id).with_changes(|_, vault, _, _| {
                    vault.backing_collateral -= amount;
                    *vault.free_balance.get_mut(&vault.collateral_currency()).unwrap() += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_withdraw_above_capacity_fails() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = default_vault_backing_collateral(currency_id).amount()
                - required_collateral(vault_id.clone()).amount()
                + 1;

            assert_noop!(
                Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
                    currency_pair: vault_id.currencies.clone(),
                    amount: amount
                })
                .dispatch(origin_of(account_of(VAULT))),
                VaultRegistryError::InsufficientCollateral
            );
        });
    }
}

#[test]
fn integration_test_vault_registry_with_parachain_shutdown_fails() {
    test_with(|vault_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::register_vault {
                currency_pair: vault_id.currencies.clone(),
                collateral: 0,
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::deposit_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: 0
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
                currency_pair: vault_id.currencies.clone(),
                amount: 0
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::register_public_key {
                public_key: Default::default()
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::register_address {
                currency_pair: vault_id.currencies.clone(),
                btc_address: Default::default()
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::accept_new_issues {
                currency_pair: vault_id.currencies.clone(),
                accept_new_issues: false
            })
            .dispatch(origin_of(account_of(VAULT))),
            SystemError::CallFiltered
        );
    });
}

#[test]
fn integration_test_vault_registry_undercollateralization_liquidation() {
    test_with(|vault_id| {
        let currency_id = vault_id.collateral_currency();
        liquidate_vault(&vault_id);

        assert_eq!(
            ParachainState::get(&vault_id),
            ParachainState::get_default(&vault_id).with_changes(|_, vault, liquidation_vault, _| {
                let liquidation_vault = liquidation_vault.with_currency(&vault_id.currencies);

                liquidation_vault.collateral = Amount::new(
                    (default_vault_backing_collateral(currency_id).amount()
                        * (DEFAULT_VAULT_ISSUED + DEFAULT_VAULT_TO_BE_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED).amount())
                        / (DEFAULT_VAULT_ISSUED + DEFAULT_VAULT_TO_BE_ISSUED).amount(),
                    currency_id,
                );
                liquidation_vault.to_be_issued = vault_id.wrapped(DEFAULT_VAULT_TO_BE_ISSUED.amount());
                liquidation_vault.issued = vault_id.wrapped(DEFAULT_VAULT_ISSUED.amount());
                liquidation_vault.to_be_redeemed = vault_id.wrapped(DEFAULT_VAULT_TO_BE_REDEEMED.amount());

                vault.issued = vault_id.wrapped(0);
                vault.to_be_issued = vault_id.wrapped(0);
                vault.backing_collateral = Amount::new(0, currency_id);
                vault.liquidated_collateral =
                    default_vault_backing_collateral(currency_id) - liquidation_vault.collateral;
            })
        );
    });
}

#[test]
fn integration_test_vault_registry_register_respects_fund_limit() {
    test_with(|vault_id| {
        let currency_id = vault_id.collateral_currency();
        let mut vault_data = CoreVaultData::vault(vault_id.clone());
        *vault_data.free_balance.get_mut(&currency_id).unwrap() = Amount::new(FUND_LIMIT_CEILING, currency_id);

        let mut user_data = default_user_state();
        (*user_data.balances.get_mut(&currency_id).unwrap()).free = Amount::new(FUND_LIMIT_CEILING + 1, currency_id);

        UserData::force_to(USER, user_data);
        let user_vault_id = VaultId {
            account_id: account_of(USER),
            ..vault_id.clone()
        };

        let current = VaultRegistryPallet::get_total_user_vault_collateral(&vault_id.currencies).unwrap();
        let remaining = Amount::new(FUND_LIMIT_CEILING, current.currency()) - current;

        // not asserting noop since this func registers a public key first
        assert_err!(
            get_register_vault_result(&user_vault_id, remaining.with_amount(|x| x + 1)),
            VaultRegistryError::CurrencyCeilingExceeded
        );

        assert_ok!(get_register_vault_result(&user_vault_id, remaining));
    });
}
