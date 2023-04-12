use super::*;
use crate::{types::BtcPublicKey, Pallet as VaultRegistry};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use oracle::Pallet as Oracle;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

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

fn set_default_exchange_rate<T: crate::Config>() {
    <oracle::Pallet<T>>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();
}

fn register_vault_with_collateral<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    VaultRegistry::<T>::set_minimum_collateral(
        RawOrigin::Root.into(),
        get_collateral_currency_id::<T>(),
        100_000u32.into(),
    )
    .unwrap();
    let amount = VaultRegistry::<T>::minimum_collateral_vault(vault_id.collateral_currency());
    assert!(!amount.is_zero());
    mint_collateral::<T>(&vault_id.account_id, amount);

    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    set_default_exchange_rate::<T>();
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(vault_id.clone(), amount));
}

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_vault() {
        let vault_id = get_vault_id::<T>();
        set_default_exchange_rate::<T>();
        let amount = VaultRegistry::<T>::minimum_collateral_vault(vault_id.collateral_currency());
        mint_collateral::<T>(&vault_id.account_id, amount);
        let origin = RawOrigin::Signed(vault_id.account_id.clone());
        let public_key = BtcPublicKey::default();
        VaultRegistry::<T>::register_public_key(origin.clone().into(), public_key).unwrap();

        #[extrinsic_call]
        register_vault(origin, vault_id.currencies.clone(), amount);
    }

    #[benchmark]
    fn register_public_key() {
        let vault_id = get_vault_id::<T>();
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());

        #[extrinsic_call]
        register_public_key(RawOrigin::Signed(vault_id.account_id), BtcPublicKey::default());
    }

    #[benchmark]
    fn accept_new_issues() {
        let vault_id = get_vault_id::<T>();
        register_vault_with_collateral::<T>(vault_id.clone());

        #[extrinsic_call]
        accept_new_issues(
            RawOrigin::Signed(vault_id.account_id),
            vault_id.currencies.clone(),
            true,
        );
    }

    #[benchmark]
    fn set_custom_secure_threshold() {
        let vault_id = get_vault_id::<T>();
        register_vault_with_collateral::<T>(vault_id.clone());
        VaultRegistry::<T>::_set_secure_collateral_threshold(
            vault_id.currencies.clone(),
            UnsignedFixedPoint::<T>::zero(),
        );

        #[extrinsic_call]
        set_custom_secure_threshold(
            RawOrigin::Signed(vault_id.account_id),
            vault_id.currencies.clone(),
            Some(UnsignedFixedPoint::<T>::one()),
        );
    }

    #[benchmark]
    fn set_minimum_collateral() {
        #[extrinsic_call]
        set_minimum_collateral(RawOrigin::Root, get_collateral_currency_id::<T>(), 1234u32.into());
    }

    #[benchmark]
    fn set_system_collateral_ceiling() {
        #[extrinsic_call]
        set_system_collateral_ceiling(RawOrigin::Root, get_currency_pair::<T>(), 1234u32.into());
    }

    #[benchmark]
    fn set_secure_collateral_threshold() {
        #[extrinsic_call]
        set_secure_collateral_threshold(
            RawOrigin::Root,
            get_currency_pair::<T>(),
            UnsignedFixedPoint::<T>::one(),
        );
    }

    #[benchmark]
    fn set_premium_redeem_threshold() {
        #[extrinsic_call]
        set_premium_redeem_threshold(
            RawOrigin::Root,
            get_currency_pair::<T>(),
            UnsignedFixedPoint::<T>::one(),
        );
    }

    #[benchmark]
    fn set_liquidation_collateral_threshold() {
        #[extrinsic_call]
        set_liquidation_collateral_threshold(
            RawOrigin::Root,
            get_currency_pair::<T>(),
            UnsignedFixedPoint::<T>::one(),
        );
    }

    #[benchmark]
    fn report_undercollateralized_vault() {
        let vault_id = get_vault_id::<T>();
        let origin: T::AccountId = account("Origin", 0, 0);

        register_vault_with_collateral::<T>(vault_id.clone());
        Oracle::<T>::_set_exchange_rate(
            vault_id.collateral_currency(),
            UnsignedFixedPoint::<T>::checked_from_rational(1, 1).unwrap(),
        )
        .unwrap();

        let amount = VaultRegistry::<T>::get_issuable_tokens_from_vault(&vault_id).unwrap();
        assert!(!amount.is_zero());
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &amount).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, &amount).unwrap();

        Oracle::<T>::_set_exchange_rate(
            vault_id.collateral_currency(),
            UnsignedFixedPoint::<T>::checked_from_rational(2147483647, 1).unwrap(),
        )
        .unwrap();

        #[extrinsic_call]
        report_undercollateralized_vault(RawOrigin::Signed(origin), vault_id.clone());
    }

    #[benchmark]
    fn recover_vault_id() {
        let vault_id = get_vault_id::<T>();
        register_vault_with_collateral::<T>(vault_id.clone());
        Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::checked_from_rational(10, 1).unwrap(),
        )
        .unwrap();
        VaultRegistry::<T>::liquidate_vault(&vault_id).unwrap();

        #[extrinsic_call]
        recover_vault_id(RawOrigin::Signed(vault_id.account_id), vault_id.currencies.clone());
    }

    impl_benchmark_test_suite! {
        VaultRegistry,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    }
}
