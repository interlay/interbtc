//! # PolkaBTC Replace Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/replace.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

#[cfg(test)]
extern crate mocktopus;

use frame_support::transactional;
use frame_support::weights::Weight;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use frame_system::{ensure_root, ensure_signed};
#[cfg(test)]
use mocktopus::macros::mockable;
use primitive_types::H256;
use sp_runtime::traits::CheckedSub;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

use bitcoin::types::H256Le;
use btc_relay::BtcAddress;

#[doc(inline)]
pub use crate::types::ReplaceRequest;
use crate::types::{PolkaBTC, Version, DOT};

mod ext;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// The replace module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"replacem");

pub trait WeightInfo {
    fn request_replace() -> Weight;
    fn withdraw_replace() -> Weight;
    fn accept_replace() -> Weight;
    fn auction_replace() -> Weight;
    fn execute_replace() -> Weight;
    fn cancel_replace() -> Weight;
    fn set_replace_period() -> Weight;
}

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + vault_registry::Config
    + collateral::Config
    + btc_relay::Config
    + treasury::Config
    + exchange_rate_oracle::Config
    + fee::Config
    + sla::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Replace {
        /// Vaults create replace requests to transfer locked collateral.
        /// This mapping provides access from a unique hash to a `ReplaceRequest`.
        ReplaceRequests: map hasher(blake2_128_concat) H256 => Option<ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>>;

        /// The time difference in number of blocks between when a replace request is created
        /// and required completion time by a vault. The replace period has an upper limit
        /// to prevent griefing of vault collateral.
        ReplacePeriod get(fn replace_period) config(): T::BlockNumber;

        /// The minimum amount of btc that is accepted for replace requests; any lower values would
        /// risk the bitcoin client to reject the payment
        ReplaceBtcDustValue get(fn replace_btc_dust_value) config(): PolkaBTC<T>;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        PolkaBTC = PolkaBTC<T>,
        DOT = DOT<T>,
        BlockNumber = <T as frame_system::Config>::BlockNumber,
    {
        // [replace_id, old_vault_id, amount_btc, griefing_collateral]
        RequestReplace(H256, AccountId, PolkaBTC, DOT),
        // [replace_id, old_vault_id]
        WithdrawReplace(H256, AccountId),
        // [replace_id, old_vault_id, new_vault_id, amount, collateral, btc_address]
        AcceptReplace(H256, AccountId, AccountId, PolkaBTC, DOT, BtcAddress),
        // [replace_id, old_vault_id, new_vault_id]
        ExecuteReplace(H256, AccountId, AccountId),
        AuctionReplace(
            H256,        // replace_id
            AccountId,   // old_vault_id
            AccountId,   // new_vault_id
            PolkaBTC,    // btc_amount
            DOT,         // collateral
            DOT,         // reward
            DOT,         // griefing_collateral
            BlockNumber, // current_height
            BtcAddress,  // btc_address
        ),
        // [replace_id, new_vault_id, old_vault_id, griefing_collateral]
        CancelReplace(H256, AccountId, AccountId, DOT),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            0
        }

        /// Request the replacement of a new vault ownership
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `griefing_collateral` - amount of DOT
        #[weight = <T as Config>::WeightInfo::request_replace()]
        #[transactional]
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
        #[weight = <T as Config>::WeightInfo::withdraw_replace()]
        #[transactional]
        fn withdraw_replace(origin, replace_id: H256)
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
        #[weight = <T as Config>::WeightInfo::accept_replace()]
        #[transactional]
        fn accept_replace(origin, replace_id: H256, collateral: DOT<T>, btc_address: BtcAddress)
            -> DispatchResult
        {
            let new_vault = ensure_signed(origin)?;
            Self::_accept_replace(new_vault, replace_id, collateral, btc_address)?;
            Ok(())
        }

        /// Auction forces vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `old_vault` - the old vault of the replacement request
        /// * `btc_amount` - the btc amount to be transferred over from old to new
        /// * `collateral` - the collateral to be transferred over from old to new
        #[weight = <T as Config>::WeightInfo::auction_replace()]
        #[transactional]
        fn auction_replace(origin, old_vault: T::AccountId, btc_amount: PolkaBTC<T>, collateral: DOT<T>, btc_address: BtcAddress)
            -> DispatchResult
        {
            let new_vault = ensure_signed(origin)?;
            Self::_auction_replace(old_vault, new_vault, btc_amount, collateral, btc_address)?;
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
        #[weight = <T as Config>::WeightInfo::execute_replace()]
        #[transactional]
        fn execute_replace(origin, replace_id: H256, tx_id: H256Le, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_execute_replace(replace_id, tx_id, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// Cancel vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `replace_id` - the ID of the replacement request
        #[weight = <T as Config>::WeightInfo::cancel_replace()]
        #[transactional]
        fn cancel_replace(origin, replace_id: H256) -> DispatchResult {
            let new_vault = ensure_signed(origin)?;
            Self::_cancel_replace(new_vault, replace_id)?;
            Ok(())
        }

        /// Set the default replace period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::set_replace_period()]
        #[transactional]
        fn set_replace_period(origin, period: T::BlockNumber) {
            ensure_root(origin)?;
            <ReplacePeriod<T>>::set(period);
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn _request_replace(
        vault_id: T::AccountId,
        mut amount_btc: PolkaBTC<T>,
        griefing_collateral: DOT<T>,
    ) -> DispatchResult {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // check vault exists
        let vault = ext::vault_registry::get_active_vault_from_id::<T>(&vault_id)?;

        // check vault is not banned
        let height = Self::current_height();
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;

        // check that the amount_btc doesn't exceed the remaining available tokens
        if amount_btc > vault.issued_tokens {
            amount_btc = vault.issued_tokens;
        }
        // check amount_btc is above the minimum
        let dust_value = <ReplaceBtcDustValue<T>>::get();
        ensure!(amount_btc >= dust_value, Error::<T>::AmountBelowDustAmount);

        // If the request is not for the entire BTC holdings, check that the remaining DOT collateral of the Vault is higher than MinimumCollateralVault
        let vault_collateral = ext::collateral::get_collateral_from_account::<T>(&vault_id);
        if amount_btc != vault.issued_tokens {
            let over_threshold =
                ext::vault_registry::is_over_minimum_collateral::<T>(vault_collateral);
            ensure!(over_threshold, Error::<T>::InsufficientCollateral);
        }

        let amount_dot = ext::oracle::btc_to_dots::<T>(amount_btc)?;
        let expected_griefing_collateral =
            ext::fee::get_replace_griefing_collateral::<T>(amount_dot)?;

        // Check that the griefingCollateral is greater or equal to the expected
        ensure!(
            griefing_collateral >= expected_griefing_collateral,
            Error::<T>::InsufficientCollateral
        );

        // Lock the oldVault’s griefing collateral
        ext::collateral::lock_collateral::<T>(vault_id.clone(), griefing_collateral)?;

        // increase to-be-replaced tokens. This will fail if the vault does not have enough tokens available
        ext::vault_registry::force_increase_to_be_replaced_tokens::<T>(
            &vault_id,
            amount_btc.clone(),
        )?;

        // Generate a replaceId by hashing a random seed, a nonce, and the address of the Requester.
        let replace_id = ext::security::get_secure_id::<T>(&vault_id);

        // Create new ReplaceRequest entry:
        let replace = ReplaceRequest {
            old_vault: vault_id.clone(),
            open_time: height,
            amount: amount_btc,
            griefing_collateral,
            new_vault: None,
            collateral: vault_collateral,
            accept_time: None,
            btc_address: None,
            completed: false,
            cancelled: false,
        };
        Self::insert_replace_request(replace_id, replace);

        // Emit RequestReplace event
        Self::deposit_event(<Event<T>>::RequestReplace(
            replace_id,
            vault_id,
            amount_btc,
            griefing_collateral,
        ));
        Ok(())
    }

    fn _withdraw_replace_request(
        vault_id: T::AccountId,
        request_id: H256,
    ) -> Result<(), DispatchError> {
        // check vault exists
        // Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_open_replace_request(&request_id)?;

        // Check that caller of the function is indeed the to-be-replaced Vault as specified in the ReplaceRequest. Return ERR_UNAUTHORIZED error if this check fails.
        let _vault = ext::vault_registry::get_active_vault_from_id::<T>(&vault_id)?;
        ensure!(vault_id == replace.old_vault, Error::<T>::UnauthorizedVault);

        // Check that the collateral rate of the vault is not under the AuctionCollateralThreshold as defined in the VaultRegistry. If it is under the AuctionCollateralThreshold return ERR_UNAUTHORIZED
        ensure!(
            !ext::vault_registry::is_vault_below_auction_threshold::<T>(vault_id.clone())?,
            Error::<T>::VaultOverAuctionThreshold
        );

        // Check that the ReplaceRequest was not yet accepted by another Vault
        if replace.has_new_owner() {
            return Err(Error::<T>::CancelAcceptedRequest.into());
        }

        // Release the oldVault’s griefing collateral associated with this ReplaceRequests
        ext::collateral::release_collateral::<T>(
            &replace.old_vault,
            replace.griefing_collateral.clone(),
        )?;

        // decrease to-be-replaced tokens, so that the vault is free to use its issued tokens again
        ext::vault_registry::force_decrease_to_be_replaced_tokens::<T>(
            &replace.old_vault,
            replace.amount.clone(),
        )?;

        // Remove the ReplaceRequest from ReplaceRequests
        Self::remove_replace_request(request_id, true);

        // Emit WithdrawReplaceRequest event.
        Self::deposit_event(<Event<T>>::WithdrawReplace(request_id, vault_id));
        Ok(())
    }

    fn _accept_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        collateral: DOT<T>,
        btc_address: BtcAddress,
    ) -> Result<(), DispatchError> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // Retrieve the ReplaceRequest as per the replaceId parameter from ReplaceRequests.
        // Return ERR_REPLACE_ID_NOT_FOUND error if no such ReplaceRequest was found.
        let mut replace = Self::get_open_replace_request(&replace_id)?;

        // Add the new replace address to the vault's wallet,
        // this should also verify that the vault exists
        ext::vault_registry::insert_vault_deposit_address::<T>(&new_vault_id, btc_address.clone())?;

        // Check that the newVault is currently not banned
        let height = Self::current_height();
        ext::vault_registry::ensure_not_banned::<T>(&new_vault_id, height)?;

        // Lock the new-vault's additional collateral
        ext::collateral::lock_collateral::<T>(new_vault_id.clone(), collateral)?;

        // decrease old-vault's to-be-replaced tokens; turn them into to-be-redeemed tokens
        ext::vault_registry::force_decrease_to_be_replaced_tokens::<T>(
            &replace.old_vault,
            replace.amount.clone(),
        )?;
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(
            &replace.old_vault,
            replace.amount.clone(),
        )?;

        // increase to-be-issued tokens - this will fail if there is insufficient collateral
        ext::vault_registry::force_increase_to_be_issued_tokens::<T>(
            &new_vault_id,
            replace.amount,
        )?;

        // Update the ReplaceRequest entry
        replace.add_new_vault(new_vault_id.clone(), height, collateral, btc_address);
        Self::insert_replace_request(replace_id, replace.clone());

        // Emit AcceptReplace event
        Self::deposit_event(<Event<T>>::AcceptReplace(
            replace_id,
            replace.old_vault,
            new_vault_id,
            replace.amount,
            collateral,
            btc_address,
        ));
        Ok(())
    }

    fn _auction_replace(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        btc_amount: PolkaBTC<T>,
        collateral: DOT<T>,
        btc_address: BtcAddress,
    ) -> Result<(), DispatchError> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // Add the new replace address to the vault's wallet,
        // this should also verify that the vault exists
        ext::vault_registry::insert_vault_deposit_address::<T>(&new_vault_id, btc_address.clone())?;

        // Retrieve the oldVault as per the oldVault parameter from Vaults in the VaultRegistry
        let _old_vault = ext::vault_registry::get_active_vault_from_id::<T>(&old_vault_id)?;

        // Check that the oldVault is below the AuctionCollateralThreshold by calculating his current oldVault.issuedTokens and the oldVault.collateral
        ensure!(
            ext::vault_registry::is_vault_below_auction_threshold::<T>(old_vault_id.clone())?,
            Error::<T>::VaultOverAuctionThreshold
        );

        // Lock the new-vault's additional collateral
        ext::collateral::lock_collateral::<T>(new_vault_id.clone(), collateral)?;

        // increase to-be-issued tokens - this will fail if there is insufficient collateral
        ext::vault_registry::force_increase_to_be_issued_tokens::<T>(&new_vault_id, btc_amount)?;

        // claim auctioning fee that is proportional to replace amount
        let dot_amount = ext::oracle::btc_to_dots::<T>(btc_amount)?;
        let reward = ext::fee::get_auction_redeem_fee::<T>(dot_amount)?;
        ext::collateral::slash_collateral::<T>(old_vault_id.clone(), new_vault_id.clone(), reward)?;

        // Call the increaseToBeRedeemedTokens function with the oldVault and the btcAmount
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(&old_vault_id, btc_amount)?;

        // Calculate the (minimum) griefing collateral
        let amount_dot = ext::oracle::btc_to_dots::<T>(btc_amount)?;
        let griefing_collateral = ext::fee::get_replace_griefing_collateral::<T>(amount_dot)?;

        // Create a new ReplaceRequest named replace entry:
        let replace_id = ext::security::get_secure_id::<T>(&new_vault_id);
        let current_height = Self::current_height();
        Self::insert_replace_request(
            replace_id,
            ReplaceRequest {
                new_vault: Some(new_vault_id.clone()),
                old_vault: old_vault_id.clone(),
                open_time: current_height,
                accept_time: Some(current_height),
                amount: btc_amount,
                griefing_collateral,
                btc_address: Some(btc_address),
                collateral: collateral,
                completed: false,
                cancelled: false,
            },
        );

        // Emit AuctionReplace event.
        Self::deposit_event(<Event<T>>::AuctionReplace(
            replace_id,
            old_vault_id,
            new_vault_id,
            btc_amount,
            collateral,
            reward,
            griefing_collateral,
            current_height,
            btc_address,
        ));
        Ok(())
    }

    fn _execute_replace(
        replace_id: H256,
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_open_replace_request(&replace_id)?;

        // NOTE: anyone can call this method provided the proof is correct
        let new_vault_id = if let Some(new_vault_id) = replace.new_vault {
            new_vault_id
        } else {
            // cannot execute without a replacement
            return Err(Error::<T>::NoReplacement.into());
        };
        let old_vault_id = replace.old_vault;

        // only executable before the request has expired
        ensure!(
            !has_request_expired::<T>(replace.open_time, Self::replace_period()),
            Error::<T>::ReplacePeriodExpired
        );

        // Call verifyTransactionInclusion in BTC-Relay, providing txid, txBlockHeight, txIndex, and merkleProof as parameters
        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;

        // Call validateTransaction in BTC-Relay
        let amount = TryInto::<u64>::try_into(replace.amount)
            .map_err(|_e| Error::<T>::TryIntoIntError)? as i64;

        let btc_address = if let Some(btc_address) = replace.btc_address {
            btc_address
        } else {
            // cannot execute without a valid btc address
            return Err(Error::<T>::NoReplacement.into());
        };

        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            amount,
            btc_address,
            Some(replace_id.clone().as_bytes().to_vec()),
        )?;

        // decrease old-vault's issued & to-be-redeemed tokens, and
        // change new-vault's to-be-issued tokens to issued tokens
        ext::vault_registry::replace_tokens::<T>(
            old_vault_id.clone(),
            new_vault_id.clone(),
            replace.amount.clone(),
            replace.collateral.clone(),
        )?;

        // if the old vault has not been liquidated, give it back its griefing collateral
        if !ext::vault_registry::is_vault_liquidated::<T>(&old_vault_id)? {
            ext::collateral::release_collateral::<T>(&old_vault_id, replace.griefing_collateral)?;
        }

        // Emit ExecuteReplace event.
        Self::deposit_event(<Event<T>>::ExecuteReplace(
            replace_id,
            old_vault_id,
            new_vault_id,
        ));

        // Remove replace request
        Self::remove_replace_request(replace_id.clone(), false);
        Ok(())
    }

    fn _cancel_replace(caller: T::AccountId, replace_id: H256) -> Result<(), DispatchError> {
        // Check that Parachain status is RUNNING
        ext::security::ensure_parachain_status_running::<T>()?;

        // Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_open_replace_request(&replace_id)?;

        // only cancellable after the request has expired
        ensure!(
            has_request_expired::<T>(replace.open_time, Self::replace_period()),
            Error::<T>::ReplacePeriodNotExpired
        );

        let new_vault_id = replace.new_vault.ok_or(Error::<T>::NoReplacement)?;

        // only cancellable by new_vault
        ensure!(caller == new_vault_id, Error::<T>::UnauthorizedVault);

        // // Release the newVault's collateral associated with this ReplaceRequests
        // ext::collateral::release_collateral::<T>(new_vault_id.clone(), replace.collateral)?;

        // decrease old-vault's to-be-redeemed tokens, and
        // decrease new-vault's to-be-issued tokens
        ext::vault_registry::cancel_replace_tokens::<T>(
            &replace.old_vault,
            &new_vault_id,
            replace.amount,
        )?;

        // slash old-vault's griefing collateral, but only if it is not liquidated
        // (since the griefing collateral would have been confiscated by the
        // liquidation vault)
        if !ext::vault_registry::is_vault_liquidated::<T>(&replace.old_vault)? {
            // slash to new_vault if it is not liquidated - otherwise slash to liquidation vault
            if !ext::vault_registry::is_vault_liquidated::<T>(&new_vault_id)? {
                ext::collateral::slash_collateral::<T>(
                    replace.old_vault.clone(),
                    new_vault_id.clone(),
                    replace.griefing_collateral,
                )?;
                // ext::collateral::release_collateral::<T>(
                //     &new_vault_id,
                //     replace.griefing_collateral,
                // )?;
            } else {
                ext::collateral::slash_collateral::<T>(
                    replace.old_vault.clone(),
                    ext::vault_registry::get_liquidation_vault::<T>().id,
                    replace.griefing_collateral,
                )?;
            }
        }

        // if the new_vault locked additional collateral especially for this replace,
        // release it if it does not cause him to be undercollateralized
        if !ext::vault_registry::is_vault_liquidated::<T>(&new_vault_id)? {
            let new_collateral = ext::collateral::get_collateral_from_account::<T>(&new_vault_id)
                .checked_sub(&replace.collateral)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let required_collateral =
                ext::vault_registry::get_required_collateral_for_vault::<T>(&new_vault_id)?;

            if new_collateral >= required_collateral {
                // Release the newVault's collateral associated with this ReplaceRequests
                ext::collateral::release_collateral::<T>(&new_vault_id, replace.collateral)?;
            }
        }

        // Remove the ReplaceRequest from ReplaceRequests
        Self::remove_replace_request(replace_id.clone(), true);

        // Emit CancelReplace event.
        Self::deposit_event(<Event<T>>::CancelReplace(
            replace_id,
            new_vault_id,
            replace.old_vault,
            replace.griefing_collateral,
        ));
        Ok(())
    }

    /// Fetch all replace requests from the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_replace_requests_for_old_vault(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
        <ReplaceRequests<T>>::iter()
            .filter(|(_, request)| request.old_vault == account_id)
            .collect::<Vec<_>>()
    }

    /// Fetch all replace requests to the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_replace_requests_for_new_vault(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
        <ReplaceRequests<T>>::iter()
            .filter(|(_, request)| {
                if let Some(vault_id) = &request.new_vault {
                    vault_id == &account_id
                } else {
                    false
                }
            })
            .collect::<Vec<_>>()
    }

    /// Get a replace request by id. Completed or cancelled requests are not returned.
    pub fn get_open_replace_request(
        id: &H256,
    ) -> Result<ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        let request = <ReplaceRequests<T>>::get(id).ok_or(Error::<T>::ReplaceIdNotFound)?;
        // NOTE: temporary workaround until we delete
        ensure!(!request.completed, Error::<T>::ReplaceCompleted);
        ensure!(!request.cancelled, Error::<T>::ReplaceCancelled);
        Ok(request)
    }

    /// Get a open or completed replace request by id. Cancelled requests are not returned.
    pub fn get_open_or_completed_replace_request(
        id: &H256,
    ) -> Result<ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        let request = <ReplaceRequests<T>>::get(id).ok_or(Error::<T>::ReplaceIdNotFound)?;
        ensure!(!request.cancelled, Error::<T>::ReplaceCancelled);
        Ok(request)
    }

    fn insert_replace_request(
        key: H256,
        value: ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <ReplaceRequests<T>>::insert(key, value)
    }

    fn remove_replace_request(key: H256, cancelled: bool) {
        // TODO: delete replace request from storage
        <ReplaceRequests<T>>::mutate(key, |request| {
            if let Some(req) = request {
                req.completed = !cancelled;
                req.cancelled = cancelled;
            }
        });
    }

    fn current_height() -> T::BlockNumber {
        <frame_system::Module<T>>::block_number()
    }
}

fn has_request_expired<T: Config>(opentime: T::BlockNumber, period: T::BlockNumber) -> bool {
    let height = <frame_system::Module<T>>::block_number();
    height > opentime + period
}

decl_error! {
    pub enum Error for Module<T: Config> {
        AmountBelowDustAmount,
        NoReplacement,
        InsufficientCollateral,
        UnauthorizedVault,
        VaultOverAuctionThreshold,
        CancelAcceptedRequest,
        CollateralBelowSecureThreshold,
        ReplacePeriodExpired,
        ReplacePeriodNotExpired,
        ReplaceCompleted,
        ReplaceCancelled,
        ReplaceIdNotFound,
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticUnderflow,
        ArithmeticOverflow,
    }
}
