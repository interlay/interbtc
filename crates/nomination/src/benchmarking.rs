use super::*;
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use sp_runtime::traits::One;
use vault_registry::BtcPublicKey;

// Pallets
use crate::Pallet as Nomination;
use oracle::Pallet as Oracle;
use vault_registry::Pallet as VaultRegistry;

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn setup_exchange_rate<T: crate::Config>() {
    Oracle::<T>::_set_exchange_rate(
        get_collateral_currency_id::<T>(),
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();
}

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

fn register_vault<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(
        vault_id.clone(),
        100000000u32.into()
    ));
}

benchmarks! {
    set_nomination_enabled {
    }: _(RawOrigin::Root, true)

    opt_in_to_nomination {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault::<T>(vault_id.clone());

    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone())

    opt_out_of_nomination {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

    }: _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone())

    deposit_collateral {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

        let nominator: T::AccountId = account("Nominator", 0, 0);
        mint_collateral::<T>(&nominator, (1u32 << 31).into());
        let amount = 100u32.into();

    }: _(RawOrigin::Signed(nominator), vault_id, amount)

    withdraw_collateral {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

        let nominator: T::AccountId = account("Nominator", 0, 0);
        mint_collateral::<T>(&nominator, (1u32 << 31).into());
        let amount = 100u32.into();

        assert_ok!(Nomination::<T>::_deposit_collateral(&vault_id, &nominator, amount));

    }: _(RawOrigin::Signed(nominator), vault_id, amount, None)
}

impl_benchmark_test_suite!(
    Nomination,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
