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

#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::types::{PolkaBTC, DOT};
use bitcoin::types::H256Le;
use codec::{Decode, Encode};
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
use frame_system::ensure_signed;
use primitive_types::H256;
use sp_core::H160;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// The issue module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"issuemod");

pub trait WeightInfo {
    fn request_issue() -> Weight;
    fn execute_issue() -> Weight;
    fn cancel_issue() -> Weight;
}

/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait + vault_registry::Trait + collateral::Trait + btc_relay::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// Due to a known bug in serde we need to specify how u128 is (de)serialized.
// See https://github.com/paritytech/substrate/issues/4641
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct IssueRequest<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub griefing_collateral: DOT,
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "PolkaBTC: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "PolkaBTC: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount: PolkaBTC,
    pub requester: AccountId,
    pub btc_address: H160,
    pub completed: bool,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
    t: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
    deserializer: D,
) -> Result<T, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Issue {
        /// Users create issue requests to issue PolkaBTC. This mapping provides access
        /// from a unique hash `IssueId` to an `IssueRequest` struct.
        IssueRequests: map hasher(blake2_128_concat) H256 => IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;

        /// The minimum collateral (DOT) a user needs to provide as griefing protection.
        IssueGriefingCollateral get(fn issue_griefing_collateral) config(): DOT<T>;

        /// The time difference in number of blocks between an issue request is created
        /// and required completion time by a user. The issue period has an upper limit
        /// to prevent griefing of vault collateral.
        IssuePeriod get(fn issue_period) config(): T::BlockNumber;
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
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

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
        fn execute_issue(origin, issue_id: H256, tx_id: H256Le, _tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_execute_issue(requester, issue_id, tx_id, merkle_proof, raw_tx)?;
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
    ) -> Result<H256, DispatchError> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let height = <frame_system::Module<T>>::block_number();
        let _vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // Check that the vault is currently not banned
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;

        ensure!(
            griefing_collateral >= Self::issue_griefing_collateral(),
            Error::<T>::InsufficientCollateral
        );

        ext::collateral::lock_collateral::<T>(&requester, griefing_collateral)?;

        let btc_address =
            ext::vault_registry::increase_to_be_issued_tokens::<T>(&vault_id, amount)?;

        let key = ext::security::get_secure_id::<T>(&requester);

        Self::insert_issue_request(
            key,
            IssueRequest {
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
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        // Check that Parachain is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        let issue = Self::get_issue_request_from_id(&issue_id)?;
        ensure!(requester == issue.requester, Error::<T>::UnauthorizedUser);

        let height = <frame_system::Module<T>>::block_number();
        let period = Self::issue_period();
        ensure!(
            height <= issue.opentime + period,
            Error::<T>::CommitPeriodExpired
        );

        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            TryInto::<u64>::try_into(issue.amount).map_err(|_e| Error::<T>::ConversionError)?
                as i64,
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
    fn _cancel_issue(requester: T::AccountId, issue_id: H256) -> Result<(), DispatchError> {
        let issue = Self::get_issue_request_from_id(&issue_id)?;
        let height = <frame_system::Module<T>>::block_number();
        let period = Self::issue_period();

        ensure!(height > issue.opentime + period, Error::<T>::TimeNotExpired);
        ensure!(!issue.completed, Error::<T>::IssueCompleted);

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

    fn get_issue_request_from_id(
        issue_id: &H256,
    ) -> Result<IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        ensure!(
            <IssueRequests<T>>::contains_key(*issue_id),
            Error::<T>::IssueIdNotFound
        );
        Ok(<IssueRequests<T>>::get(*issue_id))
    }

    fn insert_issue_request(
        key: H256,
        value: IssueRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <IssueRequests<T>>::insert(key, value)
    }

    fn remove_issue_request(id: H256) {
        <IssueRequests<T>>::remove(id);
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        InsufficientCollateral,
        IssueIdNotFound,
        CommitPeriodExpired,
        UnauthorizedUser,
        TimeNotExpired,
        IssueCompleted,
        ConversionError,
    }
}
