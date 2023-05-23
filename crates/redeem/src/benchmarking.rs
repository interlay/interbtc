use super::*;
use bitcoin::types::{BlockBuilder, TransactionOutput};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use sp_core::{H256, U256};
use sp_runtime::traits::One;
use sp_std::{fmt::Debug, prelude::*};
use vault_registry::{
    benchmarking::{activate_lending_and_get_vault_id, mint_lend_tokens},
    types::Vault,
};

// Pallets
use crate::Pallet as Redeem;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn collateral<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_collateral_currency_id::<T>())
}

fn wrapped<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_wrapped_currency_id::<T>())
}

fn register_public_key<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let caller = RawOrigin::Signed(vault_id.account_id.clone());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        caller.into(),
        BtcPublicKey::dummy()
    ));
}

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn mint_wrapped<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    let rich_amount = Amount::<T>::new(amount, get_wrapped_currency_id::<T>());
    assert_ok!(rich_amount.mint_to(account_id));
}

fn mint_and_reserve_wrapped<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    let rich_amount = Amount::<T>::new(amount, get_wrapped_currency_id::<T>());
    assert_ok!(rich_amount.mint_to(account_id));
    assert_ok!(rich_amount.lock_on(account_id));
}

fn initialize_oracle<T: crate::Config>() {
    let oracle_id: T::AccountId = account("Oracle", 12, 0);

    Oracle::<T>::_feed_values(
        oracle_id,
        vec![
            (
                OracleKey::ExchangeRate(get_collateral_currency_id::<T>()),
                UnsignedFixedPoint::<T>::checked_from_rational(1, 1).unwrap(),
            ),
            (
                OracleKey::FeeEstimation,
                UnsignedFixedPoint::<T>::checked_from_rational(3, 1).unwrap(),
            ),
        ],
    );
    Oracle::<T>::begin_block(0u32.into());
}

fn initialize_and_mine_blocks_until_expiry<T: crate::Config>(request: &DefaultRedeemRequest<T>) {
    let relayer_id: T::AccountId = account("Relayer", 0, 0);
    mint_collateral::<T>(&relayer_id, (1u32 << 31).into());
    Security::<T>::set_active_block_number(1u32.into());

    let period = Redeem::<T>::redeem_period().max(request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(request.btc_height, period).unwrap();

    let init_block = BlockBuilder::new()
        .with_version(4)
        .with_coinbase(&BtcAddress::default(), 50, 3)
        .with_timestamp(u32::MAX)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    BtcRelay::<T>::_initialize(relayer_id.clone(), init_block.header, 0).unwrap();
    BtcRelay::<T>::mine_blocks(&relayer_id, expiry_height + 100);
}

fn test_request<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> DefaultRedeemRequest<T> {
    RedeemRequest {
        vault: vault_id.clone(),
        opentime: Default::default(),
        period: Default::default(),
        fee: Default::default(),
        transfer_fee_btc: Default::default(),
        amount_btc: Redeem::<T>::redeem_btc_dust_value() * 100u32.into(),
        premium: Default::default(),
        redeemer: account("Redeemer", 0, 0),
        btc_address: Default::default(),
        btc_height: Default::default(),
        status: Default::default(),
    }
}

fn mint_vault_collateral<T: crate::Config + loans::Config>(vault_id: &DefaultVaultId<T>) {
    if vault_id.collateral_currency().is_lend_token() {
        mint_lend_tokens::<T>(&vault_id.account_id, vault_id.collateral_currency());
    } else {
        mint_collateral::<T>(&vault_id.account_id, 100_000u32.into());
    }
}

#[benchmarks(
	where
    T: loans::Config,
		<<T as currency::Config>::Balance as TryInto<i64>>::Error: Debug,
)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn request_redeem() {
        let caller = whitelisted_caller();
        let vault_id = activate_lending_and_get_vault_id::<T>();
        let amount = Redeem::<T>::redeem_btc_dust_value() * BalanceOf::<T>::from(100u32);
        let btc_address = BtcAddress::dummy();

        initialize_oracle::<T>();

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            issued_tokens: amount,
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };

        VaultRegistry::<T>::insert_vault(&vault_id, vault);

        mint_wrapped::<T>(&caller, amount);

        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), amount, btc_address, vault_id.clone());
    }

    #[benchmark]
    pub fn liquidation_redeem() {
        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        let caller = whitelisted_caller();
        let vault_id = activate_lending_and_get_vault_id::<T>();
        let amount = 1000;

        register_public_key::<T>(vault_id.clone());

        VaultRegistry::<T>::insert_vault(&vault_id, Vault::new(vault_id.clone()));

        mint_wrapped::<T>(&caller, amount.into());

        mint_vault_collateral::<T>(&vault_id);
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(
            &vault_id,
            &collateral(100_000)
        ));

        assert_ok!(VaultRegistry::<T>::try_increase_to_be_issued_tokens(
            &vault_id,
            &wrapped(amount)
        ));
        assert_ok!(VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(amount)));

        VaultRegistry::<T>::liquidate_vault(&vault_id).unwrap();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id.currencies, amount.into());
    }

    #[benchmark]
    pub fn execute_redeem(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<2, 3>, b: Linear<541, 2_048>) {
        let vault_id = activate_lending_and_get_vault_id::<T>();
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        initialize_oracle::<T>();

        let caller_btc_address = BtcAddress::dummy();
        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.btc_address = caller_btc_address;
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);
        mint_and_reserve_wrapped::<T>(&redeem_request.redeemer, redeem_request.amount_btc);

        register_public_key::<T>(vault_id.clone());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            Vault {
                id: vault_id.clone(),
                issued_tokens: redeem_request.amount_btc,
                to_be_redeemed_tokens: redeem_request.amount_btc,
                ..Vault::new(vault_id.clone())
            },
        );

        // we always need these outputs for redeem
        let mut outputs = vec![
            TransactionOutput::payment(redeem_request.amount_btc.try_into().unwrap(), &caller_btc_address),
            TransactionOutput::op_return(0, H256::zero().as_bytes()),
        ];

        // add return-to-self output
        if o == 3 {
            outputs.push(TransactionOutput::payment(
                0u32.into(),
                &BtcAddress::P2PKH(sp_core::H160::zero()),
            ));
        }

        let (transaction, merkle_proof) =
            BtcRelay::<T>::initialize_and_store_max(relayer_id.clone(), h, i, outputs, b as usize);
        let length_bound = transaction.size_no_witness() as u32;

        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(vault_id.account_id.clone()),
            redeem_id,
            merkle_proof,
            transaction,
            length_bound,
        );
    }

    #[benchmark]
    pub fn cancel_redeem_reimburse() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = activate_lending_and_get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = caller.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        redeem_request.btc_height = BtcRelay::<T>::get_best_block_height();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);
        mint_and_reserve_wrapped::<T>(&redeem_request.redeemer, redeem_request.amount_btc);

        // expire redeem request
        initialize_and_mine_blocks_until_expiry::<T>(&redeem_request);
        Security::<T>::set_active_block_number(
            Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 100u32.into(),
        );

        register_public_key::<T>(vault_id.clone());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            Vault {
                id: vault_id.clone(),
                issued_tokens: redeem_request.amount_btc,
                to_be_redeemed_tokens: redeem_request.amount_btc,
                ..Vault::new(vault_id.clone())
            },
        );

        mint_collateral::<T>(&vault_id.account_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &collateral(1000)));

        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        #[extrinsic_call]
        cancel_redeem(RawOrigin::Signed(caller), redeem_id, true);
    }

    #[benchmark]
    pub fn cancel_redeem_retry() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = activate_lending_and_get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = caller.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);
        mint_and_reserve_wrapped::<T>(&redeem_request.redeemer, redeem_request.amount_btc);

        // expire redeem request
        initialize_and_mine_blocks_until_expiry::<T>(&redeem_request);
        Security::<T>::set_active_block_number(
            Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 100u32.into(),
        );

        register_public_key::<T>(vault_id.clone());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            Vault {
                id: vault_id.clone(),
                issued_tokens: redeem_request.amount_btc,
                to_be_redeemed_tokens: redeem_request.amount_btc,
                ..Vault::new(vault_id.clone())
            },
        );

        mint_collateral::<T>(&vault_id.account_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &collateral(1000)));

        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        #[extrinsic_call]
        cancel_redeem(RawOrigin::Signed(caller), redeem_id, false);
    }

    #[benchmark]
    pub fn set_redeem_period() {
        #[extrinsic_call]
        _(RawOrigin::Root, 1u32.into());
    }

    #[benchmark]
    pub fn self_redeem() {
        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        let vault_id = activate_lending_and_get_vault_id::<T>();
        let caller = vault_id.account_id.clone();
        let amount = 1000;

        register_public_key::<T>(vault_id.clone());

        VaultRegistry::<T>::insert_vault(&vault_id, Vault::new(vault_id.clone()));

        mint_wrapped::<T>(&caller, amount.into());

        if vault_id.collateral_currency().is_lend_token() {
            mint_lend_tokens::<T>(&vault_id.account_id, vault_id.collateral_currency());
        } else {
            mint_collateral::<T>(&vault_id.account_id, 100_000u32.into());
        }
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(
            &vault_id,
            &collateral(100_000)
        ));

        assert_ok!(VaultRegistry::<T>::try_increase_to_be_issued_tokens(
            &vault_id,
            &wrapped(amount)
        ));
        assert_ok!(VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(amount)));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id.currencies, amount.into());
    }

    impl_benchmark_test_suite!(
        Redeem,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    );
}
