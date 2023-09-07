pub use primitives::redeem::{RedeemRequest, RedeemRequestStatus};
use primitives::VaultId;
use scale_info::TypeInfo;
use sp_runtime::DispatchError;
use vault_registry::types::CurrencyId;

use crate::{Config, Pallet};
use btc_relay::BtcAddress;
use codec::{Decode, Encode, MaxEncodedLen};
use currency::Amount;
use frame_support::{pallet_prelude::OptionQuery, traits::Get, Blake2_128Concat};
use sp_core::H256;

/// Storage version.
#[derive(Debug, Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
    /// Added `issue_id` to `RedeemRequest`
    V1,
}

mod v0 {
    use super::*;

    pub type DefaultRedeemRequest<T> = RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        CurrencyId<T>,
    >;

    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    pub struct RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        /// the vault associated with this redeem request
        pub vault: VaultId<AccountId, CurrencyId>,
        /// the *active* block height when this request was opened
        pub opentime: BlockNumber,
        /// the redeem period when this request was opened
        pub period: BlockNumber,
        /// total redeem fees - taken from request amount
        pub fee: Balance,
        /// amount the vault should spend on the bitcoin inclusion fee - taken from request amount
        pub transfer_fee_btc: Balance,
        /// total amount of BTC for the vault to send
        pub amount_btc: Balance,
        /// premium redeem amount in collateral
        pub premium: Balance,
        /// the account redeeming tokens (for BTC)
        pub redeemer: AccountId,
        /// the user's Bitcoin address for payment verification
        pub btc_address: BtcAddress,
        /// the highest recorded height in the BTC-Relay (at time of opening)
        pub btc_height: u32,
        /// the status of this redeem request
        pub status: RedeemRequestStatus,
    }

    #[frame_support::storage_alias]
    pub(super) type RedeemRequests<T: Config> =
        StorageMap<Pallet<T>, Blake2_128Concat, H256, DefaultRedeemRequest<T>, OptionQuery>;
}

pub mod v1 {
    use super::*;

    pub fn migrate_v0_to_v1<T: Config>() -> frame_support::weights::Weight {
        if !matches!(crate::StorageVersion::<T>::get(), Version::V0) {
            return T::DbWeight::get().reads(1); // already upgraded; don't run migration
        }
        // update vault struct to remove replace pallet fields
        crate::RedeemRequests::<T>::translate(|_key, old: v0::DefaultRedeemRequest<T>| {
            Some(crate::DefaultRedeemRequest::<T> {
                vault: old.vault,
                opentime: old.opentime,
                period: old.period,
                fee: old.fee,
                transfer_fee_btc: old.transfer_fee_btc,
                amount_btc: old.amount_btc,
                premium: old.premium,
                redeemer: old.redeemer,
                btc_address: old.btc_address,
                btc_height: old.btc_height,
                status: old.status,
                issue_id: None,
            })
        });

        // update version
        crate::StorageVersion::<T>::put(Version::V1);

        T::DbWeight::get().reads_writes(0, 1)
    }
}

pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub type DefaultRedeemRequest<T> = RedeemRequest<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    BalanceOf<T>,
    CurrencyId<T>,
>;

pub trait RedeemRequestExt<T: Config> {
    fn amount_btc(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn premium(&self) -> Result<Amount<T>, DispatchError>;
    fn transfer_fee_btc(&self) -> Amount<T>;
}

impl<T: Config> RedeemRequestExt<T> for RedeemRequest<T::AccountId, T::BlockNumber, BalanceOf<T>, CurrencyId<T>> {
    fn amount_btc(&self) -> Amount<T> {
        Amount::new(self.amount_btc, self.vault.wrapped_currency())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, self.vault.wrapped_currency())
    }
    fn premium(&self) -> Result<Amount<T>, DispatchError> {
        Ok(Amount::new(self.premium, self.vault.collateral_currency()))
    }
    fn transfer_fee_btc(&self) -> Amount<T> {
        Amount::new(self.transfer_fee_btc, self.vault.wrapped_currency())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mock::{Test as T, *};

    #[test]
    fn migrating_from_v0_to_v1() {
        run_test(|| {
            assert_eq!(crate::StorageVersion::<T>::get(), Version::V0);

            let old = v0::DefaultRedeemRequest::<T> {
                vault: DefaultVaultId::<T>::new(123, Token(DOT), Token(IBTC)),
                opentime: 1,
                period: 12313,
                fee: 456,
                transfer_fee_btc: 1,
                amount_btc: 100,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Reimbursed(false),
            };
            let key = H256::zero();
            v0::RedeemRequests::<T>::insert(key, old.clone());

            v1::migrate_v0_to_v1::<T>();

            assert_eq!(crate::StorageVersion::<T>::get(), Version::V1);

            let new = crate::RedeemRequests::<T>::get(key).unwrap();

            assert!(old.vault == new.vault);
            assert!(old.opentime == new.opentime);
            assert!(old.period == new.period);
            assert!(old.fee == new.fee);
            assert!(old.transfer_fee_btc == new.transfer_fee_btc);
            assert!(old.amount_btc == new.amount_btc);
            assert!(old.premium == new.premium);
            assert!(old.redeemer == new.redeemer);
            assert!(old.btc_address == new.btc_address);
            assert!(old.btc_height == new.btc_height);
            assert!(old.status == new.status);
            assert_eq!(new.issue_id.is_none(), true);
        });
    }
}
