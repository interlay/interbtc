use crate::{Config, CurrencyId, Pallet, RebaseTokens};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Contains;
use orml_tokens::ConvertBalance;
use orml_traits::MultiCurrency;
use scale_info::TypeInfo;
use sp_runtime::DispatchResult;
use sp_std::marker::PhantomData;

pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;

pub type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
}

pub struct RebaseAdapter<T>(PhantomData<T>);

impl<T> ConvertBalance<BalanceOf<T>, BalanceOf<T>> for RebaseAdapter<T>
where
    T: Config,
{
    type AssetId = CurrencyId;

    fn convert_balance(amount: BalanceOf<T>, from_asset_id: CurrencyId) -> BalanceOf<T> {
        if let Some(to_asset_id) = RebaseTokens::<T>::get(&from_asset_id) {
            let amount = Pallet::<T>::collateral_to_wrapped(amount, from_asset_id).unwrap_or_default();
            Pallet::<T>::wrapped_to_collateral(amount, to_asset_id).unwrap_or_default()
        } else {
            amount
        }
    }

    fn convert_balance_back(amount: BalanceOf<T>, from_asset_id: CurrencyId) -> BalanceOf<T> {
        if let Some(to_asset_id) = RebaseTokens::<T>::get(&from_asset_id) {
            let amount = Pallet::<T>::collateral_to_wrapped(amount, to_asset_id).unwrap_or_default();
            Pallet::<T>::wrapped_to_collateral(amount, from_asset_id).unwrap_or_default()
        } else {
            amount
        }
    }
}

pub struct IsRebaseToken<T>(PhantomData<T>);

impl<T> Contains<CurrencyId> for IsRebaseToken<T>
where
    T: Config,
{
    fn contains(currency_id: &CurrencyId) -> bool {
        RebaseTokens::<T>::contains_key(currency_id)
    }
}

pub struct Combiner<AccountId, TestKey, A, B>(PhantomData<(AccountId, TestKey, A, B)>);

impl<AccountId, TestKey, A, B> MultiCurrency<AccountId> for Combiner<AccountId, TestKey, A, B>
where
    TestKey: Contains<CurrencyId>,
    A: MultiCurrency<AccountId, CurrencyId = CurrencyId, Balance = <B as MultiCurrency<AccountId>>::Balance>,
    B: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
    type CurrencyId = CurrencyId;
    type Balance = <B as MultiCurrency<AccountId>>::Balance;

    fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
        if TestKey::contains(&currency_id) {
            A::minimum_balance(currency_id)
        } else {
            B::minimum_balance(currency_id)
        }
    }

    fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
        if TestKey::contains(&currency_id) {
            A::total_issuance(currency_id)
        } else {
            B::total_issuance(currency_id)
        }
    }

    fn total_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance {
        if TestKey::contains(&currency_id) {
            A::total_balance(currency_id, who)
        } else {
            B::total_balance(currency_id, who)
        }
    }

    fn free_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance {
        if TestKey::contains(&currency_id) {
            A::free_balance(currency_id, who)
        } else {
            B::free_balance(currency_id, who)
        }
    }

    fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        if TestKey::contains(&currency_id) {
            A::ensure_can_withdraw(currency_id, who, amount)
        } else {
            B::ensure_can_withdraw(currency_id, who, amount)
        }
    }

    fn transfer(
        currency_id: Self::CurrencyId,
        from: &AccountId,
        to: &AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if TestKey::contains(&currency_id) {
            A::transfer(currency_id, from, to, amount)
        } else {
            B::transfer(currency_id, from, to, amount)
        }
    }

    fn deposit(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        if TestKey::contains(&currency_id) {
            A::deposit(currency_id, who, amount)
        } else {
            B::deposit(currency_id, who, amount)
        }
    }

    fn withdraw(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        if TestKey::contains(&currency_id) {
            A::withdraw(currency_id, who, amount)
        } else {
            B::withdraw(currency_id, who, amount)
        }
    }

    fn can_slash(currency_id: Self::CurrencyId, who: &AccountId, value: Self::Balance) -> bool {
        if TestKey::contains(&currency_id) {
            A::can_slash(currency_id, who, value)
        } else {
            B::can_slash(currency_id, who, value)
        }
    }

    fn slash(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> Self::Balance {
        if TestKey::contains(&currency_id) {
            A::slash(currency_id, who, amount)
        } else {
            B::slash(currency_id, who, amount)
        }
    }
}

pub struct Mapper<AccountId, C, T>(PhantomData<(AccountId, C, T)>);

impl<AccountId, C, T> MultiCurrency<AccountId> for Mapper<AccountId, C, T>
where
    C: ConvertBalance<
        <T as MultiCurrency<AccountId>>::Balance,
        <T as MultiCurrency<AccountId>>::Balance,
        AssetId = CurrencyId,
    >,
    T: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
    type CurrencyId = CurrencyId;
    type Balance = <T as MultiCurrency<AccountId>>::Balance;

    fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
        C::convert_balance(T::minimum_balance(currency_id), currency_id)
    }

    fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
        C::convert_balance(T::total_issuance(currency_id), currency_id)
    }

    fn total_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance {
        C::convert_balance(T::total_balance(currency_id, who), currency_id)
    }

    fn free_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance {
        C::convert_balance(T::free_balance(currency_id, who), currency_id)
    }

    fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        T::ensure_can_withdraw(currency_id, who, C::convert_balance_back(amount, currency_id))
    }

    fn transfer(
        currency_id: Self::CurrencyId,
        from: &AccountId,
        to: &AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        T::transfer(currency_id, from, to, C::convert_balance_back(amount, currency_id))
    }

    fn deposit(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        T::deposit(currency_id, who, C::convert_balance_back(amount, currency_id))
    }

    fn withdraw(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        T::withdraw(currency_id, who, C::convert_balance_back(amount, currency_id))
    }

    fn can_slash(currency_id: Self::CurrencyId, who: &AccountId, value: Self::Balance) -> bool {
        T::can_slash(currency_id, who, C::convert_balance_back(value, currency_id))
    }

    fn slash(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> Self::Balance {
        T::slash(currency_id, who, C::convert_balance_back(amount, currency_id))
    }
}
