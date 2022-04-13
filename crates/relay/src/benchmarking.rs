use super::*;
use crate::types::BalanceOf;
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{
        BlockBuilder, H256Le, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
        TransactionOutput,
    },
};
use btc_relay::{BtcAddress, BtcPublicKey, Pallet as BtcRelay};
use currency::{
    getters::{get_relay_chain_currency_id as get_collateral_currency_id, *},
    Amount,
};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::Get};
use frame_system::RawOrigin;
use oracle::Pallet as Oracle;
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, VaultId};
use security::Pallet as Security;
use sp_core::{H160, U256};
use sp_runtime::traits::One;
use sp_std::prelude::*;
use vault_registry::{
    types::{Vault, Wallet},
    Pallet as VaultRegistry,
};

#[cfg(test)]
use crate::Pallet as Relay;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn register_public_key<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        dummy_public_key()
    ));
}

benchmarks! {

    initialize {
        let height = 0u32;
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake = 100u32;

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();
        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
    }: _(RawOrigin::Signed(origin), block_header, height)

    store_block_header {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let height = 0;
        let stake = 100u32;

        let init_block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let init_block_hash = init_block.header.hash;
        let raw_block_header = RawBlockHeader::from_bytes(&init_block.header.try_format().unwrap())
            .expect("could not serialize block header");
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        BtcRelay::<T>::initialize(origin.clone(), block_header, height).unwrap();

        let block = BlockBuilder::new()
            .with_previous_hash(init_block_hash)
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap())
            .expect("could not serialize block header");

    }: _(RawOrigin::Signed(origin), raw_block_header)

    report_vault_theft {
        let origin: T::AccountId = account("Origin", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let vault_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));

        let address = BtcAddress::P2PKH(H160([0; 20]));

        let vault_id: VaultId<T::AccountId, _> = VaultId::new(
            account("Vault", 0, 0),
            T::GetGriefingCollateralCurrencyId::get(),
            <T as currency::Config>::GetWrappedCurrencyId::get()
        );

        register_public_key::<T>(vault_id.clone());

        let mut vault = Vault {
            wallet: Wallet::new(),
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };
        vault.wallet.add_btc_address(vault_address);
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        VaultRegistry::<T>::_set_secure_collateral_threshold(vault_id.currencies.clone(), UnsignedFixedPoint::<T>::one());
        VaultRegistry::<T>::_set_system_collateral_ceiling(vault_id.currencies.clone(), 1_000_000_000u32.into());

        mint_collateral::<T>(&vault_id.account_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &Amount::new(1000u32.into(), T::GetGriefingCollateralCurrencyId::get())));

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let block_hash = block.header.hash;
        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        Security::<T>::set_active_block_number(1u32.into());
        BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                                        .with_sequence(4294967295)
                    .with_source(TransactionInputSource::FromOutput(H256Le::from_bytes_le(&[
                            193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76,
                            226, 9, 127, 8, 96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                        ]), 1))
                    .with_script(&[
                        73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197,
                        90, 159, 173, 211, 192, 115, 51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60,
                        161, 89, 147, 72, 2, 33, 0, 155, 72, 45, 127, 123, 77, 71, 154, 255, 98,
                        189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214, 61, 208, 62, 195,
                        239, 101, 30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101,
                        156, 155, 240, 63, 67, 252, 78, 160, 85, 243, 167, 28, 214, 12, 123, 31,
                        212, 116, 171, 87, 143, 153, 119, 250,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(value.into(), &address))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into())).unwrap();

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&[tx_id]).unwrap().try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() +
        BtcRelay::<T>::parachain_confirmations() + 1u32.into());

        Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            <T as currency::Config>::UnsignedFixedPoint::one()
        ).unwrap();
    }: _(RawOrigin::Signed(origin), vault_id, proof, raw_tx)
}

impl_benchmark_test_suite!(Relay, crate::mock::ExtBuilder::build_with(|_| {}), crate::mock::Test);
