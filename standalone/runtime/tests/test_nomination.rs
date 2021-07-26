mod mock;

use mock::{nomination_testing_utils::*, *};
use sp_runtime::traits::{CheckedDiv, CheckedSub};

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());
        execute()
    })
}

fn test_with_nomination_enabled<R>(execute: impl FnOnce() -> R) -> R {
    test_with(|| {
        enable_nomination();
        execute()
    })
}

fn test_with_nomination_enabled_and_vault_opted_in<R>(execute: impl FnOnce() -> R) -> R {
    test_with_nomination_enabled(|| {
        assert_nomination_opt_in(VAULT);
        execute()
    })
}

#[test]
fn integration_test_regular_vaults_are_not_opted_in_to_nomination() {
    test_with_nomination_enabled(|| {
        assert_register_vault(CAROL);
        assert_eq!(NominationPallet::is_nominatable(&account_of(CAROL)).unwrap(), false);
    })
}

#[test]
fn integration_test_vaults_can_opt_in() {
    test_with_nomination_enabled(|| {
        assert_nomination_opt_in(VAULT);
        assert_eq!(NominationPallet::is_nominatable(&account_of(VAULT)).unwrap(), true);
    });
}

#[test]
fn integration_test_vaults_cannot_opt_in_if_disabled() {
    test_with(|| {
        assert_noop!(nomination_opt_in(VAULT), NominationError::VaultNominationDisabled);
    });
}

#[test]
fn integration_test_vaults_can_still_opt_out_if_disabled() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        disable_nomination();
        assert_ok!(nomination_opt_out(VAULT));
    });
}

#[test]
fn integration_test_cannot_nominate_if_not_opted_in() {
    test_with_nomination_enabled(|| {
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
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        let nominator_collateral = get_nominator_collateral(VAULT, USER);
        assert_eq!(nominator_collateral, DEFAULT_NOMINATION);
        assert_total_nominated_collateral_is(VAULT, DEFAULT_NOMINATION);
    });
}

#[test]
fn integration_test_vaults_cannot_withdraw_nominated_collateral() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        assert_noop!(
            withdraw_vault_collateral(VAULT, DEFAULT_BACKING_COLLATERAL + 1),
            VaultRegistryError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_nominated_collateral_cannot_exceed_max_nomination_ratio() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_noop!(
            nominate_collateral(VAULT, USER, DEFAULT_BACKING_COLLATERAL),
            NominationError::DepositViolatesMaxNominationRatio
        );
    });
}

#[test]
fn integration_test_nominated_collateral_prevents_replace_requests() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, DEFAULT_BACKING_COLLATERAL))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultHasEnabledNomination
        );
    });
}

#[test]
fn integration_test_vaults_with_zero_nomination_cannot_request_replacement() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
        let griefing_collateral = 200;
        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(VAULT))),
            ReplaceError::VaultHasEnabledNomination
        );
    });
}

#[test]
fn integration_test_nomination_increases_issuable_tokens() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        let issuance_capacity_before_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_before_nomination, 556666);
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        let issuance_capacity_after_nomination =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_eq!(issuance_capacity_after_nomination, 570000);
    });
}

#[test]
fn integration_test_nominator_withdrawal_request_reduces_issuable_tokens() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        let issuance_capacity_before_withdrawal_request =
            VaultRegistryPallet::get_issuable_tokens_from_vault(account_of(VAULT)).unwrap();
        assert_ok!(withdraw_nominator_collateral(USER, VAULT, DEFAULT_NOMINATION));
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
        assert_nomination_opt_in(VAULT);
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
            FixedU128::checked_from_integer(3).unwrap()
        ));
        assert_noop!(
            withdraw_nominator_collateral(USER, VAULT, DEFAULT_NOMINATION),
            NominationError::InsufficientCollateral
        );
    });
}

#[test]
fn integration_test_nomination_fee_distribution() {
    test_with_nomination_enabled(|| {});
}

#[test]
fn integration_test_maximum_nomination_ratio_calculation() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        let expected_nomination_ratio = FixedU128::checked_from_rational(150, 100)
            .unwrap()
            .checked_div(&FixedU128::checked_from_rational(135, 100).unwrap())
            .unwrap()
            .checked_sub(&FixedU128::one())
            .unwrap();
        assert_eq!(
            VaultRegistryPallet::get_max_nomination_ratio().unwrap(),
            expected_nomination_ratio
        );
    })
}

#[test]
fn integration_test_vault_opt_out_must_refund_nomination() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        assert_eq!(
            NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
            DEFAULT_NOMINATION
        );
        assert_eq!(
            NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
            DEFAULT_NOMINATION
        );
        assert_eq!(
            VaultRewardsPallet::compute_reward(INTERBTC, &account_of(USER)).unwrap(),
            0
        );
        assert_ok!(nomination_opt_out(VAULT));
        assert_eq!(
            NominationPallet::get_total_nominated_collateral(&account_of(VAULT)).unwrap(),
            0
        );
        assert_eq!(
            NominationPallet::get_nominator_collateral(&account_of(VAULT), &account_of(USER)).unwrap(),
            0
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
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        VaultRegistryPallet::ban_vault(account_of(VAULT)).unwrap();
        let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
        assert_eq!(nonce, 0);
    })
}

#[test]
fn integration_test_liquidating_a_vault_does_not_force_refund() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        assert_nominate_collateral(VAULT, USER, DEFAULT_NOMINATION);
        VaultRegistryPallet::liquidate_vault(&account_of(VAULT)).unwrap();
        let nonce: u32 = VaultStakingPallet::nonce(INTERBTC, &account_of(VAULT));
        assert_eq!(nonce, 0);
    })
}

#[test]
fn integration_test_vault_withdrawal_cannot_exceed_max_nomination_taio() {
    test_with_nomination_enabled_and_vault_opted_in(|| {
        let max_nomination = VaultRegistryPallet::get_max_nominatable_collateral(DEFAULT_BACKING_COLLATERAL).unwrap();
        assert_nominate_collateral(VAULT, USER, max_nomination);

        // Need to withdraw 10 units to account for rounding errors
        assert_noop!(
            withdraw_vault_collateral(VAULT, 10),
            VaultRegistryError::MaxNominatioRatioViolation
        );
    })
}
