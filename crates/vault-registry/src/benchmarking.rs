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
    Amount::new(amount.into(), T::GetWrappedCurrencyId::get())
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

benchmarks! {
    register_vault {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let amount: u32 = 100;
        let public_key = BtcPublicKey::default();
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
    }: _(RawOrigin::Signed(origin.clone()), amount.into(), public_key, currency_id)

    deposit_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())

    withdraw_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, u.into(), dummy_public_key(), currency_id).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())

    update_public_key {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), BtcPublicKey::default())

    register_address {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), BtcAddress::default())

    accept_new_issues {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), true)

    adjust_collateral_ceiling {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), currency_id, 1234u32.into())

    adjust_secure_collateral_threshold {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), currency_id, UnsignedFixedPoint::<T>::one())

    adjust_premium_redeem_threshold {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), currency_id, UnsignedFixedPoint::<T>::one())

    adjust_liquidation_collateral_threshold {
        let origin: T::AccountId = account("Origin", 0, 0);
        mint_collateral::<T>(&origin, (1u32 << 31).into());
        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key(), currency_id).unwrap();
    }: _(RawOrigin::Signed(origin), currency_id, UnsignedFixedPoint::<T>::one())

    report_undercollateralized_vault {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault_id, (1u32 << 31).into());

        let currency_id = T::GetGriefingCollateralCurrencyId::get();
        VaultRegistry::<T>::_register_vault(&vault_id, 10_000u32.into(), dummy_public_key(), currency_id).unwrap();
        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, UnsignedFixedPoint::<T>::one()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &wrapped(5_000)).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(5_000)).unwrap();

        Oracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, UnsignedFixedPoint::<T>::checked_from_rational(10, 1).unwrap()).unwrap();
    }: _(RawOrigin::Signed(origin), vault_id)
}

impl_benchmark_test_suite!(
    VaultRegistry,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
