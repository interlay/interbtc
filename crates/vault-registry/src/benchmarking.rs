use super::*;
use crate::{types::BtcPublicKey, Module as VaultRegistry};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_std::prelude::*;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

benchmarks! {
    register_vault {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount: u32 = 100;
        let public_key = BtcPublicKey::default();
    }: _(RawOrigin::Signed(origin.clone()), amount.into(), public_key)
    verify {
        // assert_eq!(Vaults::<T>::get(origin).wallet.get_btc_address(), btc_address);
    }

    lock_additional_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    withdraw_collateral {
        let origin: T::AccountId = account("Origin", 0, 0);
        let u in 0 .. 100;
        VaultRegistry::<T>::_register_vault(&origin, u.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
    }

    update_public_key {
        let origin: T::AccountId = account("Origin", 0, 0);
        VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key()).unwrap();
    }: _(RawOrigin::Signed(origin), BtcPublicKey::default())

    liquidate_undercollateralized_vaults {
        let u in 0 .. 100;

        for i in 0..u {
            let origin: T::AccountId = account("Origin", i, 0);
            VaultRegistry::<T>::_register_vault(&origin, 1234u32.into(), dummy_public_key()).unwrap();
        }
    }: {
        VaultRegistry::<T>::liquidate_undercollateralized_vaults(LiquidationTarget::NonOperatorsOnly).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build_with(pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
            balances: (0..100).map(|i| (account("Origin", i, 0), 1 << 64)).collect(),
        })
        .execute_with(|| {
            assert_ok!(test_benchmark_register_vault::<Test>());
            assert_ok!(test_benchmark_lock_additional_collateral::<Test>());
            assert_ok!(test_benchmark_withdraw_collateral::<Test>());
            assert_ok!(test_benchmark_update_public_key::<Test>());
            assert_ok!(test_benchmark_liquidate_undercollateralized_vaults::<Test>());
        });
    }
}
