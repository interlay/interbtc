use super::*;
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::CurrencyId;
use sp_core::Get;
use sp_runtime::traits::One;
use sp_std::vec;
use vault_registry::{
    benchmarking::{activate_lending_and_get_vault_id, mint_lend_tokens},
    BtcPublicKey,
};

// Pallets
use crate::Pallet as Nomination;
use fee::Pallet as Fee;
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

fn register_vault<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    VaultRegistry::<T>::set_minimum_collateral(
        RawOrigin::Root.into(),
        get_collateral_currency_id::<T>(),
        100_000u32.into(),
    )
    .unwrap();
    mint_collateral::<T>(&vault_id.account_id, (1000000000u32).into());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(
        vault_id.clone(),
        100000000u32.into()
    ));
}

fn distribute_rewards<T: fee::Config>(currency: CurrencyId) {
    let amount = Amount::<T>::new(1000u32.into(), currency);
    amount.mint_to(&Fee::<T>::fee_pool_account_id()).unwrap();
    Fee::<T>::distribute_rewards(&amount).unwrap();
}

#[benchmarks(where T: loans::Config)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn set_nomination_enabled() {
        #[extrinsic_call]
        _(RawOrigin::Root, true);
    }

    #[benchmark]
    pub fn set_nomination_limit() {
        let vault_id = activate_lending_and_get_vault_id::<T>();
        let amount = 100u32.into();
        #[extrinsic_call]
        _(
            RawOrigin::Signed(vault_id.account_id),
            vault_id.currencies.clone(),
            amount,
        );
    }

    #[benchmark]
    pub fn opt_in_to_nomination() {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = activate_lending_and_get_vault_id::<T>();
        register_vault::<T>(vault_id.clone());

        #[extrinsic_call]
        _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone());
    }

    #[benchmark]
    pub fn opt_out_of_nomination() {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = activate_lending_and_get_vault_id::<T>();
        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

        #[extrinsic_call]
        _(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone());
    }

    #[benchmark]
    pub fn deposit_collateral() {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = activate_lending_and_get_vault_id::<T>();

        Nomination::<T>::set_nomination_limit(
            RawOrigin::Signed(vault_id.account_id.clone()).into(),
            vault_id.currencies.clone(),
            (1u32 << 31).into(),
        )
        .unwrap();

        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

        let nominator: T::AccountId = account("Nominator", 0, 0);
        let collateral_currency = vault_id.collateral_currency();
        if collateral_currency.is_lend_token() {
            mint_lend_tokens::<T>(&nominator, collateral_currency);
        } else {
            mint_collateral::<T>(&nominator, (1u32 << 31).into());
        }
        let amount = 100u32.into();

        // only two reward currencies supported
        distribute_rewards::<T>(T::GetWrappedCurrencyId::get());
        distribute_rewards::<T>(T::GetNativeCurrencyId::get());

        let balance_before = <orml_tokens::Pallet<T>>::reserved_balance(collateral_currency, &vault_id.account_id);

        #[extrinsic_call]
        _(RawOrigin::Signed(nominator), vault_id.clone(), amount);

        let balance_after = <orml_tokens::Pallet<T>>::reserved_balance(collateral_currency, &vault_id.account_id);
        assert_eq!(balance_before + amount, balance_after);
    }

    #[benchmark]
    pub fn withdraw_collateral() {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault_id = activate_lending_and_get_vault_id::<T>();
        register_vault::<T>(vault_id.clone());

        <Vaults<T>>::insert(&vault_id, true);

        Nomination::<T>::set_nomination_limit(
            RawOrigin::Signed(vault_id.account_id.clone()).into(),
            vault_id.currencies.clone(),
            (1u32 << 31).into(),
        )
        .unwrap();

        let nominator: T::AccountId = account("Nominator", 0, 0);
        let collateral_currency = vault_id.collateral_currency();
        if collateral_currency.is_lend_token() {
            mint_lend_tokens::<T>(&nominator, collateral_currency);
        } else {
            mint_collateral::<T>(&nominator, (1u32 << 31).into());
        }
        let amount = 100u32.into();

        assert_ok!(Nomination::<T>::_deposit_collateral(&vault_id, &nominator, amount));

        // only two reward currencies supported
        distribute_rewards::<T>(T::GetWrappedCurrencyId::get());
        distribute_rewards::<T>(T::GetNativeCurrencyId::get());

        let balance_before = <orml_tokens::Pallet<T>>::reserved_balance(collateral_currency, &vault_id.account_id);

        #[extrinsic_call]
        _(RawOrigin::Signed(nominator.clone()), vault_id.clone(), amount, None);

        let balance_after = <orml_tokens::Pallet<T>>::reserved_balance(collateral_currency, &vault_id.account_id);
        assert_eq!(balance_before - amount, balance_after);
    }

    impl_benchmark_test_suite!(
        Nomination,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    );
}
