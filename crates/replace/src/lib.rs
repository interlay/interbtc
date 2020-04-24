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

use bitcoin::types::H256Le;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
/// # PolkaBTC Replace implementation
/// The Replace module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/issue.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::H160;
use sp_runtime::ModuleId;
use system::ensure_signed;
use x_core::Error;

type DOT<T> = <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;
type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The issue module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"issuemod");

/// The pallet's configuration trait.
pub trait Trait:
    system::Trait + vault_registry::Trait + collateral::Trait + btc_relay::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
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
    trait Store for Module<T: Trait> as Replace {
        ReplaceGriefingCollateral: DOT<T>;
        ReplacePeriod: T::BlockNumber;
        ReplaceRequests: map hasher(blake2_128_concat) H256 => Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
    {
        RequestReplace(H256, AccountId, PolkaBTC, AccountId, H160),
        ExecuteReplace(H256, AccountId, AccountId),
        CancelReplace(H256, AccountId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
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
        // TODO: check precondition
        let height = <system::Module<T>>::block_number();
        // TODO: check vault exists
        let vault = <vault_registry::Module<T>>::get_vault_from_id(vault_id.clone());
        match vault.banned_until {
            Some(until) => ensure!(until < height, Error::VaultBanned),
            None => (),
        };

        ensure!(
            griefing_collateral >= <ReplaceGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );

        <collateral::Module<T>>::lock_collateral(requester.clone(), griefing_collateral)?;

        let btc_address = <vault_registry::Module<T>>::increase_to_be_issued_tokens(
            vault_id.clone(),
            amount.clone(),
        )?;

        let mut hasher = Sha256::default();
        // TODO: nonce from security module
        hasher.input(requester.encode());

        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        let key = H256(result);

        Self::insert_issue_request(
            key,
            Replace {
                vault: vault_id.clone(),
                opentime: height,
                griefing_collateral: griefing_collateral,
                amount: amount,
                requester: requester.clone(),
                btc_address: btc_address,
                completed: false,
            },
        );

        Self::deposit_event(<Event<T>>::RequestReplace(
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
        // TODO: check precondition
        let issue = Self::get_issue_request_from_id(&issue_id)?;
        ensure!(requester == issue.requester, Error::UnauthorizedUser);

        let height = <system::Module<T>>::block_number();
        let period = <ReplacePeriod<T>>::get();
        ensure!(
            period < height && issue.opentime < height - period,
            Error::CommitPeriodExpired
        );

        Self::verify_inclusion_and_validate_transaction(
            tx_id,
            tx_block_height,
            merkle_proof,
            raw_tx,
            0, // TODO: issue.amount,
            issue.btc_address.as_bytes().to_vec(),
            issue_id.clone().as_bytes().to_vec(),
        )?;

        <vault_registry::Module<T>>::issue_tokens(issue.vault.clone(), issue.amount.clone())?;
        <treasury::Module<T>>::mint(issue.requester, issue.amount);
        <ReplaceRequests<T>>::remove(issue_id);

        Self::deposit_event(<Event<T>>::ExecuteReplace(issue_id, requester, issue.vault));
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_issue(requester: T::AccountId, issue_id: H256) -> Result<(), Error> {
        let issue = Self::get_issue_request_from_id(&issue_id)?;
        let height = <system::Module<T>>::block_number();
        let period = <ReplacePeriod<T>>::get();

        ensure!(issue.opentime + period > height, Error::TimeNotExpired);
        ensure!(!issue.completed, Error::IssueCompleted);

        <vault_registry::Module<T>>::decrease_to_be_issued_tokens(
            issue.vault.clone(),
            issue.amount.clone(),
        )?;
        <collateral::Module<T>>::slash_collateral(
            issue.requester.clone(),
            issue.vault.clone(),
            issue.griefing_collateral,
        )?;

        Self::deposit_event(<Event<T>>::CancelReplace(issue_id, requester));
        Ok(())
    }

    fn get_issue_request_from_id(
        issue_id: &H256,
    ) -> Result<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        ensure!(
            <ReplaceRequests<T>>::contains_key(*issue_id),
            Error::IssueIdNotFound
        );
        Ok(<ReplaceRequests<T>>::get(*issue_id))
    }

    fn insert_issue_request(
        key: H256,
        value: Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <ReplaceRequests<T>>::insert(key, value)
    }

    #[allow(dead_code)]
    fn set_issue_griefing_collateral(amount: DOT<T>) {
        <ReplaceGriefingCollateral<T>>::set(amount);
    }

    #[allow(dead_code)]
    fn set_issue_period(value: T::BlockNumber) {
        <ReplacePeriod<T>>::set(value);
    }

    // Note: the calls here are combined to simplify mocking
    fn verify_inclusion_and_validate_transaction(
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: Vec<u8>,
        issue_id: Vec<u8>,
    ) -> Result<(), Error> {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(
            tx_id,
            tx_block_height,
            merkle_proof,
            0,
            false,
        )?;

        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, issue_id)?;

        Ok(())
    }
}
