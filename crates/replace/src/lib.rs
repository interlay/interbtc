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
    old_vault: AccountId,
    open_time: BlockNumber,
    amount: PolkaBTC,
    griefing_collateral: DOT,
    new_vault: Option<AccountId>,
    collateral: DOT,
    accept_time: Option<BlockNumber>,
    btc_address: H160,
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Replace {
        ReplaceGriefingCollateral: DOT<T>;
        ReplacePeriod: T::BlockNumber;
        ReplaceRequests: map hasher(blake2_128_concat) H256 => Option<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        RequestReplace(AccountId, PolkaBTC, BlockNumber, H256),
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

        /// Request the replacement of a new vault ownership
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of DOT
        fn request_replace(origin, amount: PolkaBTC<T>, timeout: T::BlockNumber, griefing_collateral: DOT<T>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_request_replace(requester, amount, timeout, griefing_collateral)?;
            Ok(())
        }

        /// Withdraw a request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn withdraw_replace_request(origin, vault_id: T::AccountId, replace_id: H256)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_withdraw_replace_request(vault_id, replace_id)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn accept_replace(origin, new_vault_id: T::AccountId, replace_id: H256, collateral: DOT<T>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_accept_replace(new_vault_id, replace_id, collateral)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn auction_replace(origin, old_vault_id: T::AccountId, new_vault_id: T::AccountId, btc_amount: PolkaBTC<T>, collateral: DOT<T>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_auction_replace(old_vault_id, new_vault_id, btc_amount, collateral)?;
            Ok(())
        }

        fn execute_replace(origin, new_vault_id: T::AccountId, replace_id: H256, tx_id: H256Le, tx_block_height: u32, tx_index: H256, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_execute_replace(new_vault_id, replace_id, tx_id, tx_block_height, tx_index, merkle_proof, raw_tx)?;
            Ok(())
        }

        fn cancel_replace(origin, new_vault_id: T::AccountId, replace_id: H256) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_cancel_replace(new_vault_id, replace_id)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    fn _request_replace(
        vault_id: T::AccountId,
        amount: PolkaBTC<T>,
        timeout: T::BlockNumber,
        griefing_collateral: DOT<T>,
    ) -> Result<H256, Error> {
        // check preconditions
        // check amount is non zero
        let zero: PolkaBTC<T> = 0u32.into();
        if amount == zero {
            return Err(Error::InvalidAmount);
        }
        // check timeout
        let zero: T::BlockNumber = 0.into();
        if timeout == zero {
            return Err(Error::InvalidTimeout);
        }
        // check vault exists
        let vault = <vault_registry::Module<T>>::get_vault_from_id(vault_id.clone())?;
        // check vault is not banned
        let height = <system::Module<T>>::block_number();
        if vault.is_banned(height) {
            return Err(Error::VaultBanned);
        }

        // check sufficient griefing amount
        ensure!(
            griefing_collateral >= <ReplaceGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );

        let replace = Replace {
            old_vault: vault_id.clone(),
            open_time: height,
            amount,
            griefing_collateral,
            new_vault: None,
            collateral: vault.collateral,
            accept_time: None,
            btc_address: vault.btc_address,
        };
        let mut hasher = Sha256::default();
        // TODO: nonce from security module
        // TODO: test if this is correct hash input
        hasher.input(replace.encode());

        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        let key = H256(result);

        //TODO(jaupe) should we store timeout period?
        //TODO(jaupe) is the collateral value correct?
        Self::insert_replace_request(key, replace);

        Self::deposit_event(<Event<T>>::RequestReplace(vault_id, amount, timeout, key));
        Ok(key)
    }

    fn _withdraw_replace_request(vault_id: T::AccountId, request_id: H256) -> Result<(), Error> {
        // check vault exists
        let _ = <vault_registry::Module<T>>::get_vault_from_id(vault_id.clone())?;
        let _req = Self::get_replace_request(request_id)?;
        Ok(())
    }

    fn _accept_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        _collateral: DOT<T>,
    ) -> Result<(), Error> {
        // check new vault exists
        let _ = <vault_registry::Module<T>>::get_vault_from_id(new_vault_id.clone())?;
        let _req = Self::get_replace_request(replace_id)?;
        Ok(())
    }

    fn _auction_replace(
        _old_vault_id: T::AccountId,
        _new_vault_id: T::AccountId,
        _btc_amount: PolkaBTC<T>,
        _collateral: DOT<T>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn _execute_replace(
        _new_vault: T::AccountId,
        _replace_id: H256,
        _tx_id: H256Le,
        _tx_block_height: u32,
        _tx_index: H256,
        _merkle_proof: Vec<u8>,
        _raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn _cancel_replace(_new_vault_id: T::AccountId, _replace_id: H256) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_replace_request(
        id: H256,
    ) -> Result<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        <ReplaceRequests<T>>::get(id).ok_or(Error::InvalidReplaceID)
    }

    fn insert_replace_request(
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
}
