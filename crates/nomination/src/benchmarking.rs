use super::*;
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, Rate, Ratio};
use sp_runtime::{traits::One, FixedPointNumber};
use sp_std::vec;
use vault_registry::BtcPublicKey;

// Pallets
use crate::Pallet as Nomination;
use loans::{InterestRateModel, JumpModel, Market, MarketState, Pallet as Loans};
use oracle::Pallet as Oracle;
use security::{Pallet as Security, StatusCode};
use traits::LoansApi;
use vault_registry::Pallet as VaultRegistry;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

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

pub const fn market_mock<T: loans::Config>(lend_token_id: CurrencyId) -> Market<u128> {
    Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        state: MarketState::Pending,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        lend_token_id,
    }
}

fn set_collateral_config<T: vault_registry::Config>(vault_id: &DefaultVaultId<T>) {
    VaultRegistry::<T>::_set_minimum_collateral_vault(vault_id.collateral_currency(), 0u32.into());
    VaultRegistry::<T>::_set_system_collateral_ceiling(vault_id.currencies.clone(), 1_000_000_000u32.into());
    VaultRegistry::<T>::_set_secure_collateral_threshold(vault_id.currencies.clone(), UnsignedFixedPoint::<T>::one());
    VaultRegistry::<T>::_set_premium_redeem_threshold(vault_id.currencies.clone(), UnsignedFixedPoint::<T>::one());
    VaultRegistry::<T>::_set_liquidation_collateral_threshold(
        vault_id.currencies.clone(),
        UnsignedFixedPoint::<T>::one(),
    );
}

fn activate_lending_and_get_vault_id<T: loans::Config + vault_registry::Config>() -> DefaultVaultId<T> {
    let account_id: T::AccountId = account("Vault", 0, 0);
    let lend_token = CurrencyId::LendToken(1);
    activate_lending_and_mint::<T>(get_collateral_currency_id::<T>(), lend_token.clone(), &account_id);
    let vault_id = VaultId::new(account("Vault", 0, 0), lend_token, get_wrapped_currency_id::<T>());
    set_collateral_config::<T>(&vault_id);
    vault_id
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

pub fn activate_market<T: loans::Config>(underlying_id: CurrencyId, lend_token_id: CurrencyId) {
    let origin = RawOrigin::Root;
    assert_ok!(Loans::<T>::add_market(
        origin.clone().into(),
        underlying_id,
        market_mock::<T>(lend_token_id)
    ));
    assert_ok!(Loans::<T>::activate_market(origin.into(), underlying_id,));
}

pub fn mint_lend_tokens<T: loans::Config>(account_id: &T::AccountId, lend_token_id: CurrencyId) {
    const LEND_TOKEN_FUNDING_AMOUNT: u128 = 1_000_000_000_000_000_000;
    let underlying_id = Loans::<T>::underlying_id(lend_token_id).unwrap();
    let amount: Amount<T> = Amount::new(LEND_TOKEN_FUNDING_AMOUNT, underlying_id);
    let origin = RawOrigin::Signed(account_id.clone());
    assert_ok!(amount.mint_to(&account_id));

    assert_ok!(Loans::<T>::mint(
        origin.into(),
        underlying_id,
        LEND_TOKEN_FUNDING_AMOUNT
    ));
}

pub fn activate_lending_and_mint<T: loans::Config>(
    underlying_id: CurrencyId,
    lend_token_id: CurrencyId,
    account_id: &T::AccountId,
) {
    activate_market::<T>(underlying_id, lend_token_id);
    mint_lend_tokens::<T>(account_id, lend_token_id);
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
        Security::<T>::set_status(StatusCode::Running);
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
        if vault_id.collateral_currency().is_lend_token() {
            mint_lend_tokens::<T>(&nominator, vault_id.collateral_currency());
        } else {
            mint_collateral::<T>(&nominator, (1u32 << 31).into());
        }
        let amount = 100u32.into();
        #[extrinsic_call]
        _(RawOrigin::Signed(nominator), vault_id, amount);
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
        if vault_id.collateral_currency().is_lend_token() {
            mint_lend_tokens::<T>(&nominator, vault_id.collateral_currency());
        } else {
            mint_collateral::<T>(&nominator, (1u32 << 31).into());
        }
        let amount = 100u32.into();

        assert_ok!(Nomination::<T>::_deposit_collateral(&vault_id, &nominator, amount));

        #[extrinsic_call]
        _(RawOrigin::Signed(nominator), vault_id, amount, None);
    }

    impl_benchmark_test_suite!(
        Nomination,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    );
}
