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
use primitives::VaultId;
use sp_core::{H160, H256, U256};
use sp_runtime::traits::One;
use sp_std::prelude::*;
use vault_registry::types::Vault;

// Pallets
use crate::Pallet as Refund;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

benchmarks! {
    execute_refund {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: VaultId<T::AccountId, _> = VaultId::new(
            account("Vault", 0, 0),
            get_collateral_currency_id::<T>(),
            get_wrapped_currency_id::<T>(),
        );
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        let origin_btc_address = BtcAddress::P2PKH(H160::zero());

        let refund_id = H256::zero();
        let refund_request = RefundRequest {
            vault: vault_id.clone(),
            btc_address: origin_btc_address,
            completed: Default::default(),
            amount_btc: Default::default(),
            fee: Default::default(),
            issue_id: Default::default(),
            issuer: account("Issuer", 0, 0),
            transfer_fee_btc: Default::default(),
    };
        Refund::<T>::insert_refund_request(&refund_id, &refund_request);

        let origin = RawOrigin::Signed(vault_id.account_id.clone());
        assert_ok!(VaultRegistry::<T>::register_public_key(origin.into(), dummy_public_key()));

        let vault = Vault::new(vault_id.clone());

        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&origin_btc_address, 50, 3)
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
            .add_output(TransactionOutput::payment(value.into(), &origin_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(4)
            .with_coinbase(&origin_btc_address, 50, 3)
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

        assert_ok!(Oracle::<T>::_set_exchange_rate(
            get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));
    }: _(RawOrigin::Signed(vault_id.account_id.clone()), refund_id, proof, raw_tx)

    set_refund_transaction_size {
    }: _(RawOrigin::Root, 1u32.into())
}

impl_benchmark_test_suite!(Refund, crate::mock::ExtBuilder::build(), crate::mock::Test);
