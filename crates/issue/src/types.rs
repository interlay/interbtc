use codec::{Decode, Encode, MaxEncodedLen};
use currency::Amount;
use frame_support::traits::Get;
pub use primitives::issue::{IssueRequest, IssueRequestStatus};
use primitives::VaultId;
use scale_info::TypeInfo;
use vault_registry::types::CurrencyId;

use crate::Config;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// IssueRequestStatus
    V2,
    /// ActiveBlockNumber, btc_height
    V3,
    /// Removed refund
    V4,
}

pub mod v4 {
    use super::*;
    use crate::{BtcAddress, BtcPublicKey, H256};
    use frame_support::pallet_prelude::*;

    #[frame_support::storage_alias]
    pub type IssueCountBefore<T: crate::Config> = StorageValue<crate::Pallet<T>, u32, ValueQuery>;

    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    pub enum IssueRequestStatusV3 {
        Pending,
        Completed(Option<H256>),
        Cancelled,
    }

    impl Into<IssueRequestStatus> for IssueRequestStatusV3 {
        fn into(self) -> IssueRequestStatus {
            match self {
                IssueRequestStatusV3::Pending => IssueRequestStatus::Pending,
                IssueRequestStatusV3::Completed(_) => IssueRequestStatus::Completed,
                IssueRequestStatusV3::Cancelled => IssueRequestStatus::Cancelled,
            }
        }
    }

    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    pub struct IssueRequestV3<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        pub vault: VaultId<AccountId, CurrencyId>,
        pub opentime: BlockNumber,
        pub period: BlockNumber,
        pub griefing_collateral: Balance,
        pub amount: Balance,
        pub fee: Balance,
        pub requester: AccountId,
        pub btc_address: BtcAddress,
        pub btc_public_key: BtcPublicKey,
        pub btc_height: u32,
        pub status: IssueRequestStatusV3,
    }

    pub type DefaultIssueRequestV3<T> = IssueRequestV3<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        CurrencyId<T>,
    >;

    pub fn migrate_v0_to_v4<T: Config>() -> frame_support::weights::Weight {
        use sp_runtime::traits::Saturating;

        // NOTE: kintsugi & interlay still on version 0
        if !matches!(crate::StorageVersion::<T>::get(), Version::V0) {
            log::info!("Not running issue storage migration");
            return T::DbWeight::get().reads(1); // already upgraded; don't run migration
        }
        let mut num_migrated_issues = 0u64;

        crate::IssueRequests::<T>::translate::<DefaultIssueRequestV3<T>, _>(|_key, issue_v3| {
            num_migrated_issues.saturating_inc();

            Some(IssueRequest {
                vault: issue_v3.vault,
                opentime: issue_v3.opentime,
                period: issue_v3.period,
                griefing_collateral: issue_v3.griefing_collateral,
                amount: issue_v3.amount,
                fee: issue_v3.fee,
                requester: issue_v3.requester,
                btc_address: issue_v3.btc_address,
                btc_public_key: issue_v3.btc_public_key,
                btc_height: issue_v3.btc_height,
                status: issue_v3.status.into(),
            })
        });
        crate::StorageVersion::<T>::put(Version::V4);

        log::info!("Migrated {num_migrated_issues} issues");

        T::DbWeight::get().reads_writes(num_migrated_issues, num_migrated_issues)
    }

    #[cfg(test)]
    #[test]
    fn test_migration() {
        use crate::mock::Test;
        use frame_support::{storage::migration, Blake2_128Concat, StorageHasher};
        use primitives::{CurrencyId::Token, VaultCurrencyPair, KBTC, KSM};
        use sp_runtime::traits::TrailingZeroInput;

        crate::mock::run_test(|| {
            crate::StorageVersion::<Test>::put(Version::V0);

            let issue_v3: DefaultIssueRequestV3<Test> = IssueRequestV3 {
                vault: VaultId {
                    account_id: <Test as frame_system::Config>::AccountId::decode(&mut TrailingZeroInput::zeroes())
                        .unwrap(),
                    currencies: VaultCurrencyPair {
                        collateral: Token(KSM),
                        wrapped: Token(KBTC),
                    },
                },
                opentime: 1_501_896,
                period: 14_400,
                griefing_collateral: 20,
                amount: 100,
                fee: 10,
                requester: <Test as frame_system::Config>::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
                btc_address: BtcAddress::P2PKH(sp_core::H160::from([1; 20])),
                btc_public_key: BtcPublicKey::from([1; 33]),
                btc_height: 754_190,
                status: IssueRequestStatusV3::Completed(None),
            };

            let issue_id = crate::ext::security::get_secure_id::<Test>(
                &<Test as frame_system::Config>::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
            );

            migration::put_storage_value(
                b"Issue",
                b"IssueRequests",
                &Blake2_128Concat::hash(&issue_id.encode()),
                &issue_v3,
            );

            migrate_v0_to_v4::<Test>();

            let issue_v4 = IssueRequest {
                vault: issue_v3.vault,
                opentime: issue_v3.opentime,
                period: issue_v3.period,
                griefing_collateral: issue_v3.griefing_collateral,
                amount: issue_v3.amount,
                fee: issue_v3.fee,
                requester: issue_v3.requester,
                btc_address: issue_v3.btc_address,
                btc_public_key: issue_v3.btc_public_key,
                btc_height: issue_v3.btc_height,
                status: issue_v3.status.into(),
            };

            // check that migration was applied correctly
            assert_eq!(
                crate::IssueRequests::<Test>::iter().collect::<Vec<_>>(),
                vec![(issue_id.clone(), issue_v4)]
            );

            // check that storage version is bumped
            assert!(crate::StorageVersion::<Test>::get() == Version::V4);
        });
    }
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub type DefaultIssueRequest<T> = IssueRequest<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    BalanceOf<T>,
    CurrencyId<T>,
>;

pub trait IssueRequestExt<T: Config> {
    fn amount(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn griefing_collateral(&self) -> Amount<T>;
}

impl<T: Config> IssueRequestExt<T> for DefaultIssueRequest<T> {
    fn amount(&self) -> Amount<T> {
        Amount::new(self.amount, self.vault.wrapped_currency())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, self.vault.wrapped_currency())
    }
    fn griefing_collateral(&self) -> Amount<T> {
        Amount::new(self.griefing_collateral, T::GetGriefingCollateralCurrencyId::get())
    }
}
