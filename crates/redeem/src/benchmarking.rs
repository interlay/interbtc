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
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, CurrencyId::Token, TokenSymbol::*, VaultCurrencyPair, VaultId};
use sp_core::{H160, H256, U256};
use sp_runtime::traits::One;
use sp_std::prelude::*;
use vault_registry::types::{Vault, Wallet};

// Pallets
use crate::Pallet as Redeem;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

fn collateral<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_collateral_currency_id::<T>())
}

fn wrapped<T: crate::Config>(amount: u32) -> Amount<T> {
    Amount::new(amount.into(), get_wrapped_currency_id::<T>())
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn register_public_key<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        dummy_public_key()
    ));
}

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn mint_wrapped<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    let rich_amount = Amount::<T>::new(amount, get_wrapped_currency_id::<T>());
    assert_ok!(rich_amount.mint_to(account_id));
}

fn initialize_oracle<T: crate::Config>() {
    let oracle_id: T::AccountId = account("Oracle", 12, 0);

    Oracle::<T>::_feed_values(
        oracle_id,
        vec![
            (
                OracleKey::ExchangeRate(Token(DOT)),
                UnsignedFixedPoint::<T>::checked_from_rational(1, 1).unwrap(),
            ),
            (
                OracleKey::FeeEstimation,
                UnsignedFixedPoint::<T>::checked_from_rational(3, 1).unwrap(),
            ),
        ],
    );
    Oracle::<T>::begin_block(0u32.into());
}

fn mine_blocks_until_expiry<T: crate::Config>(request: &DefaultRedeemRequest<T>) {
    let period = Redeem::<T>::redeem_period().max(request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(request.btc_height, period).unwrap();
    mine_blocks::<T>(expiry_height + 100);
}

fn mine_blocks<T: crate::Config>(end_height: u32) {
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
    for _ in 0..end_height {
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

fn test_request<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> DefaultRedeemRequest<T> {
    RedeemRequest {
        vault: vault_id.clone(),
        opentime: Default::default(),
        period: Default::default(),
        fee: Default::default(),
        transfer_fee_btc: Default::default(),
        amount_btc: Default::default(),
        premium: Default::default(),
        redeemer: account("Redeemer", 0, 0),
        btc_address: Default::default(),
        btc_height: Default::default(),
        status: Default::default(),
    }
}

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

benchmarks! {
    request_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id = get_vault_id::<T>();
        let amount = Redeem::<T>::redeem_btc_dust_value() + 1000u32.into();
        let btc_address = BtcAddress::P2SH(H160::from([0; 20]));

        initialize_oracle::<T>();

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            wallet: Wallet::new(),
            issued_tokens: amount,
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };

        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

        mint_wrapped::<T>(&origin, amount);

        assert_ok!(Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));
    }: _(RawOrigin::Signed(origin), amount, btc_address, vault_id.clone())

    liquidation_redeem {
        assert_ok!(Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));

        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id = get_vault_id::<T>();
        let amount = 1000;

        register_public_key::<T>(vault_id.clone());

        VaultRegistry::<T>::insert_vault(
            &vault_id,
            Vault::new(vault_id.clone())
        );

        mint_wrapped::<T>(&origin, amount.into());

        mint_collateral::<T>(&vault_id.account_id, 100_000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &collateral(100_000)));

        assert_ok!(VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &wrapped(amount)));
        assert_ok!(VaultRegistry::<T>::issue_tokens(&vault_id, &wrapped(amount)));

        VaultRegistry::<T>::liquidate_vault(&vault_id).unwrap();
        let currency_pair = VaultCurrencyPair {
            collateral: get_collateral_currency_id::<T>(),
            wrapped: get_wrapped_currency_id::<T>()
        };
    }: _(RawOrigin::Signed(origin), currency_pair, amount.into())

    execute_redeem {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id = get_vault_id::<T>();
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        initialize_oracle::<T>();

        let origin_btc_address = BtcAddress::P2PKH(H160::zero());

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.btc_address = origin_btc_address;
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            wallet: Wallet::new(),
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };

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

        assert_ok!(Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));
    }: _(RawOrigin::Signed(vault_id.account_id.clone()), redeem_id, proof, raw_tx)

    cancel_redeem_reimburse {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id = get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = origin.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        redeem_request.btc_height = BtcRelay::<T>::get_best_block_height();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);

        // expire redeem request
        mine_blocks_until_expiry::<T>(&redeem_request);
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 100u32.into());

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            wallet: Wallet::new(),
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );
        mint_collateral::<T>(&vault_id.account_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &collateral(1000)));

        assert_ok!(Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
        ));
    }: cancel_redeem(RawOrigin::Signed(origin), redeem_id, true)

    cancel_redeem_retry {
        let origin: T::AccountId = account("Origin", 0, 0);
        let vault_id = get_vault_id::<T>();

        initialize_oracle::<T>();

        let redeem_id = H256::zero();
        let mut redeem_request = test_request::<T>(&vault_id);
        redeem_request.redeemer = origin.clone();
        redeem_request.opentime = Security::<T>::active_block_number();
        Redeem::<T>::insert_redeem_request(&redeem_id, &redeem_request);

        // expire redeem request
        mine_blocks_until_expiry::<T>(&redeem_request);
        Security::<T>::set_active_block_number(Security::<T>::active_block_number() + Redeem::<T>::redeem_period() + 100u32.into());

        register_public_key::<T>(vault_id.clone());

        let vault = Vault {
            wallet: Wallet::new(),
            id: vault_id.clone(),
            ..Vault::new(vault_id.clone())
        };
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );
        mint_collateral::<T>(&vault_id.account_id, 1000u32.into());
        assert_ok!(VaultRegistry::<T>::try_deposit_collateral(&vault_id, &collateral(1000)));

        assert_ok!(Oracle::<T>::_set_exchange_rate(get_collateral_currency_id::<T>(),
            UnsignedFixedPoint::<T>::one()
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
