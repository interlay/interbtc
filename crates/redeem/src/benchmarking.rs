use super::*;
use crate::Module as Redeem;
use bitcoin::formatter::Formattable;
use bitcoin::types::{
    BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput,
};
use btc_relay::Module as BtcRelay;
use btc_relay::{BtcAddress, BtcPublicKey};
use frame_benchmarking::{account, benchmarks};
use frame_system::Module as System;
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;
use vault_registry::types::{Vault, Wallet};
use vault_registry::Module as VaultRegistry;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55,
        18, 45, 222, 180, 119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

benchmarks! {
    request_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = Redeem::<T>::redeem_btc_dust_value() + 1000.into();
        let btc_address = BtcAddress::P2SH(H160::from([0; 20]));

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        vault.issued_tokens = amount;
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), amount, btc_address, vault_id.clone())

    execute_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let origin_btc_address = BtcAddress::P2PKH(H160::zero());

        let redeem_id = H256::zero();
        let mut redeem_request = RedeemRequest::default();
        redeem_request.vault = vault_id.clone();
        redeem_request.btc_address = origin_btc_address;
        Redeem::<T>::insert_redeem_request(redeem_id, redeem_request);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&origin_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash = block.header.hash();
        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_initialize(block_header, height).unwrap();

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_previous_hash(block.transactions[0].hash())
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
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_store_block_header(relayer_id, block_header).unwrap();

    }: _(RawOrigin::Signed(vault_id), redeem_id, tx_id, proof, raw_tx)

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
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
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
