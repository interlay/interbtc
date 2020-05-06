#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

/// # PolkaBTC Replace implementation
/// The Replace module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/replace.html
mod ext;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

// Substrate
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use primitive_types::H256;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use system::ensure_signed;

use bitcoin::types::H256Le;
use x_core::{Error, UnitResult};

use crate::types::{PolkaBTC, Replace, DOT};

/// The replace module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"replacem");

/// The pallet's configuration trait.
pub trait Trait:
    system::Trait + vault_registry::Trait + collateral::Trait + btc_relay::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
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
        DOT = DOT<T>,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        RequestReplace(AccountId, PolkaBTC, H256),
        WithdrawReplace(AccountId, H256),
        AcceptReplace(AccountId, H256, DOT),
        ExecuteReplace(AccountId, AccountId, H256),
        AuctionReplace(AccountId, AccountId, H256, PolkaBTC, DOT, BlockNumber),
        CancelReplace(AccountId, AccountId, H256),
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
        /// * `griefing_collateral` - amount of DOT
        fn request_replace(origin, amount: PolkaBTC<T>, griefing_collateral: DOT<T>)
            -> DispatchResult
        {
            let old_vault = ensure_signed(origin)?;
            Self::_request_replace(old_vault, amount, griefing_collateral)?;
            Ok(())
        }

        /// Withdraw a request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the old vault
        /// * `replace_id` - the unique identifier of the replace request
        fn withdraw_replace_request(origin, replace_id: H256)
            -> DispatchResult
        {
            let old_vault = ensure_signed(origin)?;
            Self::_withdraw_replace_request(old_vault, replace_id)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - the initiator of the transaction: the new vault
        /// * `replace_id` - the unique identifier for the specific request
        /// * `collateral` - the collateral for replacement
        fn accept_replace(origin, replace_id: H256, collateral: DOT<T>)
            -> DispatchResult
        {
            let new_vault = ensure_signed(origin)?;
            Self::_accept_replace(new_vault, replace_id, collateral)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `old_vault` - the old vault of the replacement request
        /// * `btc_amount` - the btc amount to be transferred over from old to new
        /// * `collateral` - the collateral to be transferred over from old to new
        fn auction_replace(origin, old_vault: T::AccountId, btc_amount: PolkaBTC<T>, collateral: DOT<T>)
            -> DispatchResult
        {
            let new_vault = ensure_signed(origin)?;
            Self::_auction_replace(old_vault, new_vault, btc_amount, collateral)?;
            Ok(())
        }

        /// Execute vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `replace_id` - the ID of the replacement request
        /// * `tx_id` - the backing chain transaction id
        /// * `tx_block_height` - the blocked height of the backing transaction
        /// * 'merkle_proof' - the merkle root of the block
        /// * `raw_tx` - the transaction id in bytes
        fn execute_replace(origin, replace_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let new_vault = ensure_signed(origin)?;
            Self::_execute_replace(new_vault, replace_id, tx_id, tx_block_height, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// Cancel vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `replace_id` - the ID of the replacement request
        fn cancel_replace(origin, replace_id: H256) -> DispatchResult {
            let new_vault = ensure_signed(origin)?;
            Self::_cancel_replace(new_vault, replace_id)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    fn _request_replace(
        vault_id: T::AccountId,
        mut amount: PolkaBTC<T>,
        griefing_collateral: DOT<T>,
    ) -> UnitResult {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // check amount is non zero
        let zero: PolkaBTC<T> = 0u32.into();
        if amount == zero {
            return Err(Error::InvalidAmount);
        }
        // check vault exists
        let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // step 3: check vault is not banned
        let height = Self::current_height();
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;
        // step 4: check that the amount doesn't exceed the remaining available tokens
        if amount > vault.issued_tokens {
            amount = vault.issued_tokens;
        }
        // step 5: If the request is not for the entire BTC holdings, check that the remaining DOT collateral of the Vault is higher than MinimumCollateralVault
        let vault_collateral = ext::collateral::get_collateral_from_account::<T>(vault_id.clone());
        if amount != vault.issued_tokens {
            let over_threshold =
                ext::vault_registry::is_over_minimum_collateral::<T>(vault_collateral);
            ensure!(over_threshold, Error::InsufficientCollateral);
        }
        // step 6: Check that the griefingCollateral is greater or equal ReplaceGriefingCollateral
        ensure!(
            griefing_collateral >= <ReplaceGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );
        // step 7: Lock the oldVault’s griefing collateral
        ext::collateral::lock_collateral::<T>(vault_id.clone(), griefing_collateral)?;
        // step 8: Call the increaseToBeRedeemedTokens function with the oldVault and the btcAmount to ensure that the oldVault’s tokens cannot be redeemed when a replace procedure is happening.
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(&vault_id, amount.clone())?;
        // step 9: Generate a replaceId by hashing a random seed, a nonce, and the address of the Requester.
        let replace_id = ext::security::get_secure_id::<T>(&vault_id);
        // step 10: Create new ReplaceRequest entry:
        let replace = Replace {
            old_vault: vault_id.clone(),
            open_time: height,
            amount,
            griefing_collateral,
            new_vault: None,
            collateral: vault_collateral,
            accept_time: None,
            btc_address: vault.btc_address,
        };
        Self::insert_replace_request(replace_id, replace);
        // step 11: Emit RequestReplace event
        Self::deposit_event(<Event<T>>::RequestReplace(vault_id, amount, replace_id));
        // step 12
        Ok(())
    }

    fn _withdraw_replace_request(vault_id: T::AccountId, request_id: H256) -> Result<(), Error> {
        // check vault exists
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_replace_request(request_id)?;
        // step 2: Check that caller of the function is indeed the to-be-replaced Vault as specified in the ReplaceRequest. Return ERR_UNAUTHORIZED error if this check fails.
        let _vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        ensure!(vault_id == replace.old_vault, Error::UnauthorizedVault);
        // step 3: Check that the collateral rate of the vault is not under the AuctionCollateralThreshold as defined in the VaultRegistry. If it is under the AuctionCollateralThreshold return ERR_UNAUTHORIZED
        ensure!(
            !ext::vault_registry::is_vault_below_auction_threshold::<T>(vault_id.clone())?,
            Error::UnauthorizedVault
        );
        // step 4: Check that the ReplaceRequest was not yet accepted by another Vault
        if replace.has_new_owner() {
            return Err(Error::CancelAcceptedRequest);
        }
        // step 5: Release the oldVault’s griefing collateral associated with this ReplaceRequests
        ext::collateral::release_collateral::<T>(
            replace.old_vault.clone(),
            replace.griefing_collateral.clone(),
        )?;
        // step 6: Call the decreaseToBeRedeemedTokens function in the VaultRegistry to allow the vault to be part of other redeem or replace requests again
        ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(
            replace.old_vault,
            replace.amount.clone(),
        )?;
        // step 7: Remove the ReplaceRequest from ReplaceRequests
        Self::remove_replace_request(request_id);
        // step 8: Emit a WithdrawReplaceRequest(oldVault, replaceId) event.
        Self::deposit_event(<Event<T>>::WithdrawReplace(vault_id, request_id));
        Ok(())
    }

    fn _accept_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        collateral: DOT<T>,
    ) -> Result<(), Error> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from ReplaceRequests. Return ERR_REPLACE_ID_NOT_FOUND error if no such ReplaceRequest was found.
        let mut replace = Self::get_replace_request(replace_id)?;
        // step 2: Retrieve the Vault as per the newVault parameter from Vaults in the VaultRegistry
        let vault = ext::vault_registry::get_vault_from_id::<T>(&new_vault_id)?;
        // step 3: Check that the newVault is currently not banned
        let height = Self::current_height();
        ext::vault_registry::ensure_not_banned::<T>(&new_vault_id, height)?;
        // step 4: Check that the provided collateral exceeds the necessary amount
        let is_below = ext::vault_registry::is_collateral_below_secure_threshold::<T>(
            collateral,
            replace.amount,
        )?;
        ensure!(!is_below, Error::InsufficientCollateral);
        // step 5: Lock the newVault’s collateral by calling lockCollateral
        ext::collateral::lock_collateral::<T>(new_vault_id.clone(), collateral)?;
        // step 6: Update the ReplaceRequest entry
        replace.add_new_vault(new_vault_id.clone(), height, collateral, vault.btc_address);
        Self::insert_replace_request(replace_id, replace);
        // step 7: Emit a AcceptReplace(newVault, replaceId, collateral) event
        Self::deposit_event(<Event<T>>::AcceptReplace(
            new_vault_id,
            replace_id,
            collateral,
        ));
        Ok(())
    }

    fn _auction_replace(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        btc_amount: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> Result<(), Error> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;
        // step 1: Retrieve the newVault as per the newVault parameter from Vaults in the VaultRegistry
        let new_vault = ext::vault_registry::get_vault_from_id::<T>(&new_vault_id)?;
        // step 2: Retrieve the oldVault as per the oldVault parameter from Vaults in the VaultRegistry
        let _old_vault = ext::vault_registry::get_vault_from_id::<T>(&old_vault_id)?;
        // step 3: Check that the oldVault is below the AuctionCollateralThreshold by calculating his current oldVault.issuedTokens and the oldVault.collateral
        ensure!(
            ext::vault_registry::is_vault_below_auction_threshold::<T>(old_vault_id.clone())?,
            Error::InsufficientCollateral
        );
        // step 4: Check that the provided collateral exceeds the necessary amount
        ensure!(
            !ext::vault_registry::is_collateral_below_secure_threshold::<T>(
                collateral, btc_amount
            )?,
            Error::InsufficientCollateral
        );
        // step 5: Lock the newVault’s collateral by calling lockCollateral and providing newVault and collateral as parameters.
        ext::collateral::lock_collateral::<T>(new_vault_id.clone(), collateral)?;
        // step 6: Call the increaseToBeRedeemedTokens function with the oldVault and the btcAmount
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(&old_vault_id, btc_amount)?;
        // step 8: Create a new ReplaceRequest named replace entry:
        let replace_id = ext::security::get_secure_id::<T>(&new_vault_id);
        let current_height = Self::current_height();
        Self::insert_replace_request(
            replace_id,
            Replace {
                new_vault: Some(new_vault_id.clone()),
                old_vault: old_vault_id.clone(),
                open_time: current_height,
                accept_time: Some(current_height),
                amount: btc_amount,
                griefing_collateral: 0.into(),
                btc_address: new_vault.btc_address,
                collateral: collateral,
            },
        );
        // step 9: Emit a AuctionReplace(newVault, replaceId, collateral) event.
        Self::deposit_event(<Event<T>>::AuctionReplace(
            old_vault_id,
            new_vault_id,
            replace_id,
            btc_amount,
            collateral,
            current_height,
        ));
        Ok(())
    }

    fn _execute_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_replace_request(replace_id)?;
        // step 2: Check that the current Parachain block height minus the ReplacePeriod is smaller than the opentime of the ReplaceRequest
        let replace_period = Self::replace_period();
        let current_height = Self::current_height();
        ensure!(
            current_height <= replace.open_time + replace_period,
            Error::ReplacePeriodExpired
        );
        // step 3: Retrieve the Vault as per the newVault parameter from Vaults in the VaultRegistry
        let _new_vault = ext::vault_registry::get_vault_from_id::<T>(&new_vault_id)?;
        // step 4: Call verifyTransactionInclusion in BTC-Relay, providing txid, txBlockHeight, txIndex, and merkleProof as parameters
        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, tx_block_height, merkle_proof)?;
        // step 5: Call validateTransaction in BTC-Relay
        let amount =
            TryInto::<u64>::try_into(replace.amount).map_err(|_e| Error::RuntimeError)? as i64;

        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            amount,
            replace.btc_address.as_bytes().to_vec(),
            replace_id.clone().as_bytes().to_vec(),
        )?;
        // step 6: Call the replaceTokens
        ext::vault_registry::replace_tokens::<T>(
            replace.old_vault.clone(),
            new_vault_id.clone(),
            replace.amount.clone(),
            replace.collateral.clone(),
        )?;
        // step 7: Call the releaseCollateral function to release the oldVaults griefing collateral griefingCollateral
        ext::collateral::release_collateral::<T>(
            replace.old_vault.clone(),
            replace.griefing_collateral,
        )?;
        // step 8: Emit the ExecuteReplace(oldVault, newVault, replaceId) event.
        Self::deposit_event(<Event<T>>::ExecuteReplace(
            replace.old_vault,
            new_vault_id,
            replace_id,
        ));
        // step 9: Remove replace request
        Self::remove_replace_request(replace_id.clone());
        Ok(())
    }

    fn _cancel_replace(new_vault_id: T::AccountId, replace_id: H256) -> Result<(), Error> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_replace_request(replace_id)?;
        // step 2: Check that the current Parachain block height minus the ReplacePeriod is greater than the opentime of the ReplaceRequest
        let current_height = Self::current_height();
        let replace_period = Self::replace_period();
        ensure!(
            current_height > replace.open_time + replace_period,
            Error::ReplacePeriodNotExpired
        );
        // step 4: Transfer the oldVault’s griefing collateral associated with this ReplaceRequests to the newVault by calling slashCollateral
        ext::collateral::slash_collateral::<T>(
            replace.old_vault.clone(),
            new_vault_id.clone(),
            replace.griefing_collateral,
        )?;
        // step 5: Call the decreaseToBeRedeemedTokens function in the VaultRegistry for the oldVault.
        let tokens = replace.amount;
        ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(
            replace.old_vault.clone(),
            tokens,
        )?;
        // step 6: Remove the ReplaceRequest from ReplaceRequests
        Self::remove_replace_request(replace_id.clone());
        // step 7: Emit a CancelReplace(newVault, oldVault, replaceId)
        Self::deposit_event(<Event<T>>::CancelReplace(
            new_vault_id,
            replace.old_vault,
            replace_id,
        ));
        Ok(())
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

    fn replace_period() -> T::BlockNumber {
        <ReplacePeriod<T>>::get()
    }

    fn remove_replace_request(key: H256) {
        <ReplaceRequests<T>>::remove(key)
    }

    #[allow(dead_code)]
    fn set_replace_griefing_collateral(amount: DOT<T>) {
        <ReplaceGriefingCollateral<T>>::set(amount);
    }

    #[allow(dead_code)]
    fn set_replace_period(value: T::BlockNumber) {
        <ReplacePeriod<T>>::set(value);
    }

    fn current_height() -> T::BlockNumber {
        <system::Module<T>>::block_number()
    }
}
