mod mock;

use currency::Amount;
use mock::{nomination_testing_utils::*, *};
use sp_runtime::traits::{CheckedDiv, CheckedSub};

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
                currency_id,
                FixedU128::one()
            ));
            UserData::force_to(USER, default_user_state());
            CoreVaultData::force_to(VAULT, default_vault_state(currency_id));
            execute(currency_id)
        })
    };
    test_with(CurrencyId::DOT);
    test_with(CurrencyId::KSM);
}

fn test_with_nomination_enabled<R>(execute: impl Fn(CurrencyId) -> R) {
    test_with(|currency_id| {
        enable_nomination();
        execute(currency_id)
    })
}

fn test_with_nomination_enabled_and_vault_opted_in<R>(execute: impl Fn(CurrencyId) -> R) {
    test_with_nomination_enabled(|currency_id| {
        assert_nomination_opt_in(VAULT);
        execute(currency_id)
    })
}

fn default_nomination(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_NOMINATION, currency_id)
}

mod spec_based_tests {
    use super::*;
    use sp_runtime::DispatchError;

    #[test]
    fn integration_test_enable_nomination() {
        // PRECONDITION: The calling account MUST be root or the function MUST be called from a passed governance
        // referendum. POSTCONDITION: The `NominationEnabled` scalar MUST be set to the value of the `enabled`
        // parameter.
        test_with(|_currency_id| {
            assert_noop!(
                Call::Nomination(NominationCall::set_nomination_enabled(true)).dispatch(origin_of(account_of(CAROL))),
                DispatchError::BadOrigin
            );
            let mut nomination_enabled = true;
            assert_ok!(
                Call::Nomination(NominationCall::set_nomination_enabled(nomination_enabled))
                    .dispatch(<Runtime as frame_system::Config>::Origin::root())
            );
            assert_eq!(NominationPallet::is_nomination_enabled(), nomination_enabled);
            nomination_enabled = false;
            assert_ok!(
                Call::Nomination(NominationCall::set_nomination_enabled(nomination_enabled))
                    .dispatch(<Runtime as frame_system::Config>::Origin::root())
            );
            assert_eq!(NominationPallet::is_nomination_enabled(), nomination_enabled);
        })
    }

    fn nomination_with_non_running_status_fails(status: StatusCode) {
        SecurityPallet::set_status(status);
        assert_noop!(
            Call::Nomination(NominationCall::opt_in_to_nomination()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
        assert_noop!(
            Call::Nomination(NominationCall::opt_out_of_nomination()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
        assert_noop!(
            Call::Nomination(NominationCall::deposit_collateral(account_of(BOB), 100))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
        assert_noop!(
            Call::Nomination(NominationCall::withdraw_collateral(account_of(BOB), 100))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    }

    #[test]
    fn integration_test_nomination_with_parachain_shutdown_status_fails() {
        // Checked PRECONDITION: The BTC Parachain status in the Security component be `RUNNING:0`.
        test_with(|_currency_id| {
            nomination_with_non_running_status_fails(StatusCode::Shutdown);
            nomination_with_non_running_status_fails(StatusCode::Error);
        });
    }

    #[test]
    fn integration_test_opt_in() {
        // PRECONDITIONS:
        //   - The BTC Parachain status in the Security component MUST be `RUNNING:0`.
        //   - A Vault with id `vaultId` MUST be registered.
        //   - The Vault MUST NOT be opted in.
        // POSTCONDITION: The Vault MUST be allowed to receive nominated collateral.
        test_with(|_| {
            assert_noop!(
                Call::Nomination(NominationCall::set_nomination_enabled(true)).dispatch(origin_of(account_of(CAROL))),
                DispatchError::BadOrigin
            );
            let mut nomination_enabled = true;
            assert_ok!(
                Call::Nomination(NominationCall::set_nomination_enabled(nomination_enabled))
                    .dispatch(<Runtime as frame_system::Config>::Origin::root())
            );
            assert_eq!(NominationPallet::is_nomination_enabled(), nomination_enabled);
            nomination_enabled = false;
            assert_ok!(
                Call::Nomination(NominationCall::set_nomination_enabled(nomination_enabled))
                    .dispatch(<Runtime as frame_system::Config>::Origin::root())
            );
            assert_eq!(NominationPallet::is_nomination_enabled(), nomination_enabled);
        })
    }

    #[test]
    fn integration_test_opt_out_preconditions() {
        // PRECONDITIONS:
        //   - A Vault with id `vaultId` MUST be registered.
        //   - A Vault with id `vaultId` MUST exist in the Vaults mapping.
        test_with(|_| {
            assert_noop!(nomination_opt_out(USER), NominationError::VaultNotOptedInToNomination);
            assert_noop!(nomination_opt_out(VAULT), NominationError::VaultNotOptedInToNomination);
        })
    }

    #[test]
    fn integration_test_opt_out_postconditions() {
        // POSTCONDITIONS:
        //   - The Vault MUST be removed from the `Vaults` mapping.
        //   - The Vault MUST remain above the secure collateralization threshold.
        //   - `get_total_nominated_collateral(vault_id)` must return zero.
        //   - For all nominators, `get_nominator_collateral(vault_id, user_id)` must return zero.
        //   - Staking pallet `nonce` must be incremented by one.
        //   - `compute_reward_at_index(nonce - 1, INTERBTC, vault_id, user_id)` in the Staking pallet must be equal to
        //     the user’s nomination just before the vault opted out.
        test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
            assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
            assert_eq!(
                NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
                default_nomination(currency_id)
            );
            assert_eq!(
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
                default_nomination(currency_id)
            );
            assert_eq!(
                VaultRewardsPallet::compute_reward(INTERBTC, &account_of(USER)).unwrap(),
                0
            );
            assert_eq!(NominationPallet::is_opted_in(&account_of(VAULT)).unwrap(), true);
            assert_ok!(nomination_opt_out(VAULT));
            assert_eq!(NominationPallet::is_opted_in(&account_of(VAULT)).unwrap(), false);
            assert_eq!(
                VaultRegistryPallet::get_collateralization_from_vault(account_of(VAULT), false).unwrap()
                    >= VaultRegistryPallet::secure_collateral_threshold(DEFAULT_TESTING_CURRENCY).unwrap(),
                true
            );
            assert_eq!(
                NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
                Amount::new(0, currency_id)
            );
            assert_eq!(
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
                Amount::new(0, currency_id)
            );
            let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
            assert_eq!(nonce, 1);
            assert_eq!(
                VaultStakingPallet::compute_reward_at_index(nonce - 1, INTERBTC, &account_of(VAULT), &account_of(USER))
                    .unwrap(),
                DEFAULT_NOMINATION as i128
            );
        })
    }

    #[test]
    fn integration_test_deposit_collateral_preconditions() {
        // PRECONDITIONS:
        //   - The global nomination flag MUST be enabled.
        //   - A Vault with id `vaultId` MUST be registered.
        //   - A Vault with id `vaultId` MUST exist in the `Vaults` mapping.
        //   - The Vault MUST remain below the max nomination ratio.
        test_with(|_| {
            assert_noop!(
                Call::Nomination(NominationCall::deposit_collateral(
                    account_of(VAULT),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNominationDisabled
            );
            enable_nomination();
            assert_noop!(
                Call::Nomination(NominationCall::deposit_collateral(
                    account_of(CAROL),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNotOptedInToNomination
            );
            assert_noop!(
                Call::Nomination(NominationCall::deposit_collateral(
                    account_of(VAULT),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNotOptedInToNomination
            );
            assert_nomination_opt_in(VAULT);
            assert_noop!(
                Call::Nomination(NominationCall::deposit_collateral(
                    account_of(VAULT),
                    100000000000000000000000
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::DepositViolatesMaxNominationRatio
            );
            assert_ok!(Call::Nomination(NominationCall::deposit_collateral(
                account_of(VAULT),
                DEFAULT_NOMINATION
            ))
            .dispatch(origin_of(account_of(USER))));
        })
    }

    #[test]
    fn integration_test_deposit_collateral_postconditions() {
        // POSTCONDITIONS:
        //   - The Vault’s collateral MUST increase by the amount nominated.
        //   - The Nominator’s balance MUST decrease by the amount nominated.
        test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
            let vault_backing_collateral_before_nomination =
                VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
            let user_collateral_before_nomination =
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap();
            assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
            let vault_backing_collateral_after_nomination =
                VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
            let user_collateral_after_nomination =
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap();
            assert_eq!(
                vault_backing_collateral_after_nomination,
                vault_backing_collateral_before_nomination + default_nomination(currency_id)
            );
            assert_eq!(
                user_collateral_after_nomination,
                user_collateral_before_nomination + default_nomination(currency_id)
            );
        })
    }

    #[test]
    fn integration_test_withdraw_collateral_preconditions() {
        // PRECONDITIONS:
        //   - The global nomination flag MUST be enabled.
        //   - A Vault with id vaultId MUST be registered.
        //   - A Vault with id vaultId MUST exist in the Vaults mapping.
        //   - Nominator MUST have nominated at least amount.
        test_with(|_| {
            assert_noop!(
                Call::Nomination(NominationCall::withdraw_collateral(
                    account_of(VAULT),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNominationDisabled
            );
            enable_nomination();
            assert_noop!(
                Call::Nomination(NominationCall::withdraw_collateral(
                    account_of(CAROL),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNotOptedInToNomination
            );
            assert_noop!(
                Call::Nomination(NominationCall::withdraw_collateral(
                    account_of(VAULT),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::VaultNotOptedInToNomination
            );
            assert_nomination_opt_in(VAULT);
            assert_noop!(
                Call::Nomination(NominationCall::withdraw_collateral(
                    account_of(VAULT),
                    DEFAULT_BACKING_COLLATERAL
                ))
                .dispatch(origin_of(account_of(USER))),
                NominationError::InsufficientCollateral
            );
        })
    }

    #[test]
    fn integration_test_withdraw_collateral_preconditions_collateralization() {
        // PRECONDITION: The Vault MUST remain above the secure collateralization threshold.
        test_with_nomination_enabled(|currency_id| {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(750000))
                .dispatch(origin_of(account_of(VAULT))));
            assert_nomination_opt_in(VAULT);
            assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
            assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
                currency_id,
                FixedU128::checked_from_integer(3).unwrap()
            ));
            assert_noop!(
                withdraw_nominator_collateral(USER, VAULT, default_nomination(currency_id)),
                NominationError::InsufficientCollateral
            );
        });
    }

    #[test]
    fn integration_test_withdraw_collateral_postconditions() {
        // POSTCONDITIONS:
        //   - The Vault’s collateral MUST decrease by the amount nominated.
        //   - The Nominator’s balance MUST increase by the amount nominated.
        test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
            assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
            let vault_backing_collateral_before_withdrawal =
                VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
            let user_collateral_before_withdrawal =
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap();
            withdraw_nominator_collateral(USER, VAULT, default_nomination(currency_id)).unwrap();
            let vault_backing_collateral_after_withdrawal =
                VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
            let user_collateral_after_withdrawal =
                NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap();
            assert_eq!(
                vault_backing_collateral_after_withdrawal,
                vault_backing_collateral_before_withdrawal - default_nomination(currency_id)
            );
            assert_eq!(
                user_collateral_after_withdrawal,
                user_collateral_before_withdrawal - default_nomination(currency_id)
            );
        });
    }
}

#[test]
fn integration_test_regular_vaults_are_not_opted_in_to_nomination() {
    test_with_nomination_enabled(|currency_id| {
        assert_register_vault(currency_id, CAROL);
        assert_eq!(NominationPallet::is_opted_in(&account_of(CAROL)).unwrap(), false);
    })
}

#[test]
fn integration_test_vaults_can_opt_in() {
    test_with_nomination_enabled(|_currency_id| {
        assert_nomination_opt_in(VAULT);
        assert_eq!(NominationPallet::is_opted_in(&account_of(VAULT)).unwrap(), true);
    });
}

#[test]
fn integration_test_vaults_cannot_opt_in_if_disabled() {
    test_with(|_currency_id| {
        assert_noop!(nomination_opt_in(VAULT), NominationError::VaultNominationDisabled);
    });
}

#[test]
fn integration_test_vaults_can_still_opt_out_if_disabled() {
    test_with_nomination_enabled_and_vault_opted_in(|_currency_id| {
        disable_nomination();
        assert_ok!(nomination_opt_out(VAULT));
    });
}

#[test]
fn integration_test_cannot_nominate_if_not_opted_in() {
    test_with_nomination_enabled(|_currency_id| {
        assert_noop!(
            Call::Nomination(NominationCall::deposit_collateral(
                account_of(VAULT),
                DEFAULT_BACKING_COLLATERAL
            ))
            .dispatch(origin_of(account_of(USER))),
            NominationError::VaultNotOptedInToNomination
        );
    });
}

#[test]
fn integration_test_can_nominate_if_opted_in() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        let nominator_collateral = get_nominator_collateral(VAULT, USER);
        assert_eq!(nominator_collateral, default_nomination(currency_id));
        assert_total_nominated_collateral_is(VAULT, default_nomination(currency_id));
    });
}

#[test]
fn integration_test_vaults_cannot_withdraw_nominated_collateral() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        assert_noop!(
            withdraw_vault_collateral(VAULT, default_backing_collateral(currency_id).with_amount(|x| x + 1)),
            VaultRegistryError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_nominated_collateral_cannot_exceed_max_nomination_ratio() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_noop!(
            nominate_collateral(VAULT, USER, default_backing_collateral(currency_id)),
            NominationError::DepositViolatesMaxNominationRatio
        );
    });
}

#[test]
fn integration_test_nominated_collateral_prevents_replace_requests() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, DEFAULT_BACKING_COLLATERAL))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultHasEnabledNomination
        );
    });
}

#[test]
fn integration_test_vaults_with_zero_nomination_cannot_request_replacement() {
    test_with_nomination_enabled_and_vault_opted_in(|_currency_id| {
        let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
        let griefing_collateral = 200;
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(amount.amount(), griefing_collateral))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultHasEnabledNomination
        );
    });
}

#[test]
fn integration_test_nomination_increases_issuable_tokens() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        let issuance_capacity_before_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_before_nomination, wrapped(556666));
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        let issuance_capacity_after_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_after_nomination, wrapped(570000));
    });
}

#[test]
fn integration_test_nominator_withdrawal_request_reduces_issuable_tokens() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        let issuance_capacity_before_withdrawal_request =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_ok!(withdraw_nominator_collateral(
            USER,
            VAULT,
            default_nomination(currency_id)
        ));
        let issuance_capacity_after_withdrawal_request =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_before_withdrawal_request, wrapped(570000));
        assert_eq!(issuance_capacity_after_withdrawal_request, wrapped(556666));
    });
}

#[test]
fn integration_test_nominator_withdrawal_below_collateralization_threshold_fails() {
    test_with_nomination_enabled(|currency_id| {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(750000)).dispatch(origin_of(account_of(VAULT)))
        );
        assert_nomination_opt_in(VAULT);
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
            currency_id,
            FixedU128::checked_from_integer(3).unwrap()
        ));
        assert_noop!(
            withdraw_nominator_collateral(USER, VAULT, default_nomination(currency_id)),
            NominationError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_nomination_fee_distribution() {
    test_with_nomination_enabled(|_currency_id| {});
}

#[test]
fn integration_test_maximum_nomination_ratio_calculation() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        let expected_nomination_ratio = FixedU128::checked_from_rational(150, 100)
            .unwrap()
            .checked_div(&FixedU128::checked_from_rational(135, 100).unwrap())
            .unwrap()
            .checked_sub(&FixedU128::one())
            .unwrap();
        assert_eq!(
            VaultRegistryPallet::get_max_nomination_ratio(currency_id).unwrap(),
            expected_nomination_ratio
        );
    })
}

#[test]
fn integration_test_vault_opt_out_must_refund_nomination() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        assert_eq!(
            NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
            default_nomination(currency_id)
        );
        assert_eq!(
            NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
            default_nomination(currency_id)
        );
        assert_eq!(
            VaultRewardsPallet::compute_reward(INTERBTC, &account_of(USER)).unwrap(),
            0
        );
        assert_ok!(nomination_opt_out(VAULT));
        assert_eq!(
            NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
            Amount::new(0, currency_id)
        );
        assert_eq!(
            NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
            Amount::new(0, currency_id)
        );
        let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
        assert_eq!(nonce, 1);
        assert_eq!(
            VaultStakingPallet::compute_reward_at_index(nonce - 1, INTERBTC, &account_of(VAULT), &account_of(USER))
                .unwrap(),
            DEFAULT_NOMINATION as i128
        );
    })
}

#[test]
fn integration_test_banning_a_vault_does_not_force_refund() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        VaultRegistryPallet::ban_vault(account_of(VAULT)).unwrap();
        let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
        assert_eq!(nonce, 0);
    })
}

#[test]
fn integration_test_liquidating_a_vault_does_not_force_refund() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));
        VaultRegistryPallet::liquidate_vault(&account_of(VAULT)).unwrap();
        let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
        assert_eq!(nonce, 0);
    })
}

#[test]
fn integration_test_vault_withdrawal_cannot_exceed_max_nomination_taio() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        let max_nomination =
            VaultRegistryPallet::get_max_nominatable_collateral(&default_backing_collateral(currency_id)).unwrap();
        assert_nominate_collateral(VAULT, USER, max_nomination);

        // Need to withdraw 10 units to account for rounding errors
        assert_noop!(
            withdraw_vault_collateral(VAULT, Amount::new(10, currency_id)),
            VaultRegistryError::MaxNominationRatioViolation
        );
    })
}

#[test]
fn integration_test_rewards_are_preserved_on_collateral_withdrawal() {
    test_with_nomination_enabled_and_vault_opted_in(|currency_id| {
        let mut user_data = default_user_state();
        (*user_data.balances.get_mut(&currency_id).unwrap()).free =
            default_user_free_balance(currency_id) + default_nomination(currency_id);

        UserData::force_to(USER, user_data);
        assert_nominate_collateral(VAULT, USER, default_nomination(currency_id));

        let (issue_id, _) = issue_testing_utils::request_issue(currency_id, wrapped(100000));
        issue_testing_utils::execute_issue(issue_id);
        FeePallet::withdraw_all_vault_rewards(&account_of(VAULT)).unwrap();
        let reward_before_nomination_withdrawal =
            VaultStakingPallet::compute_reward(INTERBTC, &account_of(VAULT), &account_of(USER)).unwrap();
        assert_eq!(reward_before_nomination_withdrawal > 0, true);
        assert_ok!(withdraw_nominator_collateral(
            USER,
            VAULT,
            default_nomination(currency_id)
        ));
        assert_eq!(
            VaultStakingPallet::compute_reward(INTERBTC, &account_of(VAULT), &account_of(USER)).unwrap(),
            reward_before_nomination_withdrawal
        );
    })
}
