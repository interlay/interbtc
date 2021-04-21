mod mock;

use mock::{nomination_testing_utils::*, *};
use vault_registry::VaultStatus;

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());
        NominationPallet::set_max_nomination_ratio(FixedU128::checked_from_rational(50, 100).unwrap()).unwrap();
        NominationPallet::set_max_nominators_per_operator(1).unwrap();
        NominationPallet::set_operator_unbonding_period(DEFAULT_OPERATOR_UNBONDING_PERIOD).unwrap();
        NominationPallet::set_nominator_unbonding_period(DEFAULT_NOMINATOR_UNBONDING_PERIOD).unwrap();

        execute()
    })
}

fn test_with_nomination_enabled<R>(execute: impl FnOnce() -> R) -> R {
    test_with(|| {
        enable_nomination();
        execute()
    })
}

fn test_with_nomination_enabled_and_operator_registered<R>(execute: impl FnOnce() -> R) -> R {
    test_with_nomination_enabled(|| {
        assert_register_operator(VAULT);
        execute()
    })
}

#[test]
fn integration_test_vaults_can_opt_in() {
    test_with_nomination_enabled(|| {
        assert_register_operator(VAULT);
        assert_eq!(NominationPallet::is_operator(&account_of(VAULT)).unwrap(), true);
    });
}

#[test]
fn integration_test_vaults_cannot_opt_in_if_disabled() {
    test_with(|| {
        assert_noop!(register_operator(VAULT), NominationError::VaultNominationDisabled);
    });
}

#[test]
fn integration_test_operators_can_still_opt_out_if_disabled() {
    test_with_nomination_enabled_and_operator_registered(|| {
        disable_nomination();
        assert_ok!(deregister_operator(VAULT));
    });
}

#[test]
fn integration_test_operators_with_nonzero_nomination_can_force_opt_opt() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let user_collateral_balance_before_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        assert_eq!(user_collateral_balance_before_force_refund, 900000);
        assert_ok!(deregister_operator(VAULT));
        let user_collateral_balance_after_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        assert_eq!(user_collateral_balance_after_force_refund, 900000 + DEFAULT_NOMINATION);
    });
}

#[test]
fn integration_test_non_operators_cannot_have_collateral_nominated() {
    test_with_nomination_enabled(|| {
        assert_noop!(
            Call::Nomination(NominationCall::deposit_nominated_collateral(
                account_of(VAULT),
                DEFAULT_BACKING_COLLATERAL
            ))
            .dispatch(origin_of(account_of(USER))),
            NominationError::VaultNotOptedInToNomination
        );
    });
}

#[test]
fn integration_test_operators_can_have_collateral_nominated() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let nominator_collateral = get_nominator_collateral();
        assert_eq!(nominator_collateral, DEFAULT_NOMINATION);
        assert_total_nominated_collateral_is(VAULT, DEFAULT_NOMINATION);
    });
}

#[test]
fn integration_test_operators_cannot_withdraw_nominated_collateral() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_noop!(
            request_operator_collateral_withdrawal(VAULT, DEFAULT_BACKING_COLLATERAL + 1),
            NominationError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_nominated_collateral_cannot_exceed_max_nomination_ratio() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_noop!(
            nominate_collateral(USER, VAULT, DEFAULT_BACKING_COLLATERAL),
            NominationError::DepositViolatesMaxNominationRatio
        );
    });
}

#[test]
fn integration_test_nominated_collateral_prevents_replace_requests() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, DEFAULT_BACKING_COLLATERAL))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultUsesNominatedCollateral
        );
    });
}

#[test]
fn integration_test_operators_with_zero_nomination_can_request_replacement() {
    test_with_nomination_enabled_and_operator_registered(|| {
        let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
        let griefing_collateral = 200;
        assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
            .dispatch(origin_of(account_of(VAULT))));
    });
}

#[test]
fn integration_test_nomination_increases_issuable_tokens() {
    test_with_nomination_enabled_and_operator_registered(|| {
        let issuance_capacity_before_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_before_nomination, 556666);
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let issuance_capacity_after_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_after_nomination, 623333);
    });
}

#[test]
fn integration_test_nominator_withdrawal_request_reduces_issuable_tokens() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let issuance_capacity_before_withdrawal_request =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_ok!(request_nominator_collateral_withdrawal(USER, VAULT, DEFAULT_NOMINATION));
        let issuance_capacity_after_withdrawal_request =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_before_withdrawal_request, 623333);
        assert_eq!(issuance_capacity_after_withdrawal_request, 556666);
    });
}

#[test]
fn integration_test_operator_cannot_withdraw_directly() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_noop!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(100000000))
                .dispatch(origin_of(account_of(VAULT))),
            VaultRegistryError::NominationOperatorCannotWithdrawDirectly
        );
    });
}

#[test]
fn integration_test_nominator_withdrawal_below_collateralization_threshold_fails() {
    test_with_nomination_enabled(|| {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(800000)).dispatch(origin_of(account_of(VAULT)))
        );
        assert_register_operator(VAULT);
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
            FixedU128::checked_from_integer(3).unwrap()
        ));
        assert_noop!(
            request_nominator_collateral_withdrawal(USER, VAULT, 10),
            VaultRegistryError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_operator_withdrawal_can_force_refunds_nominators() {
    test_with_nomination_enabled(|| {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(800000)).dispatch(origin_of(account_of(VAULT)))
        );
        assert_register_operator(VAULT);
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let user_collateral_balance_before_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        let amount_to_withdraw = 10000;
        assert_ok!(request_operator_collateral_withdrawal(VAULT, amount_to_withdraw));
        let user_collateral_balance_after_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        let max_nomination_ratio = NominationPallet::get_max_nomination_ratio();
        let expected_force_liquidation = FeePallet::dot_for(amount_to_withdraw, max_nomination_ratio).unwrap();
        assert_eq!(
            user_collateral_balance_before_force_refund + expected_force_liquidation,
            user_collateral_balance_after_force_refund
        );
    });
}

#[test]
fn integration_test_liquidating_operators_for_low_collateralization_also_liquidates_nominators() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let expected_slashed_amount = 900000;
        let vault_backing_collateral_before_liquidation =
            VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
        let nominator_collateral_before_liquidation = get_nominator_collateral();
        drop_exchange_rate_and_liquidate_operator(VAULT);
        let vault_backing_collateral_after_liquidation =
            VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
        let nominator_collateral_after_liquidation = get_nominator_collateral();
        assert_eq!(
            vault_backing_collateral_before_liquidation,
            vault_backing_collateral_after_liquidation + expected_slashed_amount
        );
        let expected_nominator_slashed_amount = nominator_collateral_before_liquidation * expected_slashed_amount
            / vault_backing_collateral_before_liquidation;
        assert_eq!(
            nominator_collateral_after_liquidation,
            DEFAULT_NOMINATION - expected_nominator_slashed_amount
        );
    });
}

#[test]
fn integration_test_liquidating_operators_for_stealing_minimizes_nominator_slashing() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let expected_slashed_amount = 900000;

        let user_collateral_balance_before_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        let vault_backing_collateral_before_liquidation =
            VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
        let nominator_collateral_before_liquidation = get_nominator_collateral();

        // Theft liquidation
        NominationPallet::liquidate_operator_with_status(&account_of(VAULT), VaultStatus::CommittedTheft).unwrap();

        let user_collateral_balance_after_force_refund = CollateralPallet::get_balance_from_account(&account_of(USER));
        let vault_backing_collateral_after_liquidation =
            VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
        let nominator_collateral_after_liquidation = get_nominator_collateral();

        // The operator was completely liquidated
        assert_eq!(vault_backing_collateral_after_liquidation, 0);

        // The liquidator was force-refunded
        assert_eq!(nominator_collateral_after_liquidation, 0);

        // The remaining user collateral (which was force refunded) is greater compared to non-theft liquidation
        let non_theft_nominator_slashed_amount = nominator_collateral_before_liquidation * expected_slashed_amount
            / vault_backing_collateral_before_liquidation;
        let force_refunded_user_collateral =
            user_collateral_balance_after_force_refund - user_collateral_balance_before_force_refund;
        assert_eq!(
            force_refunded_user_collateral > DEFAULT_NOMINATION - non_theft_nominator_slashed_amount,
            true
        );
    });
}

#[test]
fn integration_test_nominator_withdrawal_after_unbonding_period_succeeds() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(request_nominator_collateral_withdrawal(USER, VAULT, 10));
        SecurityPallet::set_active_block_number(
            SecurityPallet::active_block_number() + DEFAULT_NOMINATOR_UNBONDING_PERIOD,
        );
        assert_ok!(execute_nominator_collateral_withdrawal(USER, VAULT));
    });
}

#[test]
fn integration_test_nominator_withdrawal_before_unbonding_period_fails() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(request_nominator_collateral_withdrawal(USER, VAULT, 10));
        assert_noop!(
            execute_nominator_collateral_withdrawal(USER, VAULT),
            NominationError::NoMaturedCollateral
        );
    });
}

#[test]
fn integration_test_operator_withdrawal_after_unbonding_period_succeeds() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(request_operator_collateral_withdrawal(VAULT, 10));
        SecurityPallet::set_active_block_number(
            SecurityPallet::active_block_number() + DEFAULT_OPERATOR_UNBONDING_PERIOD,
        );
        assert_ok!(execute_operator_collateral_withdrawal(VAULT));
    });
}

#[test]
fn integration_test_operator_withdrawal_before_unbonding_period_fails() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(request_operator_collateral_withdrawal(VAULT, 10));
        assert_noop!(
            execute_operator_collateral_withdrawal(VAULT),
            NominationError::NoMaturedCollateral
        );
    });
}

#[test]
fn integration_test_attempting_to_register_too_many_nominators_fails() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(ALICE, VAULT, DEFAULT_NOMINATION);
        assert_noop!(
            nominate_collateral(CAROL, VAULT, DEFAULT_NOMINATION),
            NominationError::OperatorHasTooManyNominators
        );
    });
}

// #[test]
// fn test_withdrawal_request_can_be_cancelled() {
//     run_test(|| {})
// }

// #[test]
// fn test_nomination_fee_distribution() {
//     run_test(|| {})
// }

// #[test]
// fn test_banning_an_operator_force_refunds_as_much_nominated_collateral_as_possible() {
//     run_test(|| {})
// }

// #[test]
// fn test_maximum_nomination_ratio_calculation() {
//     run_test(|| {})
// }
