// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![feature(array_windows)]

#[cfg(test)]
mod mock;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;
pub mod default_weights;
#[cfg(test)]
mod test;
pub use default_weights::WeightInfo;

use codec::{Decode, Encode};

use frame_support::{
    dispatch::{Codec, DispatchResult},
    pallet_prelude::*,
    transactional,
};
use sp_runtime::traits::{AtLeast32BitUnsigned, One, Zero};
use sp_std::{fmt::Debug, prelude::*, vec::Vec};

use dex_general::{AssetBalance, ExportDexGeneral};
use dex_stable::traits::StableAmmApi;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct GeneralPath<CurrencyId> {
    pub asset_0: CurrencyId,
    pub asset_1: CurrencyId,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct StablePath<PoolId, CurrencyId> {
    pub pool_id: PoolId,
    pub base_pool_id: PoolId,
    pub mode: StableSwapMode,
    pub from_currency: CurrencyId,
    pub to_currency: CurrencyId,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum StableSwapMode {
    Single,
    FromBase,
    ToBase,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum Route<PoolId, CurrencyId> {
    General(GeneralPath<CurrencyId>),
    Stable(StablePath<PoolId, CurrencyId>),
}

impl<PoolId, CurrencyId: Clone> Route<PoolId, CurrencyId> {
    fn input_currency(&self) -> CurrencyId {
        match self {
            Route::General(x) => x.asset_0.clone(),
            Route::Stable(x) => x.from_currency.clone(),
        }
    }

    fn output_currency(&self) -> CurrencyId {
        match self {
            Route::General(x) => x.asset_1.clone(),
            Route::Stable(x) => x.to_currency.clone(),
        }
    }
}
pub use pallet::*;

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type StablePoolId: Parameter + Codec + Copy + Ord + AtLeast32BitUnsigned + Zero + One + Default;

        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + MaxEncodedLen
            + From<AssetBalance>
            + Into<AssetBalance>
            + TypeInfo;

        // The currency id used in both amms
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + TypeInfo + MaxEncodedLen;

        type GeneralAmm: ExportDexGeneral<AccountIdOf<Self>, Self::CurrencyId>;

        type StableAmm: StableAmmApi<Self::StablePoolId, Self::CurrencyId, AccountIdOf<Self>, Self::Balance>;

        /// The maximum number of swaps allowed in routes
        #[pallet::constant]
        type MaxSwaps: Get<u16>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        MismatchPoolAndCurrencyId,
        Deadline,
        InvalidRoutes,
        ConvertCurrencyFailed,
        AmountSlippage,
        InvalidPath,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(
            T::WeightInfo::swap_exact_tokens_for_tokens_general(
                routes.len() as u32
            ).max(T::WeightInfo::swap_exact_tokens_for_tokens_stable(
                routes.len() as u32
            ))
        )]
        #[transactional]
        pub fn swap_exact_tokens_for_tokens(
            origin: OriginFor<T>,
            amount_in: T::Balance,
            amount_out_min: T::Balance,
            routes: Vec<Route<T::StablePoolId, T::CurrencyId>>,
            to: T::AccountId,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::validate_routes(&routes)?;

            let mut amount_out = amount_in;
            let mut receiver = who.clone();

            let num_routes = routes.len();
            for (i, route) in routes.into_iter().enumerate() {
                if i == num_routes - 1 {
                    receiver = to.clone();
                }
                match route {
                    Route::General(GeneralPath { asset_0, asset_1 }) => {
                        let path = vec![asset_0, asset_1];
                        let amounts = T::GeneralAmm::get_amount_out_by_path(amount_out.into(), &path)?;
                        Self::swap(&who, amount_out, &path, &receiver)?;
                        amount_out = T::Balance::from(*amounts.last().ok_or(Error::<T>::InvalidPath)?);
                    }
                    Route::Stable(stable_path) => {
                        (amount_out) = Self::stable_swap(&who, &stable_path, amount_out, &receiver)?;
                    }
                }
            }

            ensure!(amount_out >= amount_out_min, Error::<T>::AmountSlippage);

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn stable_swap(
        who: &T::AccountId,
        path: &StablePath<T::StablePoolId, T::CurrencyId>,
        amount_in: T::Balance,
        to: &T::AccountId,
    ) -> Result<T::Balance, DispatchError> {
        let out_amount = match path.mode {
            StableSwapMode::Single => {
                let from_index = Self::currency_index_from_stable_pool(path.pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.pool_id, path.to_currency)?;
                T::StableAmm::swap(who, path.pool_id, from_index, to_index, amount_in, Zero::zero(), to)?
            }
            StableSwapMode::FromBase => {
                let from_index = Self::currency_index_from_stable_pool(path.base_pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.pool_id, path.to_currency)?;

                T::StableAmm::swap_pool_from_base(
                    who,
                    path.pool_id,
                    path.base_pool_id,
                    from_index,
                    to_index,
                    amount_in,
                    Zero::zero(),
                    to,
                )?
            }
            StableSwapMode::ToBase => {
                let from_index = Self::currency_index_from_stable_pool(path.pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.base_pool_id, path.to_currency)?;
                T::StableAmm::swap_pool_to_base(
                    who,
                    path.pool_id,
                    path.base_pool_id,
                    from_index,
                    to_index,
                    amount_in,
                    Zero::zero(),
                    to,
                )?
            }
        };
        Ok(out_amount)
    }

    fn swap(who: &T::AccountId, amount_in: T::Balance, path: &[T::CurrencyId], to: &T::AccountId) -> DispatchResult {
        T::GeneralAmm::inner_swap_exact_assets_for_assets(who, amount_in.into(), Zero::zero(), path, to)
    }

    fn currency_index_from_stable_pool(
        pool_id: T::StablePoolId,
        currency_id: T::CurrencyId,
    ) -> Result<u32, DispatchError> {
        T::StableAmm::currency_index(pool_id, currency_id).ok_or_else(|| Error::<T>::MismatchPoolAndCurrencyId.into())
    }

    fn validate_routes(routes: &[Route<T::StablePoolId, T::CurrencyId>]) -> DispatchResult {
        for [route_1, route_2] in routes.array_windows::<2>() {
            let output_1 = route_1.output_currency();
            let input_2 = route_2.input_currency();
            ensure!(output_1 == input_2, Error::<T>::InvalidRoutes);
        }
        Ok(())
    }
}
