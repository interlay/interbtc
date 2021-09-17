use super::*;
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

pub const DEFAULT_TESTING_CURRENCY: CurrencyId = CurrencyId::DOT;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: Collateral<T>) {
    <orml_tokens::Pallet<T>>::deposit(DEFAULT_TESTING_CURRENCY, account_id, amount).unwrap();
}

fn setup_exchange_rate<T: crate::Config>() {
    Oracle::<T>::_set_exchange_rate(
        DEFAULT_TESTING_CURRENCY,
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();
}

benchmarks! {
    set_nomination_enabled {
    }: _(RawOrigin::Root, true)

    opt_in_to_nomination {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault, (1u32 << 31).into());
        assert_ok!(VaultRegistry::<T>::_register_vault(&vault, 100000000u32.into(), dummy_public_key(), DEFAULT_TESTING_CURRENCY));

    }: _(RawOrigin::Signed(vault))

    opt_out_of_nomination {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault, (1u32 << 31).into());
        assert_ok!(VaultRegistry::<T>::_register_vault(&vault, 100000000u32.into(), dummy_public_key(), DEFAULT_TESTING_CURRENCY));

        <Vaults<T>>::insert(&vault, true);

    }: _(RawOrigin::Signed(vault))

    deposit_collateral {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault, (1u32 << 31).into());
        assert_ok!(VaultRegistry::<T>::_register_vault(&vault, 100000000u32.into(), dummy_public_key(), DEFAULT_TESTING_CURRENCY));

        <Vaults<T>>::insert(&vault, true);

        let nominator: T::AccountId = account("Nominator", 0, 0);
        mint_collateral::<T>(&nominator, (1u32 << 31).into());
        let amount = 100u32.into();

    }: _(RawOrigin::Signed(nominator), vault, amount)

    withdraw_collateral {
        setup_exchange_rate::<T>();
        <NominationEnabled<T>>::set(true);

        let vault: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault, (1u32 << 31).into());
        assert_ok!(VaultRegistry::<T>::_register_vault(&vault, 100000000u32.into(), dummy_public_key(), DEFAULT_TESTING_CURRENCY));

        <Vaults<T>>::insert(&vault, true);

        let nominator: T::AccountId = account("Nominator", 0, 0);
        mint_collateral::<T>(&nominator, (1u32 << 31).into());
        let amount = 100u32.into();

        assert_ok!(Nomination::<T>::_deposit_collateral(&vault, &nominator, amount));

    }: _(RawOrigin::Signed(nominator), vault, amount, None)

}

impl_benchmark_test_suite!(
    Nomination,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
