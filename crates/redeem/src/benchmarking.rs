use super::*;
use bitcoin::{
    formatter::TryFormat,
    types::{BlockBuilder, TransactionBuilder, TransactionInputBuilder, TransactionInputSource, TransactionOutput},
};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, VaultCurrencyPair, VaultId};
use sp_core::{H256, U256};
use sp_runtime::traits::One;
use sp_std::{fmt::Debug, prelude::*};
use vault_registry::types::Vault;

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

fn mine_blocks_until_expiry<T: crate::Config>(request: &DefaultRedeemRequest<T>) {
    let period = Redeem::<T>::redeem_period().max(request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(request.btc_height, period).unwrap();
    mine_blocks::<T>(expiry_height + 100);
}

fn mine_blocks<T: crate::Config>(end_height: u32) {
    let relayer_id: T::AccountId = account("Relayer", 0, 0);
    mint_collateral::<T>(&relayer_id, (1u32 << 31).into());

    let height = 0;
    let block = BlockBuilder::new()
        .with_version(4)
        .with_coinbase(&BtcAddress::dummy(), 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    Security::<T>::set_active_block_number(1u32.into());
    BtcRelay::<T>::_initialize(relayer_id.clone(), block.header, height).unwrap();

    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(
            TransactionInputBuilder::new()
                .with_source(TransactionInputSource::FromOutput(block.transactions[0].hash(), 0))
                .with_script(&[])
                .build(),
        )
        .build();

    let mut prev_hash = block.header.hash;
    for _ in 0..end_height {
        let block = BlockBuilder::new()
            .with_previous_hash(prev_hash)
            .with_version(4)
            .with_coinbase(&BtcAddress::dummy(), 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()))
            .unwrap();
        prev_hash = block.header.hash;

        BtcRelay::<T>::_store_block_header(&relayer_id, block.header).unwrap();
    }
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

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

#[benchmarks(
	where
		<<T as vault_registry::Config>::Balance as TryInto<i64>>::Error: Debug,
)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn request_redeem() {
        let caller = whitelisted_caller();
        let vault_id = get_vault_id::<T>();
        let amount = Redeem::<T>::redeem_btc_dust_value() * 100u32.into();
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
        let vault_id = get_vault_id::<T>();
        let amount = 1000;

        register_public_key::<T>(vault_id.clone());

        VaultRegistry::<T>::insert_vault(&vault_id, Vault::new(vault_id.clone()));

        mint_wrapped::<T>(&caller, amount.into());

        mint_collateral::<T>(&vault_id.account_id, 100_000u32.into());
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
        let currency_pair = VaultCurrencyPair {
            collateral: get_collateral_currency_id::<T>(),
            wrapped: get_wrapped_currency_id::<T>(),
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), currency_pair, amount.into());
    }

    #[benchmark]
    pub fn execute_redeem(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<2, 3>, b: Linear<1, 1_024>) {
        // we expect at least two hashes for payment + merkle root
        let height = h - 1; // remove the merkle root to get height
        let transactions_count = 2u32.pow(height);

        let vault_id = get_vault_id::<T>();
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

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&caller_btc_address, 50, 3)
            .with_timestamp(u32::MAX)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let block_hash = block.header.hash;

        Security::<T>::set_active_block_number(1u32.into());
        BtcRelay::<T>::_initialize(relayer_id.clone(), block.header, height).unwrap();

        let mut block_builder = BlockBuilder::new();
        block_builder
            .with_previous_hash(block_hash)
            .with_version(4)
            .with_coinbase(&caller_btc_address, 50, 3)
            .with_timestamp(u32::MAX);

        // we always have two txs for coinbase + payment
        for _ in 0..(transactions_count - 2) {
            block_builder.add_transaction(TransactionBuilder::new().with_version(2).build());
        }

        let mut transaction_builder = TransactionBuilder::new();

        // add tx inputs
        for _ in 0..i {
            transaction_builder.add_input(
                TransactionInputBuilder::new()
                    .with_source(TransactionInputSource::FromOutput(block.transactions[0].hash(), 0))
                    .with_script(&vec![0; b as usize])
                    .add_witness(&vec![0; 72]) // max signature size
                    .add_witness(&vec![0; 65]) // uncompressed public key
                    .build(),
            );
        }

        // we always need these outputs for redeem
        transaction_builder
            .with_version(2)
            .add_output(TransactionOutput::payment(
                redeem_request.amount_btc.try_into().unwrap(),
                &caller_btc_address,
            ))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()));

        // add return-to-self output
        if o == 3 {
            transaction_builder.add_output(TransactionOutput::payment(
                0u32.into(),
                &BtcAddress::P2PKH(sp_core::H160::zero()),
            ));
        }

        let transaction = transaction_builder.build();
        block_builder.add_transaction(transaction.clone());

        let block = block_builder.mine(U256::from(2).pow(254.into())).unwrap();
        let tx_id = transaction.tx_id();
        let merkle_proof = block.merkle_proof(&[tx_id]).unwrap();
        assert_eq!(merkle_proof.transactions_count, transactions_count);
        assert_eq!(merkle_proof.hashes.len() as u32, h);
        let mut bytes = vec![];
        assert_ok!(transaction.try_format(&mut bytes));
        let length_bound = bytes.len() as u32;

        BtcRelay::<T>::_store_block_header(&relayer_id, block.header).unwrap();
        Security::<T>::set_active_block_number(
            Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations() + 1u32.into(),
        );

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
        let vault_id = get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = caller.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        redeem_request.btc_height = BtcRelay::<T>::get_best_block_height();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);
        mint_and_reserve_wrapped::<T>(&redeem_request.redeemer, redeem_request.amount_btc);

        // expire redeem request
        mine_blocks_until_expiry::<T>(&redeem_request);
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
        let vault_id = get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = caller.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);
        mint_and_reserve_wrapped::<T>(&redeem_request.redeemer, redeem_request.amount_btc);

        // expire redeem request
        mine_blocks_until_expiry::<T>(&redeem_request);
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

        let vault_id = get_vault_id::<T>();
        let caller = vault_id.account_id.clone();
        let amount = 1000;

        register_public_key::<T>(vault_id.clone());

        VaultRegistry::<T>::insert_vault(&vault_id, Vault::new(vault_id.clone()));

        mint_wrapped::<T>(&caller, amount.into());

        mint_collateral::<T>(&vault_id.account_id, 100_000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(
            &vault_id,
            &collateral(100_000)
        ));

        assert_ok!(VaultRegistry::<T>::try_increase_to_be_issued_tokens(
            &vault_id,
            &wrapped(amount)
        ));
        assert_ok!(VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(amount)));

        let currency_pair = VaultCurrencyPair {
            collateral: get_collateral_currency_id::<T>(),
            wrapped: get_wrapped_currency_id::<T>(),
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), currency_pair, amount.into());
    }

    impl_benchmark_test_suite!(
        Redeem,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    );
}
