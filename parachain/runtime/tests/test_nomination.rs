mod mock;

use mock::{nomination_testing_utils::*, *};
use std::collections::BTreeMap;
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
fn test_regular_vaults_are_not_opted_in_to_nomination() {
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
fn integration_test_operators_with_nonzero_nomination_can_force_opt_opt() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, _, operator| {
                user.free_balance -= DEFAULT_NOMINATION;
                vault.backing_collateral += DEFAULT_NOMINATION;

                let expected_nominator = CoreNominatorData {
                    collateral: DEFAULT_NOMINATION,
                    collateral_to_be_withdrawn: 0,
                };
                let mut expected_nominators = BTreeMap::<AccountId, CoreNominatorData>::new();
                expected_nominators.insert(account_of(USER), expected_nominator.clone());
                operator.nominators = expected_nominators;
                operator.total_nominated_collateral = expected_nominator.collateral;
            })
        );
        assert_ok!(deregister_operator(VAULT));
        assert_eq!(ParachainState::get(), ParachainState::default());
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
fn integration_test_operator_withdrawal_can_force_refund_nominators() {
    test_with_nomination_enabled(|| {
        let vault_withdrawal_to_reach_max_nominatio_ratio = 800000;
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(
            vault_withdrawal_to_reach_max_nominatio_ratio
        ))
        .dispatch(origin_of(account_of(VAULT))));
        assert_register_operator(VAULT);
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let operator_amount_to_withdraw = 10000;
        // The Nomination Ratio is currently at the max permitted level
        // So the force-refunded amount will be `max_nomination_ratio * amount_to_withdraw`
        let expected_nominator_force_refund = FeePallet::backing_for(
            operator_amount_to_withdraw,
            NominationPallet::get_max_nomination_ratio(),
        )
        .unwrap();

        assert_ok!(request_operator_collateral_withdrawal(
            VAULT,
            operator_amount_to_withdraw
        ));

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, _, operator| {
                user.free_balance = user.free_balance - DEFAULT_NOMINATION + expected_nominator_force_refund;

                vault.backing_collateral = vault.backing_collateral - vault_withdrawal_to_reach_max_nominatio_ratio
                    + DEFAULT_NOMINATION
                    - operator_amount_to_withdraw
                    - expected_nominator_force_refund;
                vault.free_balance += vault_withdrawal_to_reach_max_nominatio_ratio;
                vault.griefing_collateral += operator_amount_to_withdraw;

                let expected_nominator = CoreNominatorData {
                    collateral: DEFAULT_NOMINATION - expected_nominator_force_refund,
                    collateral_to_be_withdrawn: 0,
                };
                let mut expected_nominators = BTreeMap::<AccountId, CoreNominatorData>::new();
                expected_nominators.insert(account_of(USER), expected_nominator.clone());
                operator.nominators = expected_nominators;
                operator.total_nominated_collateral = expected_nominator.collateral;
                operator.collateral_to_be_withdrawn = operator_amount_to_withdraw;
            })
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
        let expected_nominator_slashed_amount = nominator_collateral_before_liquidation * expected_slashed_amount
            / vault_backing_collateral_before_liquidation;
        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, liquidation_vault, _, operator| {
                user.free_balance -= DEFAULT_NOMINATION;
                liquidation_vault.to_be_issued = DEFAULT_VAULT_TO_BE_ISSUED;
                liquidation_vault.issued = DEFAULT_VAULT_ISSUED;
                liquidation_vault.to_be_redeemed = DEFAULT_VAULT_TO_BE_REDEEMED;
                liquidation_vault.backing_collateral = expected_slashed_amount;
                vault.to_be_issued = 0;
                vault.issued = 0;
                vault.backing_collateral = vault.backing_collateral + DEFAULT_NOMINATION - expected_slashed_amount;

                let expected_nominator = CoreNominatorData {
                    collateral: DEFAULT_NOMINATION - expected_nominator_slashed_amount,
                    collateral_to_be_withdrawn: 0,
                };
                let mut expected_nominators = BTreeMap::<AccountId, CoreNominatorData>::new();
                expected_nominators.insert(account_of(USER), expected_nominator.clone());
                operator.nominators = expected_nominators;
                operator.total_nominated_collateral = expected_nominator.collateral;
            })
        );
    });
}

#[test]
fn integration_test_liquidating_operators_for_stealing_minimizes_nominator_slashing() {
    test_with_nomination_enabled_and_operator_registered(|| {
        assert_nominate_collateral(USER, VAULT, DEFAULT_NOMINATION);
        let expected_slashed_amount = 135_000;
        let expected_nominator_slashed_amount = 70_000;

        // Theft liquidation
        NominationPallet::liquidate_operator_with_status(&account_of(VAULT), VaultStatus::CommittedTheft).unwrap();

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, liquidation_vault, _, _| {
                user.free_balance = user.free_balance - expected_nominator_slashed_amount;
                liquidation_vault.to_be_issued = DEFAULT_VAULT_TO_BE_ISSUED;
                liquidation_vault.issued = DEFAULT_VAULT_ISSUED;
                liquidation_vault.to_be_redeemed = DEFAULT_VAULT_TO_BE_REDEEMED;
                liquidation_vault.backing_collateral = expected_slashed_amount;
                vault.to_be_issued = 0;
                vault.issued = 0;
                vault.free_balance = vault.free_balance
                    + (DEFAULT_VAULT_BACKING_COLLATERAL
                        - (expected_slashed_amount - expected_nominator_slashed_amount));
                vault.backing_collateral = 0;
            })
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
