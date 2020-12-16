//! # PolkaBTC Issue implementation
//! The Issue module according to the specification at
//! https://interlay.gitlab.io/polkabtc-spec/spec/issue.html

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

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

pub use crate::types::IssueRequest;

use crate::types::{IssueRequestV0, PolkaBTC, Version, DOT};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::weights::Weight;
/// # PolkaBTC Issue implementation
/// The Issue module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/issue.html
// Substrate
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use frame_system::{ensure_root, ensure_signed};
use primitive_types::H256;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// The issue module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"issuemod");

pub trait WeightInfo {
    fn request_issue() -> Weight;
    fn execute_issue() -> Weight;
    fn cancel_issue() -> Weight;
    fn set_issue_period() -> Weight;
}

/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait
    + vault_registry::Trait
    + collateral::Trait
    + btc_relay::Trait
    + treasury::Trait
    + exchange_rate_oracle::Trait
    + fee::Trait
    + sla::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Issue {
        /// Users create issue requests to issue PolkaBTC. This mapping provides access
        /// from a unique hash `IssueId` to an `IssueRequest` struct.
        IssueRequests: map hasher(blake2_128_concat) H256 => IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;

        /// The time difference in number of blocks between an issue request is created
        /// and required completion time by a user. The issue period has an upper limit
        /// to prevent griefing of vault collateral.
        IssuePeriod get(fn issue_period) config(): T::BlockNumber;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
    {
        RequestIssue(H256, AccountId, PolkaBTC, AccountId, BtcAddress),
        ExecuteIssue(H256, AccountId, AccountId),
        CancelIssue(H256, AccountId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            use frame_support::{migration::StorageKeyIterator, Blake2_128Concat};

            if Self::storage_version() == Version::V0 {
                StorageKeyIterator::<H256, IssueRequestV0<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Blake2_128Concat>::new(<IssueRequests<T>>::module_prefix(), b"IssueRequests")
                    .drain()
                    .for_each(|(id, request_v0)| {
                        let request_v1 = IssueRequest {
                            vault: request_v0.vault,
                            opentime: request_v0.opentime,
                            griefing_collateral: request_v0.griefing_collateral,
                            amount: request_v0.amount,
                            fee: PolkaBTC::<T>::default(),
                            requester: request_v0.requester,
                            btc_address: BtcAddress::P2WPKHv0(request_v0.btc_address),
                            completed: request_v0.completed,
                            cancelled: false,
                        };
                        <IssueRequests<T>>::insert(id, request_v1);
                    });

                StorageVersion::put(Version::V1);
            }

            0
        }

        /// Request the issuance of PolkaBTC
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of DOT
        #[weight = <T as Trait>::WeightInfo::request_issue()]
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
        #[weight = <T as Trait>::WeightInfo::execute_issue()]
        fn execute_issue(origin, issue_id: H256, tx_id: H256Le, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let executor = ensure_signed(origin)?;
            Self::_execute_issue(executor, issue_id, tx_id, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// Cancel the issuance of PolkaBTC if expired
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `issue_id` - identifier of issue request as output from request_issue
        #[weight = <T as Trait>::WeightInfo::cancel_issue()]
        fn cancel_issue(origin, issue_id: H256)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_cancel_issue(requester, issue_id)?;
            Ok(())
        }

        /// Set the default issue period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Trait>::WeightInfo::set_issue_period()]
        fn set_issue_period(origin, period: T::BlockNumber) {
            ensure_root(origin)?;
            <IssuePeriod<T>>::set(period);
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Requests CBA issuance, returns unique tracking ID.
    fn _request_issue(
        requester: T::AccountId,
        amount_polkabtc: PolkaBTC<T>,
        vault_id: T::AccountId,
        griefing_collateral: DOT<T>,
    ) -> Result<H256, DispatchError> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let height = <frame_system::Module<T>>::block_number();
        let _vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // Check that the vault is currently not banned
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;

        let amount_dot = ext::oracle::btc_to_dots::<T>(amount_polkabtc)?;
        let expected_griefing_collateral =
            ext::fee::get_issue_griefing_collateral::<T>(amount_dot)?;

        ensure!(
            griefing_collateral >= expected_griefing_collateral,
            Error::<T>::InsufficientCollateral
        );
        ext::collateral::lock_collateral::<T>(&requester, griefing_collateral)?;

        let fee_polkabtc = ext::fee::get_issue_fee::<T>(amount_polkabtc)?;
        let amount_btc = amount_polkabtc + fee_polkabtc;

        let btc_address =
            ext::vault_registry::increase_to_be_issued_tokens::<T>(&vault_id, amount_btc)?;

        let issue_id = ext::security::get_secure_id::<T>(&requester);

        Self::insert_issue_request(
            issue_id,
            IssueRequest {
                vault: vault_id.clone(),
                opentime: height,
                requester: requester.clone(),
                btc_address: btc_address.clone(),
                completed: false,
                cancelled: false,
                amount: amount_polkabtc,
                fee: fee_polkabtc,
                griefing_collateral,
            },
        );

        Self::deposit_event(<Event<T>>::RequestIssue(
            issue_id,
            requester,
            amount_btc,
            vault_id,
            btc_address,
        ));
        Ok(issue_id)
    }

    /// Completes CBA issuance, removing request from storage and minting token.
    fn _execute_issue(
        executor: T::AccountId,
        issue_id: H256,
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let issue = Self::get_issue_request_from_id(&issue_id)?;
        // allow anyone to complete issue request
        let requester = issue.requester;

        // only executable before the request has expired
        ensure!(
            !has_request_expired::<T>(issue.opentime, Self::issue_period()),
            Error::<T>::CommitPeriodExpired
        );

        let total_amount = issue.amount + issue.fee;
        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            TryInto::<u64>::try_into(total_amount).map_err(|_e| Error::<T>::TryIntoIntError)?
                as i64,
            issue.btc_address,
            issue_id.clone().as_bytes().to_vec(),
        )?;

        ext::vault_registry::issue_tokens::<T>(&issue.vault, total_amount)?;
        ext::collateral::release_collateral::<T>(&requester, issue.griefing_collateral)?;

        // mint polkabtc amount
        ext::treasury::mint::<T>(requester.clone(), issue.amount);

        // mint polkabtc fees
        ext::treasury::mint::<T>(ext::fee::fee_pool_account_id::<T>(), issue.fee);
        ext::fee::increase_polka_btc_rewards_for_epoch::<T>(issue.fee);

        // if it was a vault that did the execution on behalf of someone else, reward it by
        // increasing its SLA score
        if &requester != &executor {
            if let Ok(vault) = ext::vault_registry::get_vault_from_id::<T>(&executor) {
                ext::sla::event_update_vault_sla::<T>(
                    vault.id,
                    ext::sla::VaultEvent::SubmittedIssueProof,
                )?;
            }
        }

        // reward the vault for having issued PolkaBTC by increasing its sla
        ext::sla::event_update_vault_sla::<T>(
            issue.vault.clone(),
            ext::sla::VaultEvent::ExecutedIssue(issue.amount),
        )?;

        // Remove issue request from storage
        Self::remove_issue_request(issue_id, false);

        Self::deposit_event(<Event<T>>::ExecuteIssue(issue_id, requester, issue.vault));
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_issue(requester: T::AccountId, issue_id: H256) -> Result<(), DispatchError> {
        let issue = Self::get_issue_request_from_id(&issue_id)?;

        // only cancellable after the request has expired
        ensure!(
            has_request_expired::<T>(issue.opentime, Self::issue_period()),
            Error::<T>::TimeNotExpired
        );

        ext::vault_registry::decrease_to_be_issued_tokens::<T>(&issue.vault, issue.amount)?;
        ext::collateral::slash_collateral::<T>(
            &issue.requester,
            &issue.vault,
            issue.griefing_collateral,
        )?;

        // Remove issue request from storage
        Self::remove_issue_request(issue_id, true);

        Self::deposit_event(<Event<T>>::CancelIssue(issue_id, requester));
        Ok(())
    }

    /// Fetch all issue requests for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_issue_requests_for_account(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
        <IssueRequests<T>>::iter()
            .filter(|(_, request)| request.requester == account_id)
            .collect::<Vec<_>>()
    }

    /// Fetch all issue requests for the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - vault account id
    pub fn get_issue_requests_for_vault(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
        <IssueRequests<T>>::iter()
            .filter(|(_, request)| request.vault == account_id)
            .collect::<Vec<_>>()
    }

    pub fn get_issue_request_from_id(
        issue_id: &H256,
    ) -> Result<IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        ensure!(
            <IssueRequests<T>>::contains_key(*issue_id),
            Error::<T>::IssueIdNotFound
        );
        // NOTE: temporary workaround until we delete
        ensure!(
            !<IssueRequests<T>>::get(*issue_id).completed,
            Error::<T>::IssueCompleted
        );
        ensure!(
            !<IssueRequests<T>>::get(*issue_id).cancelled,
            Error::<T>::IssueCancelled
        );
        Ok(<IssueRequests<T>>::get(*issue_id))
    }

    fn insert_issue_request(
        key: H256,
        value: IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <IssueRequests<T>>::insert(key, value)
    }

    fn remove_issue_request(id: H256, cancelled: bool) {
        // TODO: delete issue request from storage
        <IssueRequests<T>>::mutate(id, |request| {
            request.completed = !cancelled;
            request.cancelled = cancelled;
        });
    }
}

fn has_request_expired<T: Trait>(opentime: T::BlockNumber, period: T::BlockNumber) -> bool {
    let height = <frame_system::Module<T>>::block_number();
    height > opentime + period
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        InsufficientCollateral,
        IssueIdNotFound,
        CommitPeriodExpired,
        TimeNotExpired,
        IssueCompleted,
        IssueCancelled,
        /// Unable to convert value
        TryIntoIntError,
    }
}
