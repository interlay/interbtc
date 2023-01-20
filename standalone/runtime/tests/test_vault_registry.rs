mod mock;

use currency::Amount;
use mock::{assert_eq, *};

use crate::{
    loans_testing_utils::activate_lending_and_mint,
    mock::issue_testing_utils::{execute_issue, request_issue},
};

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

fn test_with<R>(execute: impl Fn(VaultId) -> R) {
    let test_with = |currency_id, wrapped_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies().filter(|c| !c.is_lend_token()) {
                assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            }
            if wrapped_id != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_id, FixedU128::one()));
            }
            activate_lending_and_mint(Token(DOT), LendToken(1));
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
    test_with(ForeignAsset(1), Token(IBTC));
    test_with(LendToken(1), Token(IBTC));
}

fn deposit_collateral_and_issue(vault_id: VaultId) {
    let new_collateral = 10_000;
    assert_ok!(RuntimeCall::Nomination(NominationCall::deposit_collateral {
        vault_id: vault_id.clone(),
        amount: new_collateral,
    })
    .dispatch(origin_of(account_of(VAULT))));

    let (issue_id, _) = request_issue(&vault_id, vault_id.wrapped(4_000));
    execute_issue(issue_id);
}

mod deposit_collateral_test {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_vault_registry_deposit_collateral_below_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = Amount::new(1_000, currency_id);

            assert_ok!(RuntimeCall::Nomination(NominationCall::deposit_collateral {
                vault_id: vault_id.clone(),
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

            assert_ok!(RuntimeCall::Nomination(NominationCall::deposit_collateral {
                vault_id: vault_id.clone(),
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
                RuntimeCall::Nomination(NominationCall::deposit_collateral {
                    vault_id: vault_id.clone(),
                    amount: amount
                })
                .dispatch(origin_of(account_of(VAULT))),
                TokensError::BalanceTooLow
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_using_locked_tokens_fails() {
        ExtBuilder::build().execute_with(|| {
            let vault_id = VaultId::new(account_of(VAULT), DEFAULT_NATIVE_CURRENCY, DEFAULT_WRAPPED_CURRENCY);

            let currency_id = vault_id.collateral_currency();

            let amount_1 = 1000_000_000_000_000;

            // Mint lendTokens so that force-setting vault state doesn't fail
            activate_lending_and_mint(Token(DOT), LendToken(1));
            let mut vault_data = default_vault_state(&vault_id);
            *vault_data.free_balance.get_mut(&currency_id).unwrap() = Amount::new(amount_1, currency_id);
            CoreVaultData::force_to(&vault_id, vault_data);

            let q = currency::get_free_balance::<Runtime>(currency_id, &vault_id.account_id);
            assert_eq!(q.amount(), amount_1);

            let span = <Runtime as escrow::Config>::Span::get();
            let current_height = SystemPallet::block_number();

            assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
                amount: amount_1 / 2,
                unlock_height: current_height + span
            })
            .dispatch(origin_of(vault_id.account_id.clone())));

            assert_noop!(
                RuntimeCall::Nomination(NominationCall::deposit_collateral {
                    vault_id: vault_id.clone(),
                    amount: amount_1
                })
                .dispatch(origin_of(vault_id.account_id.clone())),
                TokensError::LiquidityRestrictions
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
                RuntimeCall::Nomination(NominationCall::deposit_collateral {
                    vault_id: vault_id.clone(),
                    amount: remaining + 1
                })
                .dispatch(origin_of(account_of(VAULT))),
                VaultRegistryError::CurrencyCeilingExceeded
            );

            assert_ok!(RuntimeCall::Nomination(NominationCall::deposit_collateral {
                vault_id: vault_id.clone(),
                amount: remaining,
            })
            .dispatch(origin_of(account_of(VAULT))));
        });
    }
}
mod withdraw_collateral_test {
    use interbtc_runtime_standalone::UnsignedFixedPoint;

    use super::{assert_eq, *};

    fn required_collateral(vault_id: VaultId) -> Amount<Runtime> {
        VaultRegistryPallet::get_required_collateral_for_vault(vault_id).unwrap()
    }

    #[test]
    fn integration_test_vault_registry_withdraw_collateral_below_capacity_succeeds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = Amount::new(1_000, currency_id);

            assert_ok!(RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                vault_id: vault_id.clone(),
                amount: amount.amount(),
                index: None,
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

            assert_ok!(RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                vault_id: vault_id.clone(),
                index: None,
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
                RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                    vault_id: vault_id.clone(),
                    index: None,
                    amount: amount
                })
                .dispatch(origin_of(account_of(VAULT))),
                NominationError::CannotWithdrawCollateral
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_withdraw_collateral_respects_custom_thresholds() {
        test_with(|vault_id| {
            let currency_id = vault_id.collateral_currency();
            let amount = default_vault_backing_collateral(currency_id) - required_collateral(vault_id.clone());

            assert_ok!(
                RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                    currency_pair: vault_id.currencies.clone(),
                    custom_threshold: UnsignedFixedPoint::checked_from_rational(20, 1),
                })
                .dispatch(origin_of(vault_id.account_id.clone()))
            );

            assert_err!(
                RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                    vault_id: vault_id.clone(),
                    index: None,
                    amount: amount.amount()
                })
                .dispatch(origin_of(account_of(VAULT))),
                NominationError::CannotWithdrawCollateral
            );

            assert_ok!(
                RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                    currency_pair: vault_id.currencies.clone(),
                    custom_threshold: None,
                })
                .dispatch(origin_of(vault_id.account_id.clone()))
            );

            assert_ok!(RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                vault_id: vault_id.clone(),
                index: None,
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
}

#[test]
fn integration_test_vault_registry_undercollateralization_liquidation() {
    test_with(|vault_id| {
        let currency_id = vault_id.collateral_currency();
        let vault_data = default_vault_state(&vault_id);
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

                vault.griefing_collateral -= DEFAULT_VAULT_REPLACE_COLLATERAL;
                vault.replace_collateral -= DEFAULT_VAULT_REPLACE_COLLATERAL;
                vault.to_be_replaced = vault_id.wrapped(0);
                vault.issued = vault_id.wrapped(0);
                vault.to_be_issued = vault_id.wrapped(0);
                vault.backing_collateral = Amount::new(0, currency_id);
                vault.liquidated_collateral =
                    default_vault_backing_collateral(currency_id) - liquidation_vault.collateral;
                vault.status = VaultStatus::Liquidated;
                *vault
                    .free_balance
                    .get_mut(&vault_data.replace_collateral.currency())
                    .unwrap() += vault_data.replace_collateral;
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

#[test]
fn integration_test_vault_registry_cannot_recover_active_vault() {
    test_with(|vault_id| {
        assert_noop!(
            RuntimeCall::VaultRegistry(VaultRegistryCall::recover_vault_id {
                currency_pair: vault_id.currencies.clone(),
            })
            .dispatch(origin_of(account_of(VAULT))),
            VaultRegistryError::VaultNotRecoverable
        );
    });
}

#[test]
fn integration_test_vault_registry_nonexistent_vault_cannot_be_recovered() {
    test_with(|vault_id| {
        assert_noop!(
            RuntimeCall::VaultRegistry(VaultRegistryCall::recover_vault_id {
                currency_pair: vault_id.currencies.clone(),
            })
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::VaultNotFound
        );
    });
}

#[test]
fn integration_test_vault_registry_liquidation_recovery_fails() {
    test_with(|vault_id| {
        liquidate_vault(&vault_id);
        // `to_be_redeemed` tokens are non-zero
        assert_err!(
            RuntimeCall::VaultRegistry(VaultRegistryCall::recover_vault_id {
                currency_pair: vault_id.currencies.clone(),
            })
            .dispatch(origin_of(account_of(VAULT))),
            VaultRegistryError::VaultNotRecoverable
        );
    });
}

fn default_liquidation_recovery_vault(vault_id: &VaultId) -> CoreVaultData {
    let mut vault_data = default_vault_state(&vault_id);
    vault_data.to_be_redeemed = vault_id.wrapped(0);
    vault_data.to_be_replaced = vault_id.wrapped(0);
    vault_data.replace_collateral = vault_data.replace_collateral.with_amount(|_| 0);
    vault_data
}

#[test]
fn integration_test_vault_registry_liquidation_recovery_works() {
    test_with(|vault_id| {
        let vault_data = default_liquidation_recovery_vault(&vault_id);
        CoreVaultData::force_to(&vault_id, vault_data.clone());
        liquidate_vault(&vault_id);

        let pre_recovery_state = ParachainState::get(&vault_id);
        assert_ok!(RuntimeCall::VaultRegistry(VaultRegistryCall::recover_vault_id {
            currency_pair: vault_id.currencies.clone(),
        })
        .dispatch(origin_of(account_of(VAULT))));

        VaultRegistryPallet::collateral_integrity_check();

        assert_eq!(
            ParachainState::get(&vault_id),
            pre_recovery_state.with_changes(|_, vault, _, _| {
                vault.status = VaultStatus::Active(true);
            })
        );
        deposit_collateral_and_issue(vault_id);
    });
}
