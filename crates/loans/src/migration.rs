use super::*;
use frame_support::traits::OnRuntimeUpgrade;

// #[cfg(feature = "try-runtime")]
use sp_std::{vec, vec::Vec};

use currency::Amount;

/// The log target.
const TARGET: &'static str = "runtime::loans::migration";

pub mod collateral_toggle {
    use super::*;

    struct InconsistentBalance<T>
    where
        T: Config,
    {
        currency_id: CurrencyId<T>,
        account_id: T::AccountId,
        free_balance: BalanceOf<T>,
    }

    fn get_inconsistent_lend_token_balances<T: Config>() -> (Vec<InconsistentBalance<T>>, u64) {
        // Iterating `AccountDeposits` should guarantee there are no other
        let mut reads = 0;
        let inconsistent_balances = crate::AccountDeposits::<T>::iter()
            .filter_map(|(currency_id, account_id, _collateral)| {
                reads += 1;
                let free_balance = Amount::<T>::new(
                    orml_tokens::Pallet::<T>::free_balance(currency_id.clone(), &account_id),
                    currency_id.clone(),
                );

                let reserved_balance = Amount::<T>::new(
                    orml_tokens::Pallet::<T>::reserved_balance(currency_id.clone(), &account_id),
                    currency_id.clone(),
                );

                if !free_balance.is_zero() && !reserved_balance.is_zero() {
                    return Some(InconsistentBalance {
                        currency_id: currency_id.clone(),
                        account_id: account_id.clone(),
                        free_balance: free_balance.amount(),
                    });
                }
                None
            })
            .collect();
        (inconsistent_balances, reads)
    }

    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for Migration<T> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let (inconsistent_balances, _) = get_inconsistent_lend_token_balances::<T>();
            ensure!(
                inconsistent_balances.len() > 0,
                "There are no inconsistent balances to migrate"
            );
            log::info!(
                target: TARGET,
                "{} inconsistent lending collateral balances will be migrated",
                inconsistent_balances.len()
            );
            Ok(vec![])
        }

        fn on_runtime_upgrade() -> Weight {
            let mut writes = 0;
            let (inconsistent_balances, mut reads) = get_inconsistent_lend_token_balances::<T>();
            for b in inconsistent_balances {
                reads += 1;
                match Pallet::<T>::lock_if_account_deposited(&b.account_id, &Amount::new(b.free_balance, b.currency_id))
                {
                    Err(e) => log::warn!(
                        target: TARGET,
                        "Failed to lock collateral for account {:?}, collateral: {:?}. Error: {:?}",
                        b.account_id,
                        b.currency_id,
                        e
                    ),
                    Ok(_) => {
                        writes += 1;
                        log::info!(
                            target: TARGET,
                            "Locked all free collateral for {:?}, in {:?}",
                            b.account_id,
                            b.currency_id,
                        );
                    }
                }
            }
            T::DbWeight::get().reads_writes(reads, writes)
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
            let (inconsistent_balances, _) = get_inconsistent_lend_token_balances::<T>();
            ensure!(
                inconsistent_balances.len() == 0,
                "There should be no inconsistent lending collateral balances left"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
    use super::*;
    use crate::{
        mock::*,
        tests::lend_tokens::{free_balance, reserved_balance},
    };
    use frame_support::{assert_err, assert_ok};

    use primitives::{
        CurrencyId::{self, Token},
        KINT as KINT_CURRENCY,
    };

    const KINT: CurrencyId = Token(KINT_CURRENCY);

    #[test]
    fn inconsistent_balances_migration_works() {
        new_test_ext().execute_with(|| {
            assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(100)));
            assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KINT));
            assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(100)));
            assert_err!(
                Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KINT),
                Error::<Test>::TokensAlreadyLocked
            );
            // Both `free` and `reserved` balances are nonzero at the same time,
            // ẉhich is an invalid state
            assert_eq!(free_balance(LEND_KINT, &DAVE), unit(100) * 50);
            assert_eq!(reserved_balance(LEND_KINT, &DAVE), unit(100) * 50);

            let state = collateral_toggle::Migration::<Test>::pre_upgrade().unwrap();
            let _w = collateral_toggle::Migration::<Test>::on_runtime_upgrade();
            collateral_toggle::Migration::<Test>::post_upgrade(state).unwrap();
        });
    }
}
