#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

mod ext;
pub mod types;

use crate::types::{PolkaBTC, DOT};
use bitcoin::types::H256Le;
use codec::{Decode, Encode};
/// # PolkaBTC Issue implementation
/// The Issue module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/issue.html
// Substrate
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure, traits::Get,
};
use frame_system::ensure_signed;
use primitive_types::H256;
use sp_core::H160;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use x_core::Error;

/// The issue module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"issuemod");

/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait + vault_registry::Trait + collateral::Trait + btc_relay::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The time difference in number of blocks between an issue request is created
    /// and required completion time by a user. The issue period has an upper limit
    /// to prevent griefing of vault collateral.
    type IssuePeriod: Get<Self::BlockNumber>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Issue<AccountId, BlockNumber, PolkaBTC, DOT> {
    vault: AccountId,
    opentime: BlockNumber,
    griefing_collateral: DOT,
    amount: PolkaBTC,
    requester: AccountId,
    btc_address: H160,
    completed: bool,
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Issue {
        /// The minimum collateral (DOT) a user needs to provide as griefing protection.
        IssueGriefingCollateral: DOT<T>;

        /// Users create issue requests to issue PolkaBTC. This mapping provides access
        /// from a unique hash `IssueId` to an `Issue` struct.
        IssueRequests: map hasher(blake2_128_concat) H256 => Issue<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
    {
        RequestIssue(H256, AccountId, PolkaBTC, AccountId, H160),
        ExecuteIssue(H256, AccountId, AccountId),
        CancelIssue(H256, AccountId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        const IssuePeriod: T::BlockNumber = T::IssuePeriod::get();

        /// Request the issuance of PolkaBTC
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of DOT
        #[weight = 1000]
        fn request_issue(origin, amount: PolkaBTC<T>, vault_id: T::AccountId, griefing_collateral: DOT<T>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_request_issue(requester, amount, vault_id, griefing_collateral)?;
            Ok(())
        }

        /// Finalize the issuance of PolkaBTC
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `issue_id` - identifier of issue request as output from request_issue
        /// * `tx_id` - transaction hash
        /// * `tx_block_height` - block number of backing chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        #[weight = 1000]
        fn execute_issue(origin, issue_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_execute_issue(requester, issue_id, tx_id, tx_block_height, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// Cancel the issuance of PolkaBTC if expired
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `issue_id` - identifier of issue request as output from request_issue
        #[weight = 1000]
        fn cancel_issue(origin, issue_id: H256)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_cancel_issue(requester, issue_id)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Requests CBA issuance, returns unique tracking ID.
    fn _request_issue(
        requester: T::AccountId,
        amount: PolkaBTC<T>,
        vault_id: T::AccountId,
        griefing_collateral: DOT<T>,
    ) -> Result<H256, Error> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let height = <frame_system::Module<T>>::block_number();
        let _vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // Check that the vault is currently not banned
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;

        ensure!(
            griefing_collateral >= <IssueGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );

        ext::collateral::lock_collateral::<T>(&requester, griefing_collateral)?;

        let btc_address =
            ext::vault_registry::increase_to_be_issued_tokens::<T>(&vault_id, amount)?;

        let key = ext::security::get_secure_id::<T>(&requester);

        Self::insert_issue_request(
            key,
            Issue {
                vault: vault_id.clone(),
                opentime: height,
                griefing_collateral: griefing_collateral,
                amount: amount,
                requester: requester.clone(),
                btc_address: btc_address,
                completed: false,
            },
        );

        Self::deposit_event(<Event<T>>::RequestIssue(
            key,
            requester,
            amount,
            vault_id,
            btc_address,
        ));
        Ok(key)
    }

    /// Completes CBA issuance, removing request from storage and minting token.
    fn _execute_issue(
        requester: T::AccountId,
        issue_id: H256,
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let issue = Self::get_issue_request_from_id(&issue_id)?;
        ensure!(requester == issue.requester, Error::UnauthorizedUser);

        let height = <frame_system::Module<T>>::block_number();
        let period = T::IssuePeriod::get();
        ensure!(
            height <= issue.opentime + period,
            Error::CommitPeriodExpired
        );

        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, tx_block_height, merkle_proof)?;
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            TryInto::<u64>::try_into(issue.amount).map_err(|_e| Error::RuntimeError)? as i64,
            issue.btc_address.as_bytes().to_vec(),
            issue_id.clone().as_bytes().to_vec(),
        )?;

        ext::vault_registry::issue_tokens::<T>(&issue.vault, issue.amount)?;
        ext::treasury::mint::<T>(issue.requester, issue.amount);
        // Remove issue request from storage
        Self::remove_issue_request(issue_id);

        Self::deposit_event(<Event<T>>::ExecuteIssue(issue_id, requester, issue.vault));
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_issue(requester: T::AccountId, issue_id: H256) -> Result<(), Error> {
        let issue = Self::get_issue_request_from_id(&issue_id)?;
        let height = <frame_system::Module<T>>::block_number();
        let period = T::IssuePeriod::get();

        ensure!(issue.opentime + period > height, Error::TimeNotExpired);
        ensure!(!issue.completed, Error::IssueCompleted);

        ext::vault_registry::decrease_to_be_issued_tokens::<T>(&issue.vault, issue.amount)?;
        ext::collateral::slash_collateral::<T>(
            &issue.requester,
            &issue.vault,
            issue.griefing_collateral,
        )?;

        // Remove issue request from storage
        Self::remove_issue_request(issue_id);

        Self::deposit_event(<Event<T>>::CancelIssue(issue_id, requester));
        Ok(())
    }

    fn get_issue_request_from_id(
        issue_id: &H256,
    ) -> Result<Issue<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        ensure!(
            <IssueRequests<T>>::contains_key(*issue_id),
            Error::IssueIdNotFound
        );
        Ok(<IssueRequests<T>>::get(*issue_id))
    }

    fn insert_issue_request(
        key: H256,
        value: Issue<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <IssueRequests<T>>::insert(key, value)
    }

    #[allow(dead_code)]
    fn set_issue_griefing_collateral(amount: DOT<T>) {
        <IssueGriefingCollateral<T>>::set(amount);
    }

    fn remove_issue_request(id: H256) {
        <IssueRequests<T>>::remove(id);
    }
}
