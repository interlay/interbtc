//! # Refund Pallet

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

mod default_weights;
pub use default_weights::WeightInfo;

mod ext;
pub mod types;

use btc_relay::BtcAddress;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchError, ensure, transactional,
};
use frame_system::ensure_signed;
use sp_core::H256;
use sp_runtime::traits::CheckedSub;
use sp_std::{convert::TryInto, vec::Vec};
pub use types::RefundRequest;
use types::Wrapped;

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + btc_relay::Config
    + currency::Config<currency::Collateral>
    + currency::Config<currency::Wrapped>
    + fee::Config
    + sla::Config
    + vault_registry::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Refund {
        /// The minimum amount of btc that is accepted for refund requests (NOTE: too low
        /// values could result in the bitcoin client rejecting the payment)
        RefundBtcDustValue get(fn refund_btc_dust_value) config(): Wrapped<T>;

        /// This mapping provides access from a unique hash refundId to a Refund struct.
        RefundRequests: map hasher(blake2_128_concat) H256 => RefundRequest<T::AccountId, Wrapped<T>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Wrapped = Wrapped<T>,
    {
        /// refund_id, issuer, amount_without_fee, vault, btc_address, issue_id, fee
        RequestRefund(H256, AccountId, Wrapped, AccountId, BtcAddress, H256, Wrapped),
        /// refund_id, issuer, vault, amount
        ExecuteRefund(H256, AccountId, AccountId, Wrapped),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Initialize errors
        type Error = Error<T>;

        // Initialize events
        fn deposit_event() = default;

        #[weight = <T as Config>::WeightInfo::execute_refund()]
        #[transactional]
        fn execute_refund(
            origin,
            refund_id: H256,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> Result<(), DispatchError> {
            ensure_signed(origin)?;
            Self::_execute_refund(refund_id, merkle_proof, raw_tx)
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    /// User failsafe: when a user accidentally overpays on an issue, and the vault does not
    /// have enough collateral for the the actual sent amount, then this function is called
    /// to request the vault to refund the surplus amount (minus a fee for the vault to keep).
    /// It will deposit an event that the client can listen for.
    ///
    /// # Arguments
    ///
    /// * `total_amount_btc` - the amount that the user has overpaid. This is the amount that will be refunded.
    /// * `vault_id` - id of the vault the issue was made to
    /// * `issuer` - id of the user that made the issue request
    /// * `btc_address` - the btc address that should receive the refund
    pub fn request_refund(
        total_amount_btc: Wrapped<T>,
        vault_id: T::AccountId,
        issuer: T::AccountId,
        btc_address: BtcAddress,
        issue_id: H256,
    ) -> Result<Option<H256>, DispatchError> {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let fee_wrapped = ext::fee::get_refund_fee_from_total::<T>(total_amount_btc)?;
        let net_refund_amount_wrapped = total_amount_btc
            .checked_sub(&fee_wrapped)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        // Only refund if the amount is above the dust value
        let dust_amount = <RefundBtcDustValue<T>>::get();
        if net_refund_amount_wrapped < dust_amount {
            return Ok(None);
        }

        let refund_id = ext::security::get_secure_id::<T>(&issuer);

        let request = RefundRequest {
            vault: vault_id,
            amount_wrapped: net_refund_amount_wrapped,
            fee: fee_wrapped,
            amount_btc: total_amount_btc,
            issuer,
            btc_address,
            issue_id,
            completed: false,
        };
        <RefundRequests<T>>::insert(refund_id, request.clone());

        Self::deposit_event(<Event<T>>::RequestRefund(
            refund_id,
            request.issuer,
            request.amount_wrapped,
            request.vault,
            request.btc_address,
            request.issue_id,
            request.fee,
        ));

        Ok(Some(refund_id))
    }

    /// Finalizes a refund. Typically called by the vault client that performed the refund.
    ///
    /// # Arguments
    ///
    /// * `refund_id` - identifier of a refund request. This ID can be obtained by listening to the RequestRefund event,
    ///   or by querying the open refunds.
    /// * `merkle_proof` - raw bytes of the proof
    /// * `raw_tx` - raw bytes of the transaction
    fn _execute_refund(refund_id: H256, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let request = Self::get_open_refund_request_from_id(&refund_id)?;

        // verify the payment
        let amount: usize = request
            .amount_wrapped
            .try_into()
            .map_err(|_e| Error::<T>::TryIntoIntError)?;

        // check the transaction inclusion and validity
        ext::btc_relay::verify_and_validate_op_return_transaction::<T>(
            merkle_proof,
            raw_tx,
            request.btc_address,
            amount as i64,
            refund_id,
        )?;
        // mint issued tokens corresponding to the fee. Note that this can fail
        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&request.vault, request.fee)?;
        ext::vault_registry::issue_tokens::<T>(&request.vault, request.fee)?;
        ext::treasury::mint::<T>(request.vault.clone(), request.fee);

        // reward vault for this refund by increasing its SLA
        ext::sla::event_update_vault_sla::<T>(&request.vault, ext::sla::VaultEvent::Refund)?;

        // mark the request as completed
        <RefundRequests<T>>::mutate(refund_id, |request| {
            request.completed = true;
        });

        Self::deposit_event(<Event<T>>::ExecuteRefund(
            refund_id,
            request.issuer,
            request.vault,
            Self::u128_to_wrapped(amount as u128)?,
        ));

        Ok(())
    }

    /// Fetch a pre-existing refund request or throw. Completed or cancelled
    /// requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `refund_id` - 256-bit identifier of the refund request
    pub fn get_open_refund_request_from_id(
        refund_id: &H256,
    ) -> Result<RefundRequest<T::AccountId, Wrapped<T>>, DispatchError> {
        ensure!(
            <RefundRequests<T>>::contains_key(*refund_id),
            Error::<T>::RefundIdNotFound
        );
        ensure!(
            !<RefundRequests<T>>::get(*refund_id).completed,
            Error::<T>::RefundCompleted
        );
        Ok(<RefundRequests<T>>::get(*refund_id))
    }

    /// Fetch a pre-existing open or completed refund request or throw.
    /// Cancelled requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `refund_id` - 256-bit identifier of the refund request
    pub fn get_open_or_completed_refund_request_from_id(
        refund_id: &H256,
    ) -> Result<RefundRequest<T::AccountId, Wrapped<T>>, DispatchError> {
        ensure!(
            <RefundRequests<T>>::contains_key(*refund_id),
            Error::<T>::RefundIdNotFound
        );
        Ok(<RefundRequests<T>>::get(*refund_id))
    }

    /// Fetch all refund requests for the specified account. This function is exposed as RPC.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_refund_requests_for_account(
        account_id: T::AccountId,
    ) -> Vec<(H256, RefundRequest<T::AccountId, Wrapped<T>>)> {
        <RefundRequests<T>>::iter()
            .filter(|(_, request)| request.issuer == account_id)
            .collect::<Vec<_>>()
    }

    /// Return the refund request corresponding to the specified issue ID, or return an error. This function is exposed
    /// as RPC.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - The ID of an issue request
    pub fn get_refund_requests_by_issue_id(issue_id: H256) -> Option<(H256, RefundRequest<T::AccountId, Wrapped<T>>)> {
        <RefundRequests<T>>::iter().find(|(_, request)| request.issue_id == issue_id)
    }

    /// Fetch all refund requests for the specified vault. This function is exposed as RPC.
    ///
    /// # Arguments
    ///
    /// * `account_id` - vault account id
    pub fn get_refund_requests_for_vault(
        account_id: T::AccountId,
    ) -> Vec<(H256, RefundRequest<T::AccountId, Wrapped<T>>)> {
        <RefundRequests<T>>::iter()
            .filter(|(_, request)| request.vault == account_id)
            .collect::<Vec<_>>()
    }

    fn u128_to_wrapped(x: u128) -> Result<Wrapped<T>, DispatchError> {
        TryInto::<Wrapped<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        ArithmeticUnderflow,
        NoRefundFoundForIssueId,
        RefundIdNotFound,
        RefundCompleted,
        TryIntoIntError,
        UnauthorizedVault
    }
}
