use super::*;
use frame_support::{pallet_prelude::StorageVersion, traits::OnRuntimeUpgrade};

/// The log target.
const TARGET: &'static str = "runtime::reward::migration";

pub mod v0 {
    use super::*;
    use frame_support::pallet_prelude::*;

    #[frame_support::storage_alias]
    pub(crate) type TotalStake<T: Config<I>, I: 'static> =
        StorageValue<Pallet<T, I>, SignedFixedPoint<T, I>, ValueQuery>;

    #[frame_support::storage_alias]
    pub(crate) type RewardPerToken<T: Config<I>, I: 'static> =
        StorageMap<Pallet<T, I>, Blake2_128Concat, <T as Config<I>>::CurrencyId, SignedFixedPoint<T, I>, ValueQuery>;

    #[frame_support::storage_alias]
    pub(crate) type Stake<T: Config<I>, I: 'static> =
        StorageMap<Pallet<T, I>, Blake2_128Concat, <T as Config<I>>::StakeId, SignedFixedPoint<T, I>, ValueQuery>;

    #[frame_support::storage_alias]
    pub(crate) type RewardTally<T: Config<I>, I: 'static> = StorageDoubleMap<
        Pallet<T, I>,
        Blake2_128Concat,
        <T as Config<I>>::CurrencyId,
        Blake2_128Concat,
        <T as Config<I>>::StakeId,
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;
}

pub mod v1 {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// Migrate the reward pallet from V0 to V1.
    pub struct MigrateToV1<T, I = ()>(sp_std::marker::PhantomData<(T, I)>);

    // we only implement this migration for the "global" pool
    // i.e. this is only intended to migrate `EscrowRewards`
    impl<T: Config<I, PoolId = ()>, I: 'static> OnRuntimeUpgrade for MigrateToV1<T, I> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            assert_eq!(
                StorageVersion::get::<Pallet<T, I>>(),
                0,
                "Can only upgrade from version 0"
            );

            Ok(Vec::new())
        }

        fn on_runtime_upgrade() -> Weight {
            todo!()
            //             let version = StorageVersion::get::<Pallet<T, I>>();
            //             if version != 0 {
            //                 log::warn!(
            //                     target: TARGET,
            //                     "skipping v0 to v1 migration: executed on wrong storage version.\
            // 				Expected version 0, found {:?}",
            //                     version,
            //                 );
            //                 return T::DbWeight::get().reads(1);
            //             }
            //
            //             let mut weight = T::DbWeight::get().reads_writes(2, 1);
            //
            //             // update total stake
            //             TotalStake::<T, I>::insert((), v0::TotalStake::<T, I>::get());
            //             weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
            //
            //             let reward_per_token_storage_map_v0 = v0::RewardPerToken::<T,
            // I>::drain().collect::<Vec<_>>();             
            // weight.saturating_accrue(T::DbWeight::get().reads(reward_per_token_storage_map_v0.len() as u64));
            //             for (currency_id, reward_per_token) in reward_per_token_storage_map_v0.into_iter() {
            //                 RewardPerToken::<T, I>::insert(currency_id, (), reward_per_token);
            //                 weight.saturating_accrue(T::DbWeight::get().writes(1));
            //             }
            //
            //             let stake_storage_map_v0 = v0::Stake::<T, I>::drain().collect::<Vec<_>>();
            //             weight.saturating_accrue(T::DbWeight::get().reads(stake_storage_map_v0.len() as u64));
            //             for (stake_id, stake) in stake_storage_map_v0.into_iter() {
            //                 Stake::<T, I>::insert(((), stake_id), stake);
            //                 weight.saturating_accrue(T::DbWeight::get().writes(1));
            //             }
            //
            //             let reward_tally_storage_map_v0 = v0::RewardTally::<T, I>::drain().collect::<Vec<_>>();
            //             weight.saturating_accrue(T::DbWeight::get().reads(reward_tally_storage_map_v0.len() as u64));
            //             for (currency_id, stake_id, reward_tally) in reward_tally_storage_map_v0.into_iter() {
            //                 RewardTally::<T, I>::insert(currency_id, ((), stake_id), reward_tally);
            //                 weight.saturating_accrue(T::DbWeight::get().writes(1));
            //             }
            //
            //             StorageVersion::new(1).put::<Pallet<T, I>>();
            //             weight.saturating_add(T::DbWeight::get().reads_writes(1, 2))
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
            assert_eq!(StorageVersion::get::<Pallet<T, I>>(), 1, "Must upgrade");

            // TODO: verify final state
            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
    use super::*;
    use crate::mock::*;
    use sp_arithmetic::FixedI128;

    #[test]
    #[allow(deprecated)]
    fn migration_v0_to_v1_works() {
        run_test(|| {
            // assume that we are at v0
            StorageVersion::new(0).put::<Reward>();

            // first account deposits 50
            v0::Stake::<Test, ()>::insert(0, FixedI128::from(50));
            // total stake is updated to 50
            v0::TotalStake::<Test, ()>::put(FixedI128::from(50));
            // distribute 100 rewards
            // reward / total_stake = 100 / 50 = 2
            v0::RewardPerToken::<Test, ()>::insert(Token(IBTC), FixedI128::from(2));
            // second account deposits 50
            v0::Stake::<Test, ()>::insert(1, FixedI128::from(50));
            // total stake is updated to 100
            v0::TotalStake::<Test, ()>::put(FixedI128::from(100));
            // reward_per_token * amount = 2 * 50
            v0::RewardTally::<Test, ()>::insert(Token(IBTC), 1, FixedI128::from(100));

            let state = v1::MigrateToV1::<Test>::pre_upgrade().unwrap();
            let _w = v1::MigrateToV1::<Test>::on_runtime_upgrade();
            v1::MigrateToV1::<Test>::post_upgrade(state).unwrap();

            assert_eq!(Stake::<Test>::get(((), 0)), FixedI128::from(50));
            assert_eq!(Stake::<Test>::get(((), 1)), FixedI128::from(50));
            assert_eq!(TotalStake::<Test>::get(()), FixedI128::from(100));
            assert_eq!(RewardPerToken::<Test>::get(Token(IBTC), ()), FixedI128::from(2));
            assert_eq!(RewardTally::<Test>::get(Token(IBTC), ((), 0)), FixedI128::from(0));
            assert_eq!(RewardTally::<Test>::get(Token(IBTC), ((), 1)), FixedI128::from(100));

            assert_eq!(StorageVersion::get::<Reward>(), 1);
        });
    }
}
