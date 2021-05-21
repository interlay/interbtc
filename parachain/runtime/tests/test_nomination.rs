mod mock;

use mock::{nomination_testing_utils::*, *};
use sp_runtime::traits::{CheckedDiv, CheckedSub};

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());
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
fn integration_test_regular_vaults_are_not_opted_in_to_nomination() {
    test_with_nomination_enabled(|| {
        assert_register_vault(CAROL);
        assert_eq!(NominationPallet::is_operator(&account_of(CAROL)).unwrap(), false);
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
        let nominator_collateral = get_nominator_collateral(USER, VAULT);
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
            ReplaceError::VaultIsNominationOperator
        );
    });
}

#[test]
fn integration_test_operators_with_zero_nomination_cannot_request_replacement() {
    test_with_nomination_enabled_and_operator_registered(|| {
        let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
        let griefing_collateral = 200;
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultIsNominationOperator
        );
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
        assert_eq!(issuance_capacity_after_nomination, 570000);
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
        assert_eq!(issuance_capacity_before_withdrawal_request, 570000);
        assert_eq!(issuance_capacity_after_withdrawal_request, 556666);
    });
}

#[test]
fn integration_test_nominator_withdrawal_below_collateralization_threshold_fails() {
    test_with_nomination_enabled(|| {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(750000)).dispatch(origin_of(account_of(VAULT)))
        );
        assert_register_operator(VAULT);
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
            FixedU128::checked_from_integer(3).unwrap()
        ));
        assert_noop!(
            request_nominator_collateral_withdrawal(USER, VAULT, DEFAULT_NOMINATION),
            NominationError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_slash_vault_and_nominator() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let expected_slashed_amount = 834545;
        let vault_backing_collateral_before_liquidation =
            VaultRegistryPallet::get_backing_collateral(&account_of(VAULT)).unwrap();
        let nominator_collateral_before_liquidation = get_nominator_collateral(USER, VAULT);
        let vault_collateral_before_liquidation = VaultRegistryPallet::compute_collateral(&account_of(VAULT)).unwrap();
        drop_exchange_rate_and_liquidate(VAULT);
        let expected_nominator_slashed_amount = nominator_collateral_before_liquidation * expected_slashed_amount
            / vault_backing_collateral_before_liquidation;
        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, liquidation_vault, _| {
                user.free_balance -= DEFAULT_NOMINATION;
                liquidation_vault.to_be_issued = DEFAULT_VAULT_TO_BE_ISSUED;
                liquidation_vault.issued = DEFAULT_VAULT_ISSUED;
                liquidation_vault.to_be_redeemed = DEFAULT_VAULT_TO_BE_REDEEMED;
                liquidation_vault.backing_collateral = expected_slashed_amount;
                vault.to_be_issued = 0;
                vault.issued = 0;
                vault.backing_collateral = vault.backing_collateral + DEFAULT_NOMINATION - expected_slashed_amount;
            })
        );
        let nominator_collateral_after_liquidation = get_nominator_collateral(USER, VAULT);
        let vault_collateral_after_liquidation = VaultRegistryPallet::compute_collateral(&account_of(VAULT)).unwrap();
        // Use this assert to print values
        assert_eq!(
            nominator_collateral_before_liquidation,
            nominator_collateral_after_liquidation
        );
        // Use this assert to print values
        assert_eq!(vault_collateral_before_liquidation, vault_collateral_after_liquidation);
        // Actual assert that should hold but doesn't
        assert_eq!(
            nominator_collateral_after_liquidation,
            DEFAULT_NOMINATION - expected_nominator_slashed_amount
        );
    });
}

// #[test]
// fn test_nomination_fee_distribution() {
//     run_test(|| {})
// }

// #[test]
// fn test_banning_an_operator_force_refunds_as_much_nominated_collateral_as_possible() {
//     run_test(|| {})
// }

#[test]
fn integration_test_maximum_nomination_ratio_calculation() {
    test_with_nomination_enabled_and_operator_registered(|| {
        let expected_nomination_ratio = FixedU128::checked_from_rational(150, 100)
            .unwrap()
            .checked_div(&FixedU128::checked_from_rational(135, 100).unwrap())
            .unwrap()
            .checked_sub(&FixedU128::one())
            .unwrap();
        assert_eq!(
            NominationPallet::get_max_nomination_ratio().unwrap(),
            expected_nomination_ratio
        );
    })
}
