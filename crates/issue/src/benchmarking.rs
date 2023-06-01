use super::*;
use bitcoin::types::{BlockBuilder, TransactionOutput};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::{CurrencyId, VaultId};
use sp_core::{H256, U256};
use sp_runtime::{traits::One, FixedPointNumber};
use sp_std::prelude::*;

// Pallets
use crate::Pallet as Issue;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

fn deposit_tokens<T: crate::Config>(currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(currency_id, account_id, amount));
}

fn mint_collateral<T: crate::Config>(account_id: &T::AccountId, amount: BalanceOf<T>) {
    deposit_tokens::<T>(get_collateral_currency_id::<T>(), account_id, amount);
    deposit_tokens::<T>(get_native_currency_id::<T>(), account_id, amount);
}

fn get_vault_id<T: crate::Config>() -> DefaultVaultId<T> {
    VaultId::new(
        account("Vault", 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

fn setup_chain<T: crate::Config>() {
    let dummy_vault = get_vault_id::<T>();

    Oracle::<T>::_set_exchange_rate(
        get_native_currency_id::<T>(), // for griefing collateral
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();
    Oracle::<T>::_set_exchange_rate(
        dummy_vault.collateral_currency(),
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();

    VaultRegistry::<T>::set_minimum_collateral(
        RawOrigin::Root.into(),
        dummy_vault.collateral_currency(),
        100_000u32.into(),
    )
    .unwrap();
    VaultRegistry::<T>::_set_system_collateral_ceiling(dummy_vault.currencies.clone(), 1_000_000_000u32.into());

    VaultRegistry::<T>::_set_secure_collateral_threshold(
        dummy_vault.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap(),
    );
    VaultRegistry::<T>::_set_premium_redeem_threshold(
        dummy_vault.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 200000).unwrap(),
    );
    VaultRegistry::<T>::_set_liquidation_collateral_threshold(
        dummy_vault.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 300000).unwrap(),
    );
}

fn register_vault<T: crate::Config>(vault_id: DefaultVaultId<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());
    mint_collateral::<T>(&vault_id.account_id.clone(), (1u32 << 31).into());

    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(
        vault_id.clone(),
        100000000u32.into()
    ));
}

fn expire_issue<T: crate::Config>(chain_state: &ChainState<T>) {
    let period = Issue::<T>::issue_period().max(chain_state.issue_request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(chain_state.issue_request.btc_height, period).unwrap();
    Security::<T>::set_active_block_number(
        chain_state.issue_request.opentime + Issue::<T>::issue_period() + 100u32.into(),
    );

    let relayer_id: T::AccountId = account("Relayer", 0, 0);
    BtcRelay::<T>::mine_blocks(&relayer_id, expiry_height + 100);
}

enum PaymentType {
    Underpayment,
    Exact,
    Overpayment,
}

struct ChainState<T: Config> {
    issue_id: H256,
    merkle_proof: MerkleProof,
    transaction: Transaction,
    issue_request: DefaultIssueRequest<T>,
    length_bound: u32,
}

fn setup_issue<T: crate::Config>(
    payment: PaymentType,
    hashes: u32,
    vin: u32,
    vout: u32,
    tx_size: u32,
) -> ChainState<T> {
    let origin: T::AccountId = account("Origin", 0, 0);
    let vault_id = get_vault_id::<T>();
    let relayer_id: T::AccountId = account("Relayer", 0, 0);

    mint_collateral::<T>(&origin, (1u32 << 31).into());
    mint_collateral::<T>(&relayer_id, (1u32 << 31).into());
    setup_chain::<T>();

    let vault_btc_address = BtcAddress::dummy();
    let value: Amount<T> = Amount::new(2u32.into(), get_wrapped_currency_id::<T>());

    let issue_id = H256::zero();
    let issue_request = IssueRequest {
        requester: origin.clone(),
        vault: vault_id.clone(),
        btc_address: vault_btc_address,
        amount: value.amount(),
        btc_height: Default::default(),
        btc_public_key: Default::default(),
        fee: Default::default(),
        griefing_collateral: Default::default(),
        griefing_currency: get_native_currency_id::<T>(),
        opentime: Default::default(),
        period: Default::default(),
        status: Default::default(),
    };
    Issue::<T>::insert_issue_request(&issue_id, &issue_request);

    let mut outputs: Vec<_> = (0..(vout - 1))
        .map(|_| TransactionOutput::payment(0, &BtcAddress::default()))
        .collect();
    // worst-case is expected payment last
    outputs.push(TransactionOutput::payment(
        match payment {
            PaymentType::Underpayment => 1u32.into(),
            PaymentType::Exact => 2u32.into(),
            PaymentType::Overpayment => 3u32.into(),
        },
        &vault_btc_address,
    ));

    let (transaction, merkle_proof) =
        BtcRelay::<T>::initialize_and_store_max(relayer_id.clone(), hashes, vin, outputs, tx_size as usize);
    let length_bound = transaction.size_no_witness() as u32;

    register_vault::<T>(vault_id.clone());

    VaultRegistry::<T>::try_increase_to_be_issued_tokens(&vault_id, &value).unwrap();
    let secure_id = Security::<T>::get_secure_id(&vault_id.account_id);
    VaultRegistry::<T>::register_deposit_address(&vault_id, secure_id).unwrap();

    ChainState {
        issue_id,
        merkle_proof,
        transaction,
        issue_request,
        length_bound,
    }
}

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn request_issue() {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = Issue::<T>::issue_btc_dust_value(get_wrapped_currency_id::<T>()).amount() + 1000u32.into();
        let vault_id = get_vault_id::<T>();
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        mint_collateral::<T>(&origin, (1u32 << 31).into());
        mint_collateral::<T>(&relayer_id, (1u32 << 31).into());

        setup_chain::<T>();
        register_vault::<T>(vault_id.clone());

        // initialize relay
        let init_block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&BtcAddress::dummy(), 50, 3)
            .with_timestamp(u32::MAX)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        Security::<T>::set_active_block_number(1u32.into());
        BtcRelay::<T>::_initialize(relayer_id.clone(), init_block.header, 0).unwrap();
        BtcRelay::<T>::mine_blocks(&relayer_id, 1);
        Security::<T>::set_active_block_number(
            Security::<T>::active_block_number() + BtcRelay::<T>::parachain_confirmations(),
        );

        #[extrinsic_call]
        request_issue(
            RawOrigin::Signed(origin),
            amount,
            vault_id,
            get_native_currency_id::<T>(),
        );
    }

    #[benchmark]
    fn execute_issue_exact(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Exact, h, i, o, b);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn execute_issue_overpayment(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Overpayment, h, i, o, b);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn execute_issue_underpayment(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Underpayment, h, i, o, b);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn execute_expired_issue_exact(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Exact, h, i, o, b);
        expire_issue::<T>(&issue_data);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn execute_expired_issue_overpayment(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Overpayment, h, i, o, b);
        expire_issue::<T>(&issue_data);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn execute_expired_issue_underpayment(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<1, 10>, b: Linear<770, 2_048>) {
        let origin: T::AccountId = account("Origin", 0, 0);
        let issue_data = setup_issue::<T>(PaymentType::Underpayment, h, i, o, b);
        expire_issue::<T>(&issue_data);

        #[extrinsic_call]
        execute_issue(
            RawOrigin::Signed(origin),
            issue_data.issue_id,
            issue_data.merkle_proof,
            issue_data.transaction,
            issue_data.length_bound,
        );
    }

    #[benchmark]
    fn cancel_issue() {
        let origin: T::AccountId = account("Origin", 0, 0);

        let issue_data = setup_issue::<T>(PaymentType::Exact, 2, 2, 2, 770);
        expire_issue::<T>(&issue_data);

        #[extrinsic_call]
        cancel_issue(RawOrigin::Signed(origin), issue_data.issue_id);
    }

    #[benchmark]
    fn set_issue_period() {
        #[extrinsic_call]
        set_issue_period(RawOrigin::Root, 1u32.into());
    }

    impl_benchmark_test_suite! {
        Issue,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    }
}
