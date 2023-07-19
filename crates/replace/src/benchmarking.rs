use super::*;
use bitcoin::types::TransactionOutput;
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::getters::{get_relay_chain_currency_id as get_collateral_currency_id, *};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use primitives::VaultId;
use sp_core::H256;
use sp_runtime::{traits::One, FixedPointNumber};
use sp_std::{fmt::Debug, prelude::*};

// Pallets
use crate::Pallet as Replace;
use btc_relay::Pallet as BtcRelay;
use oracle::Pallet as Oracle;
use security::Pallet as Security;
use vault_registry::Pallet as VaultRegistry;

fn test_request<T: crate::Config>(
    new_vault_id: &DefaultVaultId<T>,
    old_vault_id: &DefaultVaultId<T>,
) -> DefaultReplaceRequest<T> {
    ReplaceRequest {
        new_vault: new_vault_id.clone(),
        old_vault: old_vault_id.clone(),
        period: Default::default(),
        accept_time: Default::default(),
        amount: Default::default(),
        griefing_collateral: 12345u32.into(), // non-zero to hit additional code paths
        btc_address: BtcAddress::dummy(),
        collateral: Default::default(),
        btc_height: Default::default(),
        status: Default::default(),
    }
}

fn get_vault_id<T: crate::Config>(name: &'static str) -> DefaultVaultId<T> {
    VaultId::new(
        account(name, 0, 0),
        get_collateral_currency_id::<T>(),
        get_wrapped_currency_id::<T>(),
    )
}

fn register_vault<T: crate::Config>(vault_id: &DefaultVaultId<T>, issued_tokens: Amount<T>, to_be_replaced: Amount<T>) {
    let origin = RawOrigin::Signed(vault_id.account_id.clone());

    assert_ok!(<orml_tokens::Pallet<T>>::deposit(
        get_collateral_currency_id::<T>(),
        &vault_id.account_id,
        (1u32 << 31).into()
    ));
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(
        get_native_currency_id::<T>(),
        &vault_id.account_id,
        (1u32 << 31).into()
    ));

    assert_ok!(VaultRegistry::<T>::register_public_key(
        origin.into(),
        BtcPublicKey::dummy()
    ));
    assert_ok!(VaultRegistry::<T>::_register_vault(
        vault_id.clone(),
        100000000u32.into()
    ));

    VaultRegistry::<T>::try_increase_to_be_issued_tokens(vault_id, &issued_tokens).unwrap();
    VaultRegistry::<T>::issue_tokens(vault_id, &issued_tokens).unwrap();
    VaultRegistry::<T>::try_increase_to_be_replaced_tokens(vault_id, &to_be_replaced).unwrap();
}

struct ChainState<T: Config> {
    old_vault_id: DefaultVaultId<T>,
    new_vault_id: DefaultVaultId<T>,
    issued_tokens: Amount<T>,
    to_be_replaced: Amount<T>,
}

fn setup_chain<T: crate::Config>() -> ChainState<T> {
    let new_vault_id = get_vault_id::<T>("NewVault");
    let old_vault_id = get_vault_id::<T>("OldVault");

    Oracle::<T>::_set_exchange_rate(
        get_native_currency_id::<T>(), // for griefing collateral
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();
    Oracle::<T>::_set_exchange_rate(
        old_vault_id.collateral_currency(),
        <T as currency::Config>::UnsignedFixedPoint::one(),
    )
    .unwrap();

    VaultRegistry::<T>::set_minimum_collateral(
        RawOrigin::Root.into(),
        old_vault_id.collateral_currency(),
        100_000u32.into(),
    )
    .unwrap();
    VaultRegistry::<T>::_set_system_collateral_ceiling(old_vault_id.currencies.clone(), 1_000_000_000u32.into());

    VaultRegistry::<T>::_set_secure_collateral_threshold(
        old_vault_id.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap(),
    );
    VaultRegistry::<T>::_set_premium_redeem_threshold(
        old_vault_id.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 200000).unwrap(),
    );
    VaultRegistry::<T>::_set_liquidation_collateral_threshold(
        old_vault_id.currencies.clone(),
        <T as currency::Config>::UnsignedFixedPoint::checked_from_rational(1, 300000).unwrap(),
    );

    let issued_tokens = Amount::new(200000u32.into(), old_vault_id.wrapped_currency());
    let to_be_replaced = issued_tokens.clone().map(|x| x / 4u32.into());

    register_vault(&old_vault_id, issued_tokens.clone(), to_be_replaced.clone());
    register_vault(&new_vault_id, issued_tokens.clone(), to_be_replaced.clone());

    ChainState {
        old_vault_id,
        new_vault_id,
        issued_tokens,
        to_be_replaced,
    }
}

fn setup_replace<T: crate::Config>(
    old_vault_id: &DefaultVaultId<T>,
    new_vault_id: &DefaultVaultId<T>,
    to_be_replaced: Amount<T>,
    hashes: u32,
    vin: u32,
    vout: u32,
    tx_size: u32,
) -> (H256, FullTransactionProof)
where
    <<T as currency::Config>::Balance as TryInto<i64>>::Error: Debug,
{
    let replace_id = H256::zero();
    let mut replace_request = test_request::<T>(&new_vault_id, &old_vault_id);
    replace_request.amount = to_be_replaced.amount();
    Replace::<T>::insert_replace_request(&replace_id, &replace_request);

    // simulate that the request has been accepted
    VaultRegistry::<T>::try_increase_to_be_redeemed_tokens(&old_vault_id, &to_be_replaced).unwrap();
    VaultRegistry::<T>::try_increase_to_be_issued_tokens(&new_vault_id, &to_be_replaced).unwrap();

    VaultRegistry::<T>::transfer_funds(
        CurrencySource::FreeBalance(old_vault_id.account_id.clone()),
        CurrencySource::ActiveReplaceCollateral(old_vault_id.clone()),
        &Amount::new(replace_request.griefing_collateral, get_native_currency_id::<T>()),
    )
    .unwrap();

    let relayer_id: T::AccountId = account("Relayer", 0, 0);
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(
        get_collateral_currency_id::<T>(),
        &relayer_id,
        (1u32 << 31).into()
    ));
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(
        get_native_currency_id::<T>(),
        &relayer_id,
        (1u32 << 31).into()
    ));

    // we always need these outputs for replace
    let mut outputs = vec![
        TransactionOutput::payment(
            to_be_replaced.amount().try_into().unwrap(),
            &replace_request.btc_address,
        ),
        TransactionOutput::op_return(0, replace_id.as_bytes()),
    ];

    // add return-to-self output
    if vout == 3 {
        outputs.push(TransactionOutput::payment(
            0u32.into(),
            &BtcAddress::P2PKH(sp_core::H160::zero()),
        ));
    }

    let transaction =
        BtcRelay::<T>::initialize_and_store_max(relayer_id.clone(), hashes, vin, outputs, tx_size as usize);

    let period = Replace::<T>::replace_period().max(replace_request.period);
    let expiry_height = BtcRelay::<T>::bitcoin_expiry_height(replace_request.btc_height, period).unwrap();

    BtcRelay::<T>::mine_blocks(&relayer_id, expiry_height + 100);
    Security::<T>::set_active_block_number(
        Security::<T>::active_block_number() + Replace::<T>::replace_period() + 100u32.into(),
    );

    (replace_id, transaction)
}

#[benchmarks(
	where
		<<T as currency::Config>::Balance as TryInto<i64>>::Error: Debug,
)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn request_replace() {
        let ChainState {
            old_vault_id,
            issued_tokens,
            to_be_replaced,
            ..
        } = setup_chain::<T>();

        let amount = (issued_tokens.checked_sub(&to_be_replaced).unwrap()).amount();

        #[extrinsic_call]
        request_replace(
            RawOrigin::Signed(old_vault_id.account_id.clone()),
            old_vault_id.currencies.clone(),
            amount,
        );
    }

    #[benchmark]
    fn withdraw_replace() {
        let ChainState {
            old_vault_id,
            to_be_replaced,
            ..
        } = setup_chain::<T>();

        #[extrinsic_call]
        withdraw_replace(
            RawOrigin::Signed(old_vault_id.account_id.clone()),
            old_vault_id.currencies.clone(),
            to_be_replaced.amount(),
        );
    }

    #[benchmark]
    fn accept_replace() {
        let ChainState {
            old_vault_id,
            new_vault_id,
            to_be_replaced,
            ..
        } = setup_chain::<T>();

        let replace_id = H256::zero();
        let mut replace_request = test_request::<T>(&new_vault_id, &old_vault_id);
        replace_request.amount = to_be_replaced.amount();
        Replace::<T>::insert_replace_request(&replace_id, &replace_request);

        let new_vault_btc_address = BtcAddress::dummy();
        let griefing = 100000000u32.into();

        #[extrinsic_call]
        accept_replace(
            RawOrigin::Signed(new_vault_id.account_id.clone()),
            new_vault_id.currencies.clone(),
            old_vault_id,
            to_be_replaced.amount(),
            griefing,
            new_vault_btc_address,
        );
    }

    #[benchmark]
    fn execute_pending_replace(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<2, 3>, b: Linear<541, 2_048>) {
        let ChainState {
            old_vault_id,
            new_vault_id,
            to_be_replaced,
            ..
        } = setup_chain::<T>();
        let (replace_id, transaction) = setup_replace::<T>(&old_vault_id, &new_vault_id, to_be_replaced, h, i, o, b);

        #[extrinsic_call]
        execute_replace(RawOrigin::Signed(old_vault_id.account_id), replace_id, transaction);
    }

    #[benchmark]
    fn execute_cancelled_replace(h: Linear<2, 10>, i: Linear<1, 10>, o: Linear<2, 3>, b: Linear<541, 2_048>) {
        let ChainState {
            old_vault_id,
            new_vault_id,
            to_be_replaced,
            ..
        } = setup_chain::<T>();
        let (replace_id, transaction) = setup_replace::<T>(&old_vault_id, &new_vault_id, to_be_replaced, h, i, o, b);

        assert_ok!(Pallet::<T>::cancel_replace(
            RawOrigin::Signed(new_vault_id.account_id).into(),
            replace_id
        ));

        #[extrinsic_call]
        execute_replace(RawOrigin::Signed(old_vault_id.account_id), replace_id, transaction);
    }

    #[benchmark]
    fn cancel_replace() {
        let ChainState {
            old_vault_id,
            new_vault_id,
            to_be_replaced,
            ..
        } = setup_chain::<T>();

        let (replace_id, _) = setup_replace::<T>(&old_vault_id, &new_vault_id, to_be_replaced, 2, 2, 2, 541);

        #[extrinsic_call]
        cancel_replace(RawOrigin::Signed(new_vault_id.account_id), replace_id);
    }

    #[benchmark]
    fn set_replace_period() {
        #[extrinsic_call]
        set_replace_period(RawOrigin::Root, 1u32.into());
    }

    impl_benchmark_test_suite! {
        Replace,
        crate::mock::ExtBuilder::build_with(Default::default()),
        crate::mock::Test
    }
}
