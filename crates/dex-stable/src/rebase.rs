use super::*;
use orml_traits::MultiCurrency;

pub trait TryConvertBalance<A, B> {
    type AssetId;
    fn try_convert_balance(amount: A, asset_id: Self::AssetId) -> Result<B, DispatchError>;
    fn try_convert_balance_back(amount: B, asset_id: Self::AssetId) -> Result<A, DispatchError>;
}

pub trait CurrencyConversion<Balance, CurrencyId> {
    fn convert(amount: Balance, from: CurrencyId, to: CurrencyId) -> Result<Balance, DispatchError>;
}

/// Used to convert supported liquid currencies to their corresponding staking currencies
/// and vice versa, e.g. LDOT/DOT
/// NOTE: oracle already handles converting via wrapped currency
pub struct RebaseAdapter<T, C>(PhantomData<(T, C)>);

impl<T, C> TryConvertBalance<Balance, Balance> for RebaseAdapter<T, C>
where
    T: Config,
    C: CurrencyConversion<Balance, T::CurrencyId>,
{
    type AssetId = T::CurrencyId;

    // e.g. LKSM -> KSM
    fn try_convert_balance(amount: Balance, from_asset_id: T::CurrencyId) -> Result<Balance, DispatchError> {
        if let Some(to_asset_id) = RebaseTokens::<T>::get(&from_asset_id) {
            C::convert(amount, from_asset_id, to_asset_id)
        } else {
            Ok(amount)
        }
    }

    // e.g. KSM -> LKSM
    fn try_convert_balance_back(amount: Balance, from_asset_id: T::CurrencyId) -> Result<Balance, DispatchError> {
        if let Some(to_asset_id) = RebaseTokens::<T>::get(&from_asset_id) {
            C::convert(amount, to_asset_id, from_asset_id)
        } else {
            Ok(amount)
        }
    }
}

impl<T: Config> Pallet<T> {
    fn update_balances(
        pool: &mut BasePool<T::CurrencyId, T::AccountId, T::PoolCurrencyLimit, T::PoolCurrencySymbolLimit>,
    ) -> Result<(), DispatchError> {
        for (i, balance) in pool.balances.iter().enumerate() {
            pool.rebased_balances[i] = T::RebaseConvert::try_convert_balance(*balance, pool.currency_ids[i])?;
        }
        Ok(())
    }

    fn get_yield_amount(
        pool: &BasePool<T::CurrencyId, T::AccountId, T::PoolCurrencyLimit, T::PoolCurrencySymbolLimit>,
    ) -> Result<Balance, DispatchError> {
        let amp = Self::get_a_precise(pool).ok_or(Error::<T>::Arithmetic)?;
        let new_d = Self::get_d(
            &Self::xp(&pool.rebased_balances, &pool.token_multipliers).ok_or(Error::<T>::Arithmetic)?,
            amp,
        )
        .ok_or(Error::<T>::Arithmetic)?;
        Ok(new_d)
    }

    fn collect_yield(
        pool: &mut BasePool<T::CurrencyId, T::AccountId, T::PoolCurrencyLimit, T::PoolCurrencySymbolLimit>,
    ) -> DispatchResult {
        let old_d = Self::get_yield_amount(pool)?;
        Self::update_balances(pool)?;
        let new_d = Self::get_yield_amount(pool)?;
        ensure!(new_d >= old_d, Error::<T>::CheckDFailed);
        if new_d > old_d {
            let yield_amount = new_d - old_d;
            T::MultiCurrency::deposit(pool.lp_currency_id, &pool.admin_fee_receiver, yield_amount)?;
        }
        Ok(())
    }

    pub(crate) fn inner_collect_yield(
        general_pool: &mut Pool<
            T::PoolId,
            T::CurrencyId,
            T::AccountId,
            T::PoolCurrencyLimit,
            T::PoolCurrencySymbolLimit,
        >,
    ) -> DispatchResult {
        let pool = match general_pool {
            Pool::Base(bp) => bp,
            Pool::Meta(mp) => &mut mp.info,
        };
        Self::collect_yield(pool)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mock::{CurrencyId::*, *},
        *,
    };
    use frame_support::assert_ok;
    use frame_system::RawOrigin;

    const INITIAL_A_VALUE: Balance = 50;
    const SWAP_FEE: Balance = 1e7 as Balance;
    const ADMIN_FEE: Balance = 5_000_000_000;

    #[test]
    fn create_pool_with_rebasing_asset() {
        new_test_ext().execute_with(|| {
            // staking (rebase) token is worth 2:1 of liquid token
            RebaseTokens::<Test>::insert(Rebase(TOKEN1_SYMBOL), Token(TOKEN1_SYMBOL));
            mock::oracle::Price::<Test>::insert((Rebase(TOKEN1_SYMBOL), Token(TOKEN1_SYMBOL)), 1);

            assert_ok!(StableAmm::create_base_pool(
                RawOrigin::Root.into(),
                vec![Token(TOKEN1_SYMBOL), Rebase(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL)],
                vec![TOKEN1_DECIMAL, TOKEN1_DECIMAL, TOKEN2_DECIMAL],
                INITIAL_A_VALUE,
                SWAP_FEE,
                ADMIN_FEE,
                ALICE,
                Vec::from("stable_pool_lp"),
            ));

            let pool_id = StableAmm::next_pool_id() - 1;
            assert_ok!(StableAmm::add_liquidity(
                RawOrigin::Signed(BOB).into(),
                pool_id,
                vec![1e18 as Balance, 1e18 as Balance, 1e18 as Balance],
                0,
                BOB,
                u64::MAX,
            ));

            let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
            let calculated_swap_return = StableAmm::calculate_base_swap_amount(&pool, 0, 1, 1e17 as Balance).unwrap();
            assert_eq!(calculated_swap_return, 99702611562565288);

            // rebase tokens are now worth twice as much
            mock::oracle::Price::<Test>::insert((Rebase(TOKEN1_SYMBOL), Token(TOKEN1_SYMBOL)), 2);
            assert_ok!(StableAmm::add_liquidity(
                RawOrigin::Signed(BOB).into(),
                pool_id,
                vec![1e18 as Balance, 0, 1e18 as Balance],
                0,
                BOB,
                u64::MAX,
            ));

            // price stays the same
            let calculated_swap_return = StableAmm::calculate_base_swap_amount(&pool, 0, 1, 1e17 as Balance).unwrap();
            assert_eq!(calculated_swap_return, 99702611562565288);

            let admin_lp_balance = <Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &ALICE);
            assert_eq!(admin_lp_balance, 995181615631638418);

            fn calculate_remove_all_liquidity(pool_id: PoolId, account: AccountId) -> Vec<Balance> {
                let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
                let amounts = StableAmm::calculate_base_remove_liquidity(
                    &pool,
                    <Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &account),
                )
                .unwrap();

                amounts
                    .iter()
                    .zip(pool.currency_ids)
                    .map(|(amount, currency_id)| {
                        <Test as Config>::RebaseConvert::try_convert_balance_back(*amount, currency_id.clone())
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
            }

            assert_eq!(
                calculate_remove_all_liquidity(pool_id, ALICE),
                [331753180913049698, 165868730225911397, 331753180913049698]
            );
            assert_eq!(
                calculate_remove_all_liquidity(pool_id, BOB),
                [1668153408288201704, 834037180572837200, 1668153408288201704]
            );
        });
    }
}
