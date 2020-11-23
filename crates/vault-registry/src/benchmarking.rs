use super::*;
use crate::Module as VaultRegistry;
use btc_relay::BtcPayload;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_core::H160;
use sp_std::prelude::*;

benchmarks! {
    _ {}

    register_vault {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = 100;
        let btc_address = BtcPayload::P2SH(H160::zero());
    }: _(RawOrigin::Signed(origin.clone()), amount.into(), btc_address)
    verify {
        assert_eq!(Vaults::<T>::get(origin).wallet.get_btc_address(), btc_address);
    }

    lock_additional_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPayload::P2SH(H160::zero()));
        VaultRegistry::<T>::_insert_vault(&origin, vault);
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    withdraw_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPayload::P2SH(H160::zero()));
        VaultRegistry::<T>::_insert_vault(&origin, vault);
        collateral::Module::<T>::lock_collateral(&origin, u.into()).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    update_btc_address {
        let origin: T::AccountId = account("Origin", 0, 0);
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPayload::P2SH(H160::zero()));
        VaultRegistry::<T>::_insert_vault(&origin, vault);
    }: _(RawOrigin::Signed(origin), BtcPayload::P2SH(H160::from([1; 20])))

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build_with(
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
                balances: vec![(account("Origin", 0, 0), 1 << 64)],
            },
        )
        .execute_with(|| {
            assert_ok!(test_benchmark_register_vault::<Test>());
            assert_ok!(test_benchmark_lock_additional_collateral::<Test>());
            assert_ok!(test_benchmark_withdraw_collateral::<Test>());
            assert_ok!(test_benchmark_update_btc_address::<Test>());
        });
    }
}
