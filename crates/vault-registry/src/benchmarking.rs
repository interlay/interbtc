use super::*;
use crate::{types::BtcPublicKey, Pallet as VaultRegistry};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use oracle::Pallet as Oracle;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use sp_std::prelude::*;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn wrapped<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_wrapped_currency_id::<T>())
}

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

fn get_currency_pair<T: crate::Config>() -> DefaultVaultCurrencyPair<T> {
    VaultCurrencyPair {
        collateral: get_collateral_currency_id::<T>(),
        wrapped: get_wrapped_currency_id::<T>(),
    }
}

fn register_vault_with_collateral<T: crate::Config>(vault_id: DefaultVaultId<T>, collateral: u32) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(vault_id.clone(), collateral.into()));
}

benchmarks! {
    register_vault {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount: u32 = 100;
        let origin = RawOrigin::Signed(vault_id.account_id.clone());
        let public_key = BtcPublicKey::default();
        VaultRegistry::<T>::register_public_key(origin.clone().into(), public_key).unwrap();
    }: _(origin, vault_id.currencies.clone(), amount.into())

    deposit_collateral {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = 100u32.into();
        register_vault_with_collateral::<T>(vault_id.clone(), 100000000);
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), amount)

    withdraw_collateral {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = 100u32.into();
        register_vault_with_collateral::<T>(vault_id.clone(), 100000000);
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), amount)

    register_public_key {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
    }: _(RawOrigin::Signed(vault_id.account_id), BtcPublicKey::default())

    accept_new_issues {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault_with_collateral::<T>(vault_id.clone(), 100000000);
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), true)

    set_custom_secure_threshold {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault_with_collateral::<T>(vault_id.clone(), 100000000);
        VaultRegistry::<T>::_set_secure_collateral_threshold(vault_id.currencies.clone(), UnsignedFixedPoint::<T>::zero());
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone(), Some(UnsignedFixedPoint::<T>::one()))

    set_minimum_collateral {
    }: _(RawOrigin::Root, get_collateral_currency_id::<T>(), 1234u32.into())

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

        register_vault_with_collateral::<T>(vault_id.clone(), 10_000);
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &wrapped(5_000)).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(5_000)).unwrap();

        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::checked_from_rational(10, 1).unwrap()).unwrap();
    }: _(RawOrigin::Signed(origin), vault_id.clone())

    recover_vault_id {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault_with_collateral::<T>(vault_id.clone(), 100000000);
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::checked_from_rational(10, 1).unwrap()).unwrap();
        VaultRegistry::<T>::liquidate_vault(&vault_id).unwrap();
    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone())
}

impl_benchmark_test_suite!(
    VaultRegistry,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
