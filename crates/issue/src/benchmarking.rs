use super::*;
use crate::Module as Issue;
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

    request_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = 100;
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let griefing = 100;

        Collateral::<T>::lock_collateral(&vault_id, 100000000.into()).unwrap();
        ExchangeRateOracle::<T>::_set_exchange_rate(1).unwrap();
        VaultRegistry::<T>::set_secure_collateral_threshold(1);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160::zero()));
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), amount.into(), vault_id, griefing.into())

    execute_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let vault_btc_address = BtcAddress::P2SH(H160::zero());
        let value = 2;

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        issue_request.btc_address = vault_btc_address;
        issue_request.amount = value.into();
        Issue::<T>::insert_issue_request(issue_id, issue_request);

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash = block.header.hash();
        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_initialize(block_header, height).unwrap();

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
            .add_output(TransactionOutput::payment(value.into(), &vault_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_coinbase(&vault_btc_address, 50, 4)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_store_block_header(relayer_id, block_header).unwrap();

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(vault_btc_address);
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        VaultRegistry::<T>::set_secure_collateral_threshold(1);
        ExchangeRateOracle::<T>::_set_exchange_rate(1).unwrap();
        Collateral::<T>::lock_collateral(&vault_id, 100000000.into()).unwrap();
        VaultRegistry::<T>::increase_to_be_issued_tokens(&vault_id, value.into()).unwrap();
    }: _(RawOrigin::Signed(origin), issue_id, tx_id, proof, raw_tx)

    cancel_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        Issue::<T>::insert_issue_request(issue_id, issue_request);
        System::<T>::set_block_number(System::<T>::block_number() + Issue::<T>::issue_period() + 10.into());

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(BtcAddress::P2SH(H160::zero()));
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), issue_id)

    set_issue_period {
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
            assert_ok!(test_benchmark_request_issue::<Test>());
            assert_ok!(test_benchmark_execute_issue::<Test>());
            assert_ok!(test_benchmark_cancel_issue::<Test>());
            assert_ok!(test_benchmark_set_issue_period::<Test>());
        });
    }
}
