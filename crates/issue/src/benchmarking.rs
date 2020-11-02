use super::*;
use crate::Module as Issue;
use bitcoin::formatter::Formattable;
use bitcoin::types::{
    Address, BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder,
    TransactionOutput,
};
use btc_relay::Module as BtcRelay;
use collateral::Module as Collateral;
use exchange_rate_oracle::Module as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_core::{H256, U256};
use sp_std::prelude::*;
use vault_registry::types::Vault;
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
        VaultRegistry::<T>::_set_secure_collateral_threshold(1);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), amount.into(), vault_id, griefing.into())

    execute_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        Issue::<T>::insert_issue_request(issue_id, issue_request);

        let address = Address::from([0; 20]);
        let mut height = 0;

        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash = block.header.hash();
        println!("{:?}", block_hash);
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
            .add_output(TransactionOutput::p2pkh(value.into(), &address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 4)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        println!("{:?}", block.header.hash());

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
        BtcRelay::<T>::_store_block_header(block_header).unwrap();

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), issue_id, tx_id, tx_block_height, proof, raw_tx)

    cancel_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        Issue::<T>::insert_issue_request(issue_id, issue_request);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        VaultRegistry::<T>::_insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), issue_id)

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

            // issue period is 10, we issued at block 1, so at block 15 the cancel should succeed
            <frame_system::Module<Test>>::set_block_number(15);
            assert_ok!(test_benchmark_cancel_issue::<Test>());
        });
    }
}
