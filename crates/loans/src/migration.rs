use super::*;
use frame_support::{pallet_prelude::StorageVersion, traits::OnRuntimeUpgrade};

// #[cfg(feature = "try-runtime")]
use sp_std::{vec, vec::Vec};

/// The log target.
const TARGET: &'static str = "runtime::loans::migration";

pub mod collateral_toggle {
    use super::*;
    use frame_support::pallet_prelude::*;

    type SignedFixedPoint<T> = <T as currency::Config>::SignedFixedPoint;

    fn get_inconsistent_lend_token_balances<T: Config>() -> Vec<(CurrencyId<T>, T::AccountId, BalanceOf<T>)> {
        // Iterating `AccountDeposits` should guarantee there are no other
        let num_vaults = crate::AccountDeposits::<T>::iter()
            .filter(|(currency_id, account_id, _collateral)| {
                // Assumes the default is zero
                let free_tokens =
                    Pallet::<T>::free_lend_tokens(*currency_id, account_id).unwrap_or(Amount::<T>::zero(*currency_id));
                let reserved_tokens = Pallet::<T>::reserved_lend_tokens(*currency_id, account_id)
                    .unwrap_or(Amount::<T>::zero(*currency_id));
                !free_tokens.is_zero() && !reserved_tokens.is_zero()
            })
            .collect();
        num_vaults
    }

    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for Migration<T> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let inconsistent_balances = get_inconsistent_lend_token_balances::<T>();
            log::info!(
                target: TARGET,
                "{} non-zero stake entries will be migrated",
                inconsistent_balances.len()
            );
            Ok(vec![])
        }

        fn on_runtime_upgrade() -> Weight {
            let mut reads = 0;
            let mut writes = 0;
            let inconsistent_balances = get_inconsistent_lend_token_balances::<T>();

            log::info!(
                target: TARGET,
                "{} non-zero stake entries will be migrated",
                inconsistent_balances.len()
            );

            log::info!(
                "{} non-zero stake entries will be migrated",
                inconsistent_balances.len()
            );
            T::DbWeight::get().reads_writes(reads, reads * 2)
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
            let inconsistent_balances = get_inconsistent_lend_token_balances::<T>();
            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
    use super::*;
    use crate::mock::*;
    use frame_support::assert_ok;
}
