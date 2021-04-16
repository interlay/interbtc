mod mock;

use mock::*;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());
        execute()
    })
}

mod lock_additional_collateral_test {
    use super::*;

    #[test]
    fn integration_test_vault_registry_lock_additional_collateral_below_capacity_succeeds() {
        test_with(|| {
            let amount = 1_000;

            assert_ok!(
                Call::VaultRegistry(VaultRegistryCall::lock_additional_collateral(amount))
                    .dispatch(origin_of(account_of(VAULT)))
            );

            assert_eq!(
                ParachainState::get(),
                ParachainState::default().with_changes(|_, vault, _, _| {
                    vault.backing_collateral += amount;
                    vault.free_balance -= amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_at_capacity_succeeds() {
        test_with(|| {
            let amount = DEFAULT_VAULT_FREE_BALANCE;

            assert_ok!(
                Call::VaultRegistry(VaultRegistryCall::lock_additional_collateral(amount))
                    .dispatch(origin_of(account_of(VAULT)))
            );

            assert_eq!(
                ParachainState::get(),
                ParachainState::default().with_changes(|_, vault, _, _| {
                    vault.backing_collateral += amount;
                    vault.free_balance -= amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_above_capacity_fails() {
        test_with(|| {
            let amount = DEFAULT_VAULT_FREE_BALANCE + 1;

            assert_noop!(
                Call::VaultRegistry(VaultRegistryCall::lock_additional_collateral(amount))
                    .dispatch(origin_of(account_of(VAULT))),
                CollateralError::InsufficientFunds
            );
        });
    }
}
mod withdraw_collateral_test {
    use super::*;

    fn required_collateral() -> u128 {
        VaultRegistryPallet::get_required_collateral_for_vault(account_of(VAULT)).unwrap()
    }

    #[test]
    fn integration_test_vault_registry_withdraw_collateral_below_capacity_succeeds() {
        test_with(|| {
            let amount = 1_000;

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(amount))
                .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(),
                ParachainState::default().with_changes(|_, vault, _, _| {
                    vault.backing_collateral -= amount;
                    vault.free_balance += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_at_capacity_succeeds() {
        test_with(|| {
            let amount = DEFAULT_VAULT_BACKING_COLLATERAL - required_collateral();

            assert_ok!(Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(amount))
                .dispatch(origin_of(account_of(VAULT))));

            assert_eq!(
                ParachainState::get(),
                ParachainState::default().with_changes(|_, vault, _, _| {
                    vault.backing_collateral -= amount;
                    vault.free_balance += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_lock_additional_above_capacity_fails() {
        test_with(|| {
            let amount = DEFAULT_VAULT_BACKING_COLLATERAL - required_collateral() + 1;

            assert_noop!(
                Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(amount))
                    .dispatch(origin_of(account_of(VAULT))),
                VaultRegistryError::InsufficientCollateral
            );
        });
    }
}
