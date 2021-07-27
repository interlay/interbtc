use super::*;
use crate::{mock::CurrencyId, Pallet as Redeem};
use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::{
        BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
        TransactionOutput,
    },
};
use btc_relay::{BtcAddress, BtcPublicKey, Pallet as BtcRelay};
use currency::ParachainCurrency;
use exchange_rate_oracle::Pallet as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use security::Pallet as Security;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;
use vault_registry::{
    types::{Vault, Wallet},
    Pallet as VaultRegistry,
};

type Treasury<T> = <T as vault_registry::Config>::Wrapped;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: Collateral<T>) {
    assert_ok!(<T as vault_registry::Config>::Collateral::mint(account_id, amount));
}

fn initialize_oracle<T: crate::Config>() {
    let oracle_id: T::AccountId = account("Oracle", 12, 0);

    ExchangeRateOracle::<T>::_feed_values(
        oracle_id,
        vec![
            (
                OracleKey::ExchangeRate(CurrencyId::DOT),
                <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::checked_from_rational(1, 1).unwrap(),
            ),
            (
                OracleKey::FeeEstimation(BitcoinInclusionTime::Fast),
                <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::checked_from_rational(3, 1).unwrap(),
            ),
            (
                OracleKey::FeeEstimation(BitcoinInclusionTime::Half),
                <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::checked_from_rational(2, 1).unwrap(),
            ),
            (
                OracleKey::FeeEstimation(BitcoinInclusionTime::Hour),
                <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::checked_from_rational(1, 1).unwrap(),
            ),
        ],
    )
    .unwrap();
    ExchangeRateOracle::<T>::begin_block(0u32.into());
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
    request_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = Redeem::<T>::redeem_btc_dust_value() + 1000u32.into();
        let btc_address = BtcAddress::P2SH(H160::from([0; 20]));

        initialize_oracle::<T>();

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        vault.issued_tokens = amount;
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        assert_ok!(Treasury::<T>::mint(&origin, amount));

        assert_ok!(ExchangeRateOracle::<T>::_set_exchange_rate(
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ));
    }: _(RawOrigin::Signed(origin), amount, btc_address, vault_id.clone())

    execute_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        initialize_oracle::<T>();

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
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations() + 1u32.into());

        assert_ok!(ExchangeRateOracle::<T>::_set_exchange_rate(
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ));
    }: _(RawOrigin::Signed(vault_id), redeem_id, proof, raw_tx)

    cancel_redeem_reimburse {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = RedeemRequest::default();
        redeem_request.vault = vault_id.clone();
        redeem_request.redeemer = origin.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        Redeem::<T>::insert_redeem_request(redeem_id, redeem_request);
        mine_blocks::<T>();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 10u32.into());
        assert_ok!(ExchangeRateOracle::<T>::_set_exchange_rate(
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ));

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );
        mint_collateral::<T>(&vault_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, 1000u32.into()));
    }: cancel_redeem(RawOrigin::Signed(origin), redeem_id, true)

    cancel_redeem_retry {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id: T::AccountId = account("Vault", 0, 0);

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = RedeemRequest::default();
        redeem_request.vault = vault_id.clone();
        redeem_request.redeemer = origin.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        Redeem::<T>::insert_redeem_request(redeem_id, redeem_request);
        mine_blocks::<T>();
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 10u32.into());

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );
        mint_collateral::<T>(&vault_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, 1000u32.into()));

        assert_ok!(ExchangeRateOracle::<T>::_set_exchange_rate(
            <T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()
        ));
    }: cancel_redeem(RawOrigin::Signed(origin), redeem_id, false)

    set_redeem_period {
    }: _(RawOrigin::Root, 1u32.into())

}

impl_benchmark_test_suite!(
    Redeem,
    crate::mock::ExtBuilder::build_with(Default::default()),
    crate::mock::Test
);
