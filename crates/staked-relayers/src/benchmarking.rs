use super::*;
use crate::Module as StakedRelayers;
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{BlockBuilder, H256Le, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput},
};
use btc_relay::{BtcAddress, BtcPublicKey, Module as BtcRelay};
use collateral::Module as Collateral;
use frame_benchmarking::{account, benchmarks};
use frame_system::{Module as System, RawOrigin};
use sp_core::{H160, U256};
use sp_std::prelude::*;
use vault_registry::{
    types::{Vault, Wallet},
    Module as VaultRegistry,
};

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

benchmarks! {

    initialize {
        let height = 0u32;
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake = 100u32;

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();
        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        <ActiveStakedRelayers<T>>::insert(&origin, StakedRelayer { stake: stake.into(), height: System::<T>::block_number() });
    }: _(RawOrigin::Signed(origin), block_header, height.into())

    store_block_header {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let height = 0;
        let stake = 100u32;

        let init_block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let init_block_hash = init_block.header.hash().unwrap();
        let raw_block_header = RawBlockHeader::from_bytes(&init_block.header.try_format().unwrap())
            .expect("could not serialize block header");

            <ActiveStakedRelayers<T>>::insert(&origin, StakedRelayer { stake: stake.into(), height: System::<T>::block_number() });

        BtcRelay::<T>::initialize(origin.clone(), raw_block_header, height).unwrap();

        let block = BlockBuilder::new()
            .with_previous_hash(init_block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap())
            .expect("could not serialize block header");

    }: _(RawOrigin::Signed(origin), raw_block_header)

    register_staked_relayer {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 100 .. 1000;
    }: _(RawOrigin::Signed(origin.clone()), u.into())
    verify {
        assert_eq!(<InactiveStakedRelayers<T>>::get(origin).stake, u.into());
    }

    deregister_staked_relayer {
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake: u32 = 100;
        <ActiveStakedRelayers<T>>::insert(&origin, StakedRelayer { stake: stake.into(), height: System::<T>::block_number() });
        Collateral::<T>::lock_collateral(&origin, stake.into()).unwrap();
    }: _(RawOrigin::Signed(origin))

    suggest_status_update {
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake: u32 = 100;
        let deposit: u32 = 1000;
        let status_code = StatusCode::Error;
        StakedRelayers::<T>::insert_active_staked_relayer(&origin, stake.into(), System::<T>::block_number());
    }: _(RawOrigin::Signed(origin), deposit.into(), status_code, None, None, None, vec![])

    vote_on_status_update {
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake: u32 = 100;
        StakedRelayers::<T>::insert_active_staked_relayer(&origin, stake.into(), System::<T>::block_number());
        let status_update = StatusUpdate::default();
        let status_update_id = StakedRelayers::<T>::insert_active_status_update(status_update);
    }: _(RawOrigin::Signed(origin), status_update_id, true)

    force_status_update {
        let origin: T::AccountId = account("Origin", 0, 0);
        let status_code = StatusCode::Error;
    }: _(RawOrigin::Signed(origin), status_code, None, None)

    slash_staked_relayer {
        let origin: T::AccountId = account("Origin", 0, 0);
        let staked_relayer: T::AccountId = account("Vault", 0, 0);
        let stake: u32 = 100;
        StakedRelayers::<T>::insert_active_staked_relayer(&staked_relayer, stake.into(), System::<T>::block_number());
        Collateral::<T>::lock_collateral(&staked_relayer, stake.into()).unwrap();

    }: _(RawOrigin::Signed(origin), staked_relayer)

    report_vault_theft {
        let origin: T::AccountId = account("Origin", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let stake: u32 = 100;
        StakedRelayers::<T>::insert_active_staked_relayer(&origin, stake.into(), System::<T>::block_number());

        let vault_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));

        let address = BtcAddress::P2PKH(H160([0; 20]));

        let vault_id: T::AccountId = account("Vault", 0, 0);
        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        vault.wallet.add_btc_address(vault_address);
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let block_hash = block.header.hash().unwrap();
        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_sequence(4294967295)
                    .with_previous_index(1)
                    .with_previous_hash(H256Le::from_bytes_le(&[
                        193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76,
                        226, 9, 127, 8, 96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                    ]))
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
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into())).unwrap();

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&vec![tx_id]).unwrap().try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();

    }: _(RawOrigin::Signed(origin), vault_id, tx_id, proof, raw_tx)

    remove_active_status_update {
        let status_update = StatusUpdate::default();
        let status_update_id = StakedRelayers::<T>::insert_active_status_update(status_update);
    }: _(RawOrigin::Root, status_update_id)

    remove_inactive_status_update {
        let status_update_id = 0;
        let status_update = StatusUpdate::default();
        StakedRelayers::<T>::insert_inactive_status_update(status_update_id, &status_update);
    }: _(RawOrigin::Root, status_update_id)

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;
    use sp_arithmetic::FixedI128;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build_with(|storage| {
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
                balances: vec![(account("Origin", 0, 0), 1 << 32), (account("Vault", 0, 0), 1 << 32)],
            }
            .assimilate_storage(storage)
            .unwrap();

            sla::GenesisConfig::<Test> {
                vault_target_sla: FixedI128::from(100),
                vault_redeem_failure_sla_change: FixedI128::from(0),
                vault_executed_issue_max_sla_change: FixedI128::from(0),
                vault_submitted_issue_proof: FixedI128::from(0),
                vault_refunded: FixedI128::from(1),
                relayer_target_sla: FixedI128::from(100),
                relayer_block_submission: FixedI128::from(1),
                relayer_duplicate_block_submission: FixedI128::from(1),
                relayer_correct_no_data_vote_or_report: FixedI128::from(1),
                relayer_correct_invalid_vote_or_report: FixedI128::from(10),
                relayer_correct_theft_report: FixedI128::from(1),
                relayer_false_no_data_vote_or_report: FixedI128::from(-10),
                relayer_false_invalid_vote_or_report: FixedI128::from(-100),
                relayer_ignored_vote: FixedI128::from(-10),
            }
            .assimilate_storage(storage)
            .unwrap();

            GenesisConfig::<Test> {
                gov_id: account("Origin", 0, 0),
                maturity_period: 10,
            }
            .assimilate_storage(storage)
            .unwrap();
        })
        .execute_with(|| {
            assert_ok!(test_benchmark_initialize::<Test>());
            assert_ok!(test_benchmark_store_block_header::<Test>());
            assert_ok!(test_benchmark_register_staked_relayer::<Test>());
            assert_ok!(test_benchmark_deregister_staked_relayer::<Test>());
            assert_ok!(test_benchmark_suggest_status_update::<Test>());
            assert_ok!(test_benchmark_vote_on_status_update::<Test>());
            assert_ok!(test_benchmark_force_status_update::<Test>());
            assert_ok!(test_benchmark_slash_staked_relayer::<Test>());
            assert_ok!(test_benchmark_report_vault_theft::<Test>());
            assert_ok!(test_benchmark_remove_active_status_update::<Test>());
            assert_ok!(test_benchmark_remove_inactive_status_update::<Test>());
        });
    }
}
