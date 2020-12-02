use super::*;
use crate::types::ReplaceRequest;
use crate::Module as Replace;
use bitcoin::formatter::Formattable;
use bitcoin::types::{
    BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput,
};
use btc_relay::BtcAddress;
use btc_relay::Module as BtcRelay;
use collateral::Module as Collateral;
use exchange_rate_oracle::Module as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks};
use frame_system::Module as System;
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;
use vault_registry::types::{Vault, Wallet};
use vault_registry::Module as VaultRegistry;

benchmarks! {
    _ {}

    request_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = Replace::<T>::replace_btc_dust_value() + 1000.into();
        let griefing = Replace::<T>::replace_griefing_collateral() + 1000.into();

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        vault.issued_tokens = amount;
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(vault_id), amount, griefing)

    withdraw_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = vault_id.clone();
        Replace::<T>::insert_replace_request(replace_id, replace_request);

    }: _(RawOrigin::Signed(vault_id), replace_id)

    accept_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = 100;
        let collateral = 1000;

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

        Collateral::<T>::lock_collateral(&vault_id, 100000000.into()).unwrap();
        ExchangeRateOracle::<T>::_set_exchange_rate(1).unwrap();
        VaultRegistry::<T>::_set_secure_collateral_threshold(1);

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = vault_id.clone();
        replace_request.amount = amount.into();
        Replace::<T>::insert_replace_request(replace_id, replace_request);

    }: _(RawOrigin::Signed(vault_id), replace_id, collateral.into())

    auction_replace {
        let old_vault_id: T::AccountId = account("Origin", 0, 0);
        let new_vault_id: T::AccountId = account("Vault", 0, 0);
        let btc_amount = 100;
        let collateral = 1000;

        let mut old_vault = Vault::default();
        old_vault.id = old_vault_id.clone();
        old_vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        old_vault.issued_tokens = 123897.into();
        VaultRegistry::<T>::_insert_vault(
            &old_vault_id,
            old_vault
        );

        let mut new_vault = Vault::default();
        new_vault.id = new_vault_id.clone();
        new_vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        VaultRegistry::<T>::_insert_vault(
            &new_vault_id,
            new_vault
        );

        ExchangeRateOracle::<T>::_set_exchange_rate(1).unwrap();
        VaultRegistry::<T>::_set_auction_collateral_threshold(1);
        VaultRegistry::<T>::_set_secure_collateral_threshold(1);

    }: _(RawOrigin::Signed(new_vault_id), old_vault_id, btc_amount.into(), collateral.into())

    execute_replace {
        let new_vault_id: T::AccountId = account("Origin", 0, 0);
        let old_vault_id: T::AccountId = account("Vault", 0, 0);

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));
        let old_vault_btc_address = BtcAddress::P2SH(H160([1; 20]));

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.new_vault = Some(new_vault_id.clone());
        replace_request.btc_address = old_vault_btc_address;
        Replace::<T>::insert_replace_request(replace_id, replace_request);

        let mut old_vault = Vault::default();
        old_vault.id = old_vault_id.clone();
        old_vault.wallet = Wallet::new(old_vault_btc_address);
        VaultRegistry::<T>::_insert_vault(
            &old_vault_id,
            old_vault
        );

        let mut new_vault = Vault::default();
        new_vault.id = new_vault_id.clone();
        new_vault.wallet = Wallet::new(new_vault_btc_address);
        VaultRegistry::<T>::_insert_vault(
            &new_vault_id,
            new_vault
        );

        let mut height = 0;

        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash = block.header.hash();
        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_initialize(block_header, height).unwrap();

        height += 1;

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_previous_hash(block.transactions[0].hash())
                    .build(),
            )
            .add_output(TransactionOutput::payment(value.into(), &old_vault_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_store_block_header(block_header).unwrap();

    }: _(RawOrigin::Signed(old_vault_id), replace_id, tx_id, tx_block_height, proof, raw_tx)

    cancel_replace {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = vault_id.clone();
        Replace::<T>::insert_replace_request(replace_id, replace_request);
        System::<T>::set_block_number(System::<T>::block_number() + Replace::<T>::replace_period() + 10.into());

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160([0; 20])));
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), replace_id)

    set_replace_period {
    }: _(RawOrigin::Root, 1.into())

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build_with(pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (account("Origin", 0, 0), 1 << 32),
                (account("Vault", 0, 0), 1 << 32),
            ],
        })
        .execute_with(|| {
            assert_ok!(test_benchmark_request_replace::<Test>());
            assert_ok!(test_benchmark_withdraw_replace::<Test>());
            assert_ok!(test_benchmark_accept_replace::<Test>());
            assert_ok!(test_benchmark_auction_replace::<Test>());
            assert_ok!(test_benchmark_execute_replace::<Test>());
            assert_ok!(test_benchmark_cancel_replace::<Test>());
            assert_ok!(test_benchmark_set_replace_period::<Test>());
        });
    }
}
