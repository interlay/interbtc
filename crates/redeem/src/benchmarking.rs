use super::*;
use crate::Module as Redeem;
use bitcoin::formatter::Formattable;
use bitcoin::types::{
    BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput,
};
use btc_relay::BtcAddress;
use btc_relay::Module as BtcRelay;
use frame_benchmarking::{account, benchmarks};
use frame_system::Module as System;
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;
use vault_registry::types::{Vault, Wallet};
use vault_registry::Module as VaultRegistry;

benchmarks! {
    _ {}

    request_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = Redeem::<T>::redeem_btc_dust_value() + 1000.into();
        let btc_address = BtcAddress::P2SH(H160::from([0; 20]));

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160::from([0; 20])));
        vault.issued_tokens = amount;
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), amount, btc_address, vault_id.clone())

    execute_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let origin_btc_address = BtcAddress::P2PKH(H160::zero());
        let vault_btc_address = BtcAddress::P2SH(H160::zero());

        let redeem_id = H256::zero();
        let mut redeem_request = RedeemRequest::default();
        redeem_request.vault = vault_id.clone();
        redeem_request.btc_address = origin_btc_address;
        Redeem::<T>::insert_redeem_request(redeem_id, redeem_request);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(vault_btc_address);
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

        let mut height = 0;

        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&origin_btc_address, 50, 3)
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
            .add_output(TransactionOutput::payment(value.into(), &origin_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_coinbase(&origin_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_store_block_header(block_header).unwrap();

    }: _(RawOrigin::Signed(vault_id), redeem_id, tx_id, tx_block_height, proof, raw_tx)

    cancel_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let redeem_id = H256::zero();
        let mut redeem_request = RedeemRequest::default();
        redeem_request.vault = vault_id.clone();
        redeem_request.redeemer = origin.clone();
        redeem_request.opentime = System::<T>::block_number();
        Redeem::<T>::insert_redeem_request(redeem_id, redeem_request);
        System::<T>::set_block_number(System::<T>::block_number() + Redeem::<T>::redeem_period() + 10.into());

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160::from([0; 20])));
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), redeem_id, true)

    set_redeem_period {
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
            assert_ok!(test_benchmark_request_redeem::<Test>());
            assert_ok!(test_benchmark_execute_redeem::<Test>());
            assert_ok!(test_benchmark_cancel_redeem::<Test>());
            assert_ok!(test_benchmark_set_redeem_period::<Test>());
        });
    }
}
