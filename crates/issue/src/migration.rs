use super::*;
use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use sp_core::H256;

/// The log target.
const TARGET: &'static str = "runtime::issue::migration::v1";

/// The original data layout of the democracy pallet without a specific version number.
mod v0 {
    use super::*;

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct IssueRequest<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        /// the vault associated with this issue request
        pub vault: primitives::VaultId<AccountId, CurrencyId>,
        /// the *active* block height when this request was opened
        pub opentime: BlockNumber,
        /// the issue period when this request was opened
        pub period: BlockNumber,
        /// the collateral held for spam prevention
        pub griefing_collateral: Balance,
        /// the number of tokens that will be transferred to the user (as such, this does not include the fee)
        pub amount: Balance,
        /// the number of tokens that will be transferred to the fee pool
        pub fee: Balance,
        /// the account issuing tokens
        pub requester: AccountId,
        /// the vault's Bitcoin deposit address
        pub btc_address: BtcAddress,
        /// the vault's Bitcoin public key (when this request was made)
        pub btc_public_key: BtcPublicKey,
        /// the highest recorded height in the BTC-Relay (at time of opening)
        pub btc_height: u32,
        /// the status of this issue request
        pub status: IssueRequestStatus,
    }

    pub type DefaultIssueRequest<T> = IssueRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        <T as currency::Config>::Balance,
        vault_registry::types::CurrencyId<T>,
    >;
}

pub mod v1 {
    use super::*;

    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config + frame_system::Config<Hash = H256>> OnRuntimeUpgrade for Migration<T> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 0, "can only upgrade from version 0");

            let issue_count = v0::IssueRequests::<T>::get().len();
            log::info!(target: TARGET, "{} public proposals will be migrated.", issue_count,);

            Ok((issue_count as u32).encode())
        }

        #[allow(deprecated)]
        fn on_runtime_upgrade() -> Weight {
            let mut weight = T::DbWeight::get().reads(1);
            if StorageVersion::get::<Pallet<T>>() != 0 {
                log::warn!(
                    target: TARGET,
                    "skipping on_runtime_upgrade: executed on wrong storage version.\
				Expected version 0"
                );
                return weight;
            }

            IssueRequests::<T>::translate(|key, old: v0::DefaultIssueRequest<T>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
                log::info!(target: TARGET, "migrating issue #{:?}", &key);

                Some(DefaultIssueRequest::<T> {
                    vault: old.vault,
                    opentime: old.opentime,
                    period: old.period,
                    griefing_collateral: old.griefing_collateral,
                    amount: old.amount,
                    fee: old.fee,
                    requester: old.requester,
                    btc_address: old.btc_address,
                    btc_public_key: old.btc_public_key,
                    btc_height: old.btc_height,
                    status: old.status,
                    griefing_currency: T::GetGriefingCollateralCurrencyId::get(),
                })
            });

            StorageVersion::new(1).put::<Pallet<T>>();
            weight.saturating_add(T::DbWeight::get().reads_writes(0, 1))
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 1, "must upgrade");

            let (old_issue_count): u32 =
                Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
            let new_count = crate::IssueRequests::<T>::get().len() as u32;
            assert_eq!(new_count, old_issue_count, "must migrate all public proposals");

            log::info!(target: TARGET, "{} issues migrated", new_count,);
            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
    use super::*;
    use crate::{
        tests::{Test as T, *},
        types::*,
    };
    use frame_support::bounded_vec;

    #[allow(deprecated)]
    #[test]
    fn migration_works() {
        new_test_ext().execute_with(|| {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 0);
            // Insert some values into the v0 storage:

            // Case 1: Ongoing referendum
            let hash = H256::repeat_byte(1);
            let status = ReferendumStatus {
                end: 1u32.into(),
                proposal: hash.clone(),
                threshold: VoteThreshold::SuperMajorityApprove,
                delay: 1u32.into(),
                tally: Tally {
                    ayes: 1u32.into(),
                    nays: 1u32.into(),
                    turnout: 1u32.into(),
                },
            };
            v0::ReferendumInfoOf::<T>::insert(1u32, ReferendumInfo::Ongoing(status));

            // Case 2: Finished referendum
            v0::ReferendumInfoOf::<T>::insert(
                2u32,
                ReferendumInfo::Finished {
                    approved: true,
                    end: 123u32.into(),
                },
            );

            // Case 3: Public proposals
            let hash2 = H256::repeat_byte(2);
            v0::PublicProps::<T>::put(vec![(3u32, hash.clone(), 123u64), (4u32, hash2.clone(), 123u64)]);
            v0::Preimages::<T>::insert(
                hash2,
                v0::PreimageStatus::Available {
                    data: vec![],
                    provider: 0,
                    deposit: 0,
                    since: 0,
                    expiry: None,
                },
            );

            // Migrate.
            let state = v1::Migration::<T>::pre_upgrade().unwrap();
            let _weight = v1::Migration::<T>::on_runtime_upgrade();
            v1::Migration::<T>::post_upgrade(state).unwrap();
            // Check that all values got migrated.

            // Case 1: Ongoing referendum
            assert_eq!(
                ReferendumInfoOf::<T>::get(1u32),
                Some(ReferendumInfo::Ongoing(ReferendumStatus {
                    end: 1u32.into(),
                    proposal: Bounded::from_legacy_hash(hash),
                    threshold: VoteThreshold::SuperMajorityApprove,
                    delay: 1u32.into(),
                    tally: Tally {
                        ayes: 1u32.into(),
                        nays: 1u32.into(),
                        turnout: 1u32.into()
                    },
                }))
            );
            // Case 2: Finished referendum
            assert_eq!(
                ReferendumInfoOf::<T>::get(2u32),
                Some(ReferendumInfo::Finished {
                    approved: true,
                    end: 123u32.into()
                })
            );
            // Case 3: Public proposals
            let props: BoundedVec<_, <Test as Config>::MaxProposals> = bounded_vec![
                (3u32, Bounded::from_legacy_hash(hash), 123u64),
                (4u32, Bounded::from_legacy_hash(hash2), 123u64)
            ];
            assert_eq!(PublicProps::<T>::get(), props);
            assert_eq!(v0::Preimages::<T>::iter().collect::<Vec<_>>().len(), 0);
        });
    }
}
