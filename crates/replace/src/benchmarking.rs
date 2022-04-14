use super::*;
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{
        BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
        TransactionOutput,
    },
};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, VaultCurrencyPair, VaultId};
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::One, FixedPointNumber};
use sp_std::prelude::*;
use vault_registry::types::{DefaultVaultCurrencyPair, Vault, Wallet};

// Pallets
use crate::Pallet as Replace;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn wrapped<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_wrapped_currency_id::<T>())
}

fn get_currency_pair<T: crate::Config>() -> DefaultVaultCurrencyPair<T> {
    VaultCurrencyPair {
        collateral: get_collateral_currency_id::<T>(),
        wrapped: get_wrapped_currency_id::<T>(),
    }
}

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

fn mine_blocks_until_expiry<T: crate::Config>(request: &DefaultReplaceRequest<T>) {
    let period = Replace::<T>::replace_period().max(request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(request.btc_height, period).unwrap();
    mine_blocks::<T>(expiry_height + 100);
}

fn mine_blocks<T: crate::Config>(end_height: u32) {
    let relayer_id: T::AccountId = account("Relayer", 0, 0);
    mint_collateral::<T>(&relayer_id, (1u32 << 31).into());

    let height = 0;
    let block = BlockBuilder::new()
        .with_version(4)
        .with_coinbase(&BtcAddress::P2SH(H160::zero()), 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
    let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

    Security::<T>::set_active_block_number(1u32.into());
    BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(
            TransactionInputBuilder::new()
                .with_source(TransactionInputSource::FromOutput(block.transactions[0].hash(), 0))
                .with_script(&[
                    0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210, 186, 21, 187, 98,
                    38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232, 168, 2, 32, 72, 126, 179, 207,
                    142, 8, 99, 8, 32, 78, 244, 166, 106, 160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12,
                    194, 240, 212, 3, 120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247,
                    165, 78, 111, 80, 79, 50, 200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51,
                    33, 2, 128, 200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7, 84,
                    210, 85, 156, 238, 77, 97, 188, 240, 162, 197, 105, 62, 82, 174,
                ])
                .build(),
        )
        .build();

    let mut prev_hash = block.header.hash;
    for _ in 0..end_height {
        let block = BlockBuilder::new()
            .with_previous_hash(prev_hash)
            .with_version(4)
            .with_coinbase(&BtcAddress::P2SH(H160::zero()), 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()))
            .unwrap();
        prev_hash = block.header.hash;

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();
    }
}

fn test_request<T: crate::Config>(
    new_vault_id: &DefaultVaultId<T>,
    old_vault_id: &DefaultVaultId<T>,
) -> DefaultReplaceRequest<T> {
    ReplaceRequest {
        new_vault: new_vault_id.clone(),
        old_vault: old_vault_id.clone(),
        period: Default::default(),
        accept_time: Default::default(),
        amount: Default::default(),
        griefing_collateral: Default::default(),
        btc_address: Default::default(),
        collateral: Default::default(),
        btc_height: Default::default(),
        status: Default::default(),
    }
}

fn get_vault_id<T: crate::Config>(name: &'static str) -> DefaultVaultId<T> {
    VaultId::new(
        account(name, 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

fn register_public_key<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id);
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        dummy_public_key()
    ));
}

fn register_vault<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    register_public_key::<T>(vault_id.clone());
    assert_ok!(VaultRegistry::<T>::_register_vault(
        vault_id.clone(),
        100000000u32.into()
    ));
}

benchmarks! {
    request_replace {
        let vault_id = get_vault_id::<T>("Vault");
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = Replace::<T>::dust_value(get_wrapped_currency_id::<T>()).amount() + 1000u32.into();

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            wallet: Wallet::new(),
            id: vault_id.clone(),
            issued_tokens: amount,
            ..Vault::new(vault_id.clone())
        };

        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();
        VaultRegistry::<T>::_set_system_collateral_ceiling(vault_id.currencies.clone(), 1_000_000_000u32.into());
    }: _(RawOrigin::Signed(vault_id.account_id.clone()), vault_id.currencies.clone(), amount)

    withdraw_replace {
        let vault_id = get_vault_id::<T>("OldVault");
        mint_collateral::<T>(&vault_id.account_id, (1u32 << 31).into());
        let amount = wrapped(5);

        let threshold = UnsignedFixedPoint::<T>::one();
        VaultRegistry::<T>::_set_secure_collateral_threshold(get_currency_pair::<T>(), threshold);
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();
        VaultRegistry::<T>::_set_system_collateral_ceiling(get_currency_pair::<T>(), 1_000_000_000u32.into());

        register_vault::<T>(vault_id.clone());

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &amount).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, &amount).unwrap();
        VaultRegistry::<T>::try_increase_to_be_replaced_tokens(&vault_id, &amount).unwrap();

        // TODO: check that an amount was actually withdrawn
    }: _(RawOrigin::Signed(vault_id.account_id.clone()), vault_id.currencies.clone(), amount.amount())

    accept_replace {
        let new_vault_id = get_vault_id::<T>("NewVault");
        let old_vault_id = get_vault_id::<T>("OldVault");
        mint_collateral::<T>(&old_vault_id.account_id, (1u32 << 31).into());
        mint_collateral::<T>(&new_vault_id.account_id, (1u32 << 31).into());
        let dust_value =  Replace::<T>::dust_value(get_wrapped_currency_id::<T>());
        let amount = dust_value.checked_add(&wrapped(100u32)).unwrap();
        let griefing = 1000u32.into();

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));

        VaultRegistry::<T>::_set_secure_collateral_threshold(get_currency_pair::<T>(), UnsignedFixedPoint::<T>::checked_from_rational(1, 100000).unwrap());
        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();
        VaultRegistry::<T>::_set_system_collateral_ceiling(get_currency_pair::<T>(), 1_000_000_000u32.into());
        register_vault::<T>(old_vault_id.clone());

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, &amount).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, &amount).unwrap();
        VaultRegistry::<T>::try_increase_to_be_replaced_tokens(&old_vault_id, &amount).unwrap();

        register_vault::<T>(new_vault_id.clone());

        let replace_id = H256::zero();
        let mut replace_request = test_request::<T>(&old_vault_id, &old_vault_id);
        replace_request.amount = amount.amount();
        Replace::<T>::insert_replace_request(&replace_id, &replace_request);

        Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()
        ).unwrap();
    }: _(RawOrigin::Signed(new_vault_id.account_id.clone()), new_vault_id.currencies.clone(), old_vault_id, amount.amount(), griefing, new_vault_btc_address)

    execute_replace {
        let new_vault_id = get_vault_id::<T>("NewVault");
        let old_vault_id = get_vault_id::<T>("OldVault");
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));
        let old_vault_btc_address = BtcAddress::P2SH(H160([1; 20]));

        let replace_id = H256::zero();
        let mut replace_request = test_request::<T>(&new_vault_id, &old_vault_id);
        replace_request.btc_address = old_vault_btc_address;

        Replace::<T>::insert_replace_request(&replace_id, &replace_request);

        let old_vault = Vault {
            wallet: Wallet::new(),
            id: old_vault_id.clone(),
            ..Vault::new(old_vault_id.clone())
        };
        VaultRegistry::<T>::insert_vault(
            &old_vault_id,
            old_vault
        );

        let new_vault = Vault {
            wallet: Wallet::new(),
            id: new_vault_id.clone(),
            ..Vault::new(new_vault_id.clone())
        };
        VaultRegistry::<T>::insert_vault(
            &new_vault_id,
            new_vault
        );

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let block_hash = block.header.hash;
        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        Security::<T>::set_active_block_number(1u32.into());
        VaultRegistry::<T>::_set_system_collateral_ceiling(get_currency_pair::<T>(), 1_000_000_000u32.into());
        BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_source(TransactionInputSource::FromOutput(block.transactions[0].hash(), 0))
                    .with_script(&[
                        0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234,
                        210, 186, 21, 187, 98, 38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123,
                        216, 232, 168, 2, 32, 72, 126, 179, 207, 142, 8, 99, 8, 32, 78, 244, 166, 106,
                        160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3,
                        120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247,
                        165, 78, 111, 80, 79, 50, 200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253,
                        134, 127, 212, 51, 33, 2, 128, 200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54,
                        189, 164, 187, 243, 243, 152, 7, 84, 210, 85, 156, 238, 77, 97, 188, 240, 162,
                        197, 105, 62, 82, 174,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(value.into(), &old_vault_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(4)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into())).unwrap();

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&[tx_id]).unwrap().try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations() + 1u32.into());

    }: _(RawOrigin::Signed(old_vault_id.account_id), replace_id, proof, raw_tx)

    cancel_replace {
        let new_vault_id = get_vault_id::<T>("NewVault");
        let old_vault_id = get_vault_id::<T>("OldVault");
        mint_collateral::<T>(&new_vault_id.account_id, (1u32 << 31).into());
        mint_collateral::<T>(&old_vault_id.account_id, (1u32 << 31).into());

        let amount = wrapped(100);

        let replace_id = H256::zero();
        let mut replace_request = test_request::<T>(&new_vault_id, &old_vault_id);
        replace_request.amount = amount.amount();
        Replace::<T>::insert_replace_request(&replace_id, &replace_request);

        // expire replace request
        mine_blocks_until_expiry::<T>(&replace_request);
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Replace::<T>::replace_period() + 100u32.into());

        VaultRegistry::<T>::_set_secure_collateral_threshold(get_currency_pair::<T>(), UnsignedFixedPoint::<T>::checked_from_rational(1, 100000).unwrap());
        VaultRegistry::<T>::_set_system_collateral_ceiling(get_currency_pair::<T>(), 1_000_000_000u32.into());

        Oracle:: <T>::_set_exchange_rate(get_collateral_currency_id::<T>(), UnsignedFixedPoint::<T>::one()).unwrap();

        register_vault::<T>(old_vault_id.clone());
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, &amount).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, &amount).unwrap();
        VaultRegistry::<T>::try_increase_to_be_redeemed_tokens(&old_vault_id, &amount).unwrap();

        register_vault::<T>(new_vault_id.clone());
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&new_vault_id, &amount).unwrap();

    }: _(RawOrigin::Signed(new_vault_id.account_id), replace_id)

    set_replace_period {
    }: _(RawOrigin::Root, 1u32.into())

}

impl_benchmark_test_suite!(
    Replace,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
