use super::*;
use crate::types::BtcPublicKey;
use crate::Module as VaultRegistry;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks! {
    register_vault {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = 100;
        let public_key = BtcPublicKey::default();
    }: _(RawOrigin::Signed(origin.clone()), amount.into(), public_key)
    verify {
        // assert_eq!(Vaults::<T>::get(origin).wallet.get_btc_address(), btc_address);
    }

    lock_additional_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPublicKey::default());
        VaultRegistry::<T>::insert_vault(&origin, vault);
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    withdraw_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPublicKey::default());
        VaultRegistry::<T>::insert_vault(&origin, vault);
        collateral::Module::<T>::lock_collateral(&origin, u.into()).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    update_public_key {
        let origin: T::AccountId = account("Origin", 0, 0);
        let mut vault = Vault::default();
        vault.id = origin.clone();
        vault.wallet = Wallet::new(BtcPublicKey::default());
        VaultRegistry::<T>::insert_vault(&origin, vault);
    }: _(RawOrigin::Signed(origin), BtcPublicKey::default())

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
            assert_ok!(test_benchmark_update_public_key::<Test>());
        });
    }
}
