use super::*;
use crate::Pallet as Issue;
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput},
};
use btc_relay::{BtcAddress, BtcPublicKey, Pallet as BtcRelay};
use exchange_rate_oracle::Pallet as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use security::Pallet as Security;
use sp_core::{H160, H256, U256};
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;
use vault_registry::{
    types::{Vault, Wallet},
    Pallet as VaultRegistry,
};

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn make_free_balance_be<T: currency::Config<currency::Collateral>>(account_id: &T::AccountId, amount: Collateral<T>) {
    <<T as currency::Config<currency::Collateral>>::Currency>::make_free_balance_be(account_id, amount);
}

benchmarks! {
    request_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount: u32 = 100;
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let griefing: u32 = 100;
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        make_free_balance_be::<T>(&origin, (1u32 << 31).into());
        make_free_balance_be::<T>(&vault_id, (1u32 << 31).into());
        make_free_balance_be::<T>(&relayer_id, (1u32 << 31).into());

        ExchangeRateOracle::<T>::_set_exchange_rate(<T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();
        VaultRegistry::<T>::set_secure_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());// 0.001%
        VaultRegistry::<T>::_register_vault(&vault_id, 100000000u32.into(), dummy_public_key()).unwrap();

        // initialize relay

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&BtcAddress::P2SH(H160::zero()), 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();
        let block_hash = block.header.hash;
        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        Security::<T>::set_active_block_number(1u32.into());
        BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

        let vault_btc_address = BtcAddress::P2SH(H160::zero());

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
        .add_output(TransactionOutput::payment(123123, &vault_btc_address))
        .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
        .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_timestamp(1588813835)
            .add_transaction(transaction)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();
        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations());

    }: _(RawOrigin::Signed(origin), amount.into(), vault_id, griefing.into())

    execute_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        make_free_balance_be::<T>(&origin, (1u32 << 31).into());
        make_free_balance_be::<T>(&vault_id, (1u32 << 31).into());
        make_free_balance_be::<T>(&relayer_id, (1u32 << 31).into());

        let vault_btc_address = BtcAddress::P2SH(H160::zero());
        let value: u32 = 2;

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        issue_request.btc_address = vault_btc_address;
        issue_request.amount = value.into();
        Issue::<T>::insert_issue_request(&issue_id, &issue_request);

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let block_hash = block.header.hash;
        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        Security::<T>::set_active_block_number(1u32.into());
        BtcRelay::<T>::initialize(relayer_id.clone(), block_header, height).unwrap();

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
            .mine(U256::from(2).pow(254.into())).unwrap();

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&[tx_id]).unwrap().try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        let block_header = BtcRelay::<T>::parse_raw_block_header(&raw_block_header).unwrap();

        BtcRelay::<T>::store_block_header(&relayer_id, block_header).unwrap();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations());

        VaultRegistry::<T>::set_secure_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());
        ExchangeRateOracle::<T>::_set_exchange_rate(<T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();
        VaultRegistry::<T>::_register_vault(&vault_id, 100000000u32.into(), dummy_public_key()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, value.into()).unwrap();
        let secure_id = Security::<T>::get_secure_id(&vault_id);
        VaultRegistry::<T>::register_deposit_address(&vault_id, secure_id).unwrap();
    }: _(RawOrigin::Signed(origin), issue_id, proof, raw_tx)

    cancel_issue {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        make_free_balance_be::<T>(&origin, (1u32 << 31).into());
        make_free_balance_be::<T>(&vault_id, (1u32 << 31).into());

        let issue_id = H256::zero();
        let mut issue_request = IssueRequest::default();
        issue_request.requester = origin.clone();
        issue_request.vault = vault_id.clone();
        Issue::<T>::insert_issue_request(&issue_id, &issue_request);
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Issue::<T>::issue_period() + 10u32.into());

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(origin), issue_id)

    set_issue_period {
    }: _(RawOrigin::Root, 1u32.into())

}

impl_benchmark_test_suite!(
    Issue,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
