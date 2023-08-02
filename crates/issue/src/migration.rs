use super::*;
use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use sp_core::H256;

/// The log target.
const TARGET: &'static str = "runtime::issue::migration::v1";

/// The original data layout of the democracy pallet without a specific version number.
mod v0 {
    use super::*;

    #[frame_support::storage_alias]
    pub(super) type IssueRequests<T: Config> =
        StorageMap<Pallet<T>, Blake2_128Concat, H256, DefaultIssueRequest<T>, OptionQuery>;

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

            let issue_count = v0::IssueRequests::<T>::iter().count();
            log::info!(target: TARGET, "{} issues will be migrated.", issue_count,);

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

            let old_issue_count: u32 =
                Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
            let new_count = crate::IssueRequests::<T>::iter().count() as u32;
            assert_eq!(new_count, old_issue_count, "must migrate all issues");

            log::info!(target: TARGET, "{} issues migrated", new_count,);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        mock::{Test as T, *},
        types::*,
    };

    #[allow(deprecated)]
    #[test]
    fn migration_works() {
        run_test(|| {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 0);

            let old = v0::DefaultIssueRequest::<T> {
                requester: 123,
                vault: DefaultVaultId::<T>::new(123, Token(DOT), Token(IBTC)),
                btc_address: BtcAddress::random(),
                amount: 123,
                btc_height: 234,
                btc_public_key: Default::default(),
                fee: 456,
                griefing_collateral: 567,
                opentime: 12334,
                period: 12313,
                status: Default::default(),
            };
            let key = H256::zero();
            v0::IssueRequests::<T>::insert(key, old.clone());

            v1::Migration::<T>::on_runtime_upgrade();

            let new = crate::IssueRequests::<T>::get(key).unwrap();
            assert!(old.requester == new.requester);
            assert!(old.vault == new.vault);
            assert!(old.btc_address == new.btc_address);
            assert!(old.amount == new.amount);
            assert!(old.btc_height == new.btc_height);
            assert!(old.btc_public_key == new.btc_public_key);
            assert!(old.fee == new.fee);
            assert!(old.griefing_collateral == new.griefing_collateral);
            assert!(old.opentime == new.opentime);
            assert!(old.period == new.period);
            assert!(old.status == new.status);
            assert!(new.griefing_currency == <T as vault_registry::Config>::GetGriefingCollateralCurrencyId::get());
        });
    }
}
