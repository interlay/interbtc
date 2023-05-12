// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use dex_general::{ExportDexGeneral, WeightInfo};
use frame_support::{
    ensure,
    traits::{Currency, Get, IsSubType, OnUnbalanced},
};
pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{
    traits::{DispatchInfoOf, PostDispatchInfoOf, Zero},
    transaction_validity::{InvalidTransaction, TransactionValidityError},
};
use sp_std::{boxed::Box, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type CallOf<T> = <T as Config>::RuntimeCall;
type SubstrateDefaultPayment<T> =
    pallet_transaction_payment::CurrencyAdapter<<T as Config>::Currency, <T as Config>::OnUnbalanced>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        dispatch::{DispatchInfo, GetDispatchInfo, PostDispatchInfo},
        pallet_prelude::*,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Dispatchable;

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeCall = CallOf<Self>> + currency::Config<Balance = u128> {
        /// The aggregated call type.
        type RuntimeCall: Parameter
            // + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo, Info = DispatchInfo>
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo, Info = DispatchInfo>
            + GetDispatchInfo
            + IsSubType<Call<Self>>;

        /// dex api
        type Dex: ExportDexGeneral<Self::AccountId, Self::CurrencyId>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId, Balance = u128>;

        /// What to do with fees. Typically this is a transfer to the block author or treasury
        type OnUnbalanced: OnUnbalanced<<Self::Currency as Currency<Self::AccountId>>::NegativeImbalance>;

        /// weights of dex operations
        type DexWeightInfo: dex_general::WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			((T::DexWeightInfo::swap_assets_for_exact_assets(_path.len() as u32)).saturating_add(dispatch_info.weight), dispatch_info.class,)
		})]
        #[frame_support::transactional]
        pub fn with_fee_swap_path(
            origin: OriginFor<T>,
            _path: Vec<T::CurrencyId>,
            _amount_in_max: <T as currency::Config>::Balance,
            call: Box<CallOf<T>>,
        ) -> DispatchResultWithPostInfo {
            // Note: no swaps are being done here - they have already been done in the `pre_dispatch`
            // of the SignedExtension, which calls the `OnChargeTransaction::withdraw_fee` implemented below.

            let mut ret = call.dispatch(origin);
            // modify any returned weight overrides. Note that we pass-through the `Pays` unmodified.
            let modify_weight = |x: &mut PostDispatchInfo| {
                x.actual_weight = x
                    .actual_weight
                    .map(|x| x.saturating_add(T::DexWeightInfo::swap_assets_for_exact_assets(_path.len() as u32)));
            };
            match ret {
                Ok(ref mut info) => {
                    // if a weight override is returned, add the cost of the swap
                    modify_weight(info);
                }
                Err(ref mut err) => {
                    modify_weight(&mut err.post_info);
                }
            };

            ret
        }
    }
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
    T: pallet::Config + pallet_transaction_payment::Config,
{
    type LiquidityInfo = Option<<T::Currency as Currency<T::AccountId>>::NegativeImbalance>;
    type Balance = <T as currency::Config>::Balance;

    /// Withdraw the predicted fee from the transaction origin.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn withdraw_fee(
        who: &T::AccountId,
        call: &CallOf<T>,
        info: &DispatchInfoOf<CallOf<T>>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }

        match call.is_sub_type() {
            Some(pallet::Call::with_fee_swap_path {
                path, amount_in_max, ..
            }) => {
                // check that the swap path ends in the native currency
                ensure!(
                    path.iter().last() == Some(&T::GetNativeCurrencyId::get()),
                    TransactionValidityError::Invalid(InvalidTransaction::Payment)
                );
                // note: we get passed only an account id rather than an origin, so we can't do the usual
                // ensure_origin. However, the fact that there _is_ an account implies that the origin is
                // signed: an unsigned extrinsic is not associated with an account, and transactions with
                // invalid signatures fail earlier in the flow. See https://substrate.stackexchange.com/a/5806/1779
                T::Dex::inner_swap_assets_for_exact_assets(who, fee, *amount_in_max, &path, &who)
                    .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
            }
            _ => {}
        }

        <SubstrateDefaultPayment<T> as OnChargeTransaction<T>>::withdraw_fee(who, call, info, fee, tip)
    }

    /// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
    /// Since the predicted fee might have been too high, parts of the fee may
    /// be refunded.
    ///
    /// Note: we refund in the native currency, we don't do extra swaps
    fn correct_and_deposit_fee(
        who: &T::AccountId,
        dispatch_info: &DispatchInfoOf<CallOf<T>>,
        post_info: &PostDispatchInfoOf<CallOf<T>>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        <SubstrateDefaultPayment<T> as OnChargeTransaction<T>>::correct_and_deposit_fee(
            who,
            dispatch_info,
            post_info,
            corrected_fee,
            tip,
            already_withdrawn,
        )
    }
}
