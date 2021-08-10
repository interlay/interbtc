use super::*;
use crate::{types::ReplaceRequest, Pallet as Replace};
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{
        BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
        TransactionOutput,
    },
};
use btc_relay::{BtcAddress, BtcPublicKey, Pallet as BtcRelay};
use exchange_rate_oracle::Pallet as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::CurrencyId;
use security::Pallet as Security;
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::One, FixedPointNumber};
use sp_std::{convert::TryInto, prelude::*};
use vault_registry::{
    types::{Vault, Wallet},
    Pallet as VaultRegistry,
};

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

fn mine_blocks<T: crate::Config>() {
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
    for _ in 0..100 {
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

benchmarks! {
    request_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault_id, (1u32 << 31).into());
        let amount = Replace::<T>::replace_btc_dust_value() + 1000u32.into();
        // TODO: calculate from exchange rate
        let griefing = 1000u32.into();

        let vault = Vault {
            wallet: Wallet::new(dummy_public_key()),
            id: vault_id.clone(),
            issued_tokens: amount,
            ..Vault::new(Default::default(), Default::default(), T::GetGriefingCollateralCurrencyId::get())
        };

        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        ExchangeRateOracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ).unwrap();
    }: _(RawOrigin::Signed(vault_id), amount, griefing)

    withdraw_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&vault_id, (1u32 << 31).into());
        let amount = 5u32;
        VaultRegistry::<T>::_register_vault(&vault_id, 100000000u32.into(), dummy_public_key(), T::GetGriefingCollateralCurrencyId::get()).unwrap();

        let threshold = <T as vault_registry::Config>::UnsignedFixedPoint::one();
        VaultRegistry::<T>::set_secure_collateral_threshold(DEFAULT_TESTING_CURRENCY, threshold);
        ExchangeRateOracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_replaced_tokens(&vault_id, amount.into(), 1000u32.into()).unwrap();

        // TODO: check that an amount was actually withdrawn
    }: _(RawOrigin::Signed(vault_id), amount.into())

    accept_replace {
        let old_vault_id: T::AccountId = account("Origin", 0, 0);
        let new_vault_id: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&old_vault_id, (1u32 << 31).into());
        mint_collateral::<T>(&new_vault_id, (1u32 << 31).into());
        let dust_value =  Replace::<T>::replace_btc_dust_value().try_into().unwrap_or(0u32);
        let amount: u32 = dust_value + 100u32;
        let collateral: u32 = 1000;

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));

        VaultRegistry::<T>::set_secure_collateral_threshold(DEFAULT_TESTING_CURRENCY, <T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());
        ExchangeRateOracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();
        VaultRegistry::<T>::_register_vault(&old_vault_id, 100000000u32.into(), dummy_public_key(), T::GetGriefingCollateralCurrencyId::get()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_replaced_tokens(&old_vault_id, amount.into(), 1000u32.into()).unwrap();

        VaultRegistry::<T>::_register_vault(&new_vault_id, 100000000u32.into(), dummy_public_key(), T::GetGriefingCollateralCurrencyId::get()).unwrap();

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.amount = amount.into();
        Replace::<T>::insert_replace_request(&replace_id, &replace_request);


        ExchangeRateOracle::<T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY,
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ).unwrap();
    }: _(RawOrigin::Signed(new_vault_id), old_vault_id, amount.into(), collateral.into(), new_vault_btc_address)

    execute_replace {
        let new_vault_id: T::AccountId = account("Origin", 0, 0);
        let old_vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));
        let old_vault_btc_address = BtcAddress::P2SH(H160([1; 20]));

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.new_vault = new_vault_id.clone();
        replace_request.btc_address = old_vault_btc_address;

        Replace::<T>::insert_replace_request(&replace_id, &replace_request);

        let old_vault = Vault {
            wallet: Wallet::new(dummy_public_key()),
            id: old_vault_id.clone(),
            ..Vault::new(Default::default(), Default::default(), T::GetGriefingCollateralCurrencyId::get())
        };
        VaultRegistry::<T>::insert_vault(
            &old_vault_id,
            old_vault
        );

        let new_vault = Vault {
            wallet: Wallet::new(dummy_public_key()),
            id: new_vault_id.clone(),
            ..Vault::new(Default::default(), Default::default(), T::GetGriefingCollateralCurrencyId::get())
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

    }: _(RawOrigin::Signed(old_vault_id), replace_id, proof, raw_tx)

    cancel_replace {
        let new_vault_id: T::AccountId = account("Origin", 0, 0);
        let old_vault_id: T::AccountId = account("Vault", 0, 0);
        mint_collateral::<T>(&new_vault_id, (1u32 << 31).into());
        mint_collateral::<T>(&old_vault_id, (1u32 << 31).into());

        let amount:u32 = 100;

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.new_vault = new_vault_id.clone();
        replace_request.amount = amount.into();
        Replace::<T>::insert_replace_request(&replace_id, &replace_request);
        mine_blocks::<T>();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Replace::<T>::replace_period() + 10u32.into());

        VaultRegistry::<T>::set_secure_collateral_threshold(DEFAULT_TESTING_CURRENCY, <T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());

        ExchangeRateOracle:: <T>::_set_exchange_rate(DEFAULT_TESTING_CURRENCY, <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();

        VaultRegistry::<T>::_register_vault(&old_vault_id, 100000000u32.into(), dummy_public_key(), T::GetGriefingCollateralCurrencyId::get()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_redeemed_tokens(&old_vault_id, amount.into()).unwrap();

        VaultRegistry::<T>::_register_vault(&new_vault_id, 100000000u32.into(), dummy_public_key(), T::GetGriefingCollateralCurrencyId::get()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&new_vault_id, amount.into()).unwrap();

    }: _(RawOrigin::Signed(new_vault_id), replace_id)

    set_replace_period {
    }: _(RawOrigin::Root, 1u32.into())

}

impl_benchmark_test_suite!(
    Replace,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
