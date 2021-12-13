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
use vault_registry::types::CurrencyId;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

mod ext;
pub mod types;

pub use crate::types::{DefaultRefundRequest, RefundRequest};
use btc_relay::BtcAddress;
use currency::Amount;
#[doc(inline)]
use default_weights::WeightInfo;
use frame_support::{dispatch::DispatchError, ensure, sp_runtime::FixedPointNumber, transactional};
use frame_system::{ensure_root, ensure_signed};
use oracle::OracleKey;
pub use pallet::*;
use sp_core::H256;
use sp_std::{convert::TryInto, vec::Vec};
use types::{BalanceOf, DefaultVaultId, RefundRequestExt};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config + btc_relay::Config + fee::Config<UnsignedInner = BalanceOf<Self>> + vault_registry::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RequestRefund {
            refund_id: H256,
            issuer: T::AccountId,
            amount: BalanceOf<T>,
            vault_id: DefaultVaultId<T>,
            btc_address: BtcAddress,
            issue_id: H256,
            fee: BalanceOf<T>,
            transfer_fee: BalanceOf<T>,
        },
        ExecuteRefund {
            refund_id: H256,
            issuer: T::AccountId,
            vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ArithmeticUnderflow,
        ArithmeticOverflow,
        NoRefundFoundForIssueId,
        RefundIdNotFound,
        RefundCompleted,
        TryIntoIntError,
        UnauthorizedVault,
    }

    /// The minimum amount of btc that is accepted for refund requests (NOTE: too low
    /// values could result in the bitcoin client rejecting the payment)
    #[pallet::storage]
    #[pallet::getter(fn refund_btc_dust_value)]
    pub(super) type RefundBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// the expected size in bytes of the redeem bitcoin transfer
    #[pallet::storage]
    #[pallet::getter(fn refund_transaction_size)]
    pub(super) type RefundTransactionSize<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// This mapping provides access from a unique hash refundId to a Refund struct.
    #[pallet::storage]
    #[pallet::getter(fn refund_requests)]
    pub(super) type RefundRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, DefaultRefundRequest<T>, OptionQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub refund_btc_dust_value: BalanceOf<T>,
        pub refund_transaction_size: u32,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                refund_btc_dust_value: Default::default(),
                refund_transaction_size: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            RefundBtcDustValue::<T>::put(self.refund_btc_dust_value);
            RefundTransactionSize::<T>::put(self.refund_transaction_size);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::execute_refund())]
        #[transactional]
        pub fn execute_refund(
            origin: OriginFor<T>,
            refund_id: H256,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::_execute_refund(refund_id, merkle_proof, raw_tx)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::set_refund_transaction_size())]
        #[transactional]
        pub fn set_refund_transaction_size(origin: OriginFor<T>, size: u32) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            RefundTransactionSize::<T>::put(size);
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
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
        total_amount_btc: &Amount<T>,
        vault_id: DefaultVaultId<T>,
        issuer: T::AccountId,
        btc_address: BtcAddress,
        issue_id: H256,
    ) -> Result<Option<H256>, DispatchError> {
        let vault_reward_fee = ext::fee::get_refund_fee_from_total::<T>(total_amount_btc)?;
        let transfer_fee = Self::get_current_inclusion_fee(vault_id.wrapped_currency())?;

        let net_refund_amount_wrapped = match total_amount_btc
            .checked_sub(&vault_reward_fee)?
            .checked_sub(&transfer_fee)
        {
            // Only refund if the amount is above the dust value
            Ok(x) if x.ge(&Self::get_dust_value(vault_id.wrapped_currency()))? => x,
            _ => return Ok(None),
        };

        let refund_id = ext::security::get_secure_id::<T>(&issuer);

        let request = RefundRequest {
            vault: vault_id,
            fee: vault_reward_fee.amount(),
            transfer_fee_btc: transfer_fee.amount(),
            amount_btc: net_refund_amount_wrapped.amount(),
            issuer,
            btc_address,
            issue_id,
            completed: false,
        };
        Self::insert_refund_request(&refund_id, &request);

        Self::deposit_event(Event::<T>::RequestRefund {
            refund_id: refund_id,
            issuer: request.issuer,
            amount: request.amount_btc,
            vault_id: request.vault,
            btc_address: request.btc_address,
            issue_id: request.issue_id,
            fee: request.fee,
            transfer_fee: request.transfer_fee_btc,
        });

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
    fn _execute_refund(refund_id: H256, raw_merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> Result<(), DispatchError> {
        let request = Self::get_open_refund_request_from_id(&refund_id)?;

        // check the transaction inclusion and validity
        let transaction = ext::btc_relay::parse_transaction::<T>(&raw_tx)?;
        let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(&raw_merkle_proof)?;
        ext::btc_relay::verify_and_validate_op_return_transaction::<T, _>(
            merkle_proof,
            transaction,
            request.btc_address,
            request.amount_btc,
            refund_id,
        )?;
        // mint issued tokens corresponding to the fee. Note that this can fail
        let fee = request.fee();
        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&request.vault, &fee)?;
        ext::vault_registry::issue_tokens::<T>(&request.vault, &fee)?;
        fee.mint_to(&request.vault.account_id)?;

        // mark the request as completed
        <RefundRequests<T>>::mutate_exists(refund_id, |request| {
            *request = request.clone().map(|request| DefaultRefundRequest::<T> {
                completed: true,
                ..request
            });
        });

        Self::deposit_event(Event::<T>::ExecuteRefund {
            refund_id: refund_id,
            issuer: request.issuer,
            vault_id: request.vault,
            amount: request.amount_btc,
            fee: request.fee,
        });

        Ok(())
    }

    /// Fetch a pre-existing refund request or throw. Completed or cancelled
    /// requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `refund_id` - 256-bit identifier of the refund request
    pub fn get_open_refund_request_from_id(refund_id: &H256) -> Result<DefaultRefundRequest<T>, DispatchError> {
        let request = <RefundRequests<T>>::get(*refund_id).ok_or(Error::<T>::RefundIdNotFound)?;

        ensure!(!request.completed, Error::<T>::RefundCompleted);

        Ok(request)
    }

    /// Fetch a pre-existing open or completed refund request or throw.
    /// Cancelled requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `refund_id` - 256-bit identifier of the refund request
    pub fn get_open_or_completed_refund_request_from_id(
        refund_id: &H256,
    ) -> Result<DefaultRefundRequest<T>, DispatchError> {
        <RefundRequests<T>>::get(*refund_id).ok_or(Error::<T>::RefundIdNotFound.into())
    }

    /// Fetch all refund requests for the specified account. This function is exposed as RPC.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_refund_requests_for_account(account_id: T::AccountId) -> Vec<H256> {
        <RefundRequests<T>>::iter()
            .filter(|(_, request)| request.issuer == account_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    /// get current inclusion fee based on the expected number of bytes in the transaction, and
    /// the inclusion fee rate reported by the oracle
    pub fn get_current_inclusion_fee(wrapped_currency: CurrencyId<T>) -> Result<Amount<T>, DispatchError> {
        let size: u32 = Self::refund_transaction_size();
        let satoshi_per_bytes = ext::oracle::get_price::<T>(OracleKey::FeeEstimation)?;

        let fee = satoshi_per_bytes
            .checked_mul_int(size)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let amount = fee.try_into().map_err(|_| Error::<T>::TryIntoIntError)?;
        Ok(Amount::new(amount, wrapped_currency))
    }

    /// Return the refund request corresponding to the specified issue ID, or return an error. This function is exposed
    /// as RPC.
    ///
    /// # Arguments
    ///
    /// * `issue_id` - The ID of an issue request
    pub fn get_refund_requests_by_issue_id(issue_id: H256) -> Option<H256> {
        <RefundRequests<T>>::iter()
            .find(|(_, request)| request.issue_id == issue_id)
            .map(|(key, _)| key)
    }

    /// Fetch all refund requests for the specified vault. This function is exposed as RPC.
    ///
    /// # Arguments
    ///
    /// * `account_id` - vault account id
    pub fn get_refund_requests_for_vault(vault_id: T::AccountId) -> Vec<H256> {
        <RefundRequests<T>>::iter()
            .filter(|(_, request)| request.vault.account_id == vault_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    fn insert_refund_request(key: &H256, value: &DefaultRefundRequest<T>) {
        <RefundRequests<T>>::insert(key, value)
    }

    fn get_dust_value(currency_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(<RefundBtcDustValue<T>>::get(), currency_id)
    }
}
