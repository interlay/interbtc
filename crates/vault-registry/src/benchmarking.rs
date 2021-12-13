use super::*;
use crate::{types::BtcPublicKey, Pallet as VaultRegistry};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use oracle::Pallet as Oracle;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use sp_std::prelude::*;

pub const DEFAULT_TESTING_CURRENCY: CurrencyId = CurrencyId::DOT;
type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn wrapped<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), <T as currency::Config>::GetWrappedCurrencyId::get())
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: Collateral<T>) {
    <orml_tokens::Pallet<T>>::deposit(DEFAULT_TESTING_CURRENCY, account_id, amount).unwrap();
}

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        T::GetGriefingCollateralCurrencyId::get(),
        <T as currency::Config>::GetWrappedCurrencyId::get(),
    )
}

fn get_currency_pair<T: crate::Config>() -> DefaultVaultCurrencyPair<T> {
    VaultCurrencyPair {
        collateral: T::GetGriefingCollateralCurrencyId::get(),
        wrapped: <T as currency::Config>::GetWrappedCurrencyId::get(),
    }
}

benchmarks! {
    register_vault {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount: u32 = 100;
        let public_key = BtcPublicKey::default();
    }: _(RawOrigin::Signed(vault_id.account_id.clone()), vault_id.currencies.clone(), amount.into(), public_key)

    deposit_collateral {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = 100u32.into();
        VaultRegistry::<T>::_register_vault(vault_id.clone(), amount, dummy_public_key()).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), amount)

    withdraw_collateral {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = 100u32.into();
        VaultRegistry::<T>::_register_vault(vault_id.clone(), amount, dummy_public_key()).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), amount)

    update_public_key {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        VaultRegistry::<T>::_register_vault(vault_id.clone(), 1234u32.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), BtcPublicKey::default())

    register_address {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        VaultRegistry::<T>::_register_vault(vault_id.clone(), 1234u32.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), BtcAddress::default())

    accept_new_issues {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        VaultRegistry::<T>::_register_vault(vault_id.clone(), 1234u32.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), true)

    set_system_collateral_ceiling {
    }: _(RawOrigin::Root, get_currency_pair::<T>(), 1234u32.into())

    set_secure_collateral_threshold {
    }: _(RawOrigin::Root, get_currency_pair::<T>(), UnsignedFixedPoint::<T>::one())

    set_premium_redeem_threshold {
    }: _(RawOrigin::Root, get_currency_pair::<T>(), UnsignedFixedPoint::<T>::one())

    set_liquidation_collateral_threshold {
    }: _(RawOrigin::Root, get_currency_pair::<T>(), UnsignedFixedPoint::<T>::one())

    report_undercollateralized_vault {
        let vault_id = get_vault_id::<T>();
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());

        VaultRegistry::<T>::_register_vault(vault_id.clone(), 10_000u32.into(), dummy_public_key()).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, UnsignedFixedPoint::<T>::one()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &wrapped(5_000)).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(5_000)).unwrap();

        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, UnsignedFixedPoint::<T>::checked_from_rational(10, 1).unwrap()).unwrap();
    }: _(RawOrigin::Signed(origin), vault_id.clone())
}

impl_benchmark_test_suite!(
    VaultRegistry,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
