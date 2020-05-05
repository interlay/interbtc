#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;

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
use sp_core::H160;
use sp_runtime::ModuleId;
use std::convert::TryInto;
use system::ensure_signed;
use x_core::Error;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;
pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The replace module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"replacem");

/// The pallet's configuration trait.
pub trait Trait:
    system::Trait
    + vault_registry::Trait
    + collateral::Trait
    + btc_relay::Trait
    + treasury::Trait
    + exchange_rate_oracle::Trait
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

impl<AccountId, BlockNumber, PolkaBTC, DOT> Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
    fn add_new_vault(
        &mut self,
        new_vault_id: AccountId,
        accept_time: BlockNumber,
        collateral: DOT,
        btc_address: H160,
    ) {
        self.new_vault = Some(new_vault_id);
        self.accept_time = Some(accept_time);
        self.collateral = collateral;
        self.btc_address = btc_address;
    }

    fn has_new_owner(&self) -> bool {
        self.new_vault.is_some()
    }
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
        RequestReplace(AccountId, PolkaBTC, BlockNumber, H256),
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
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of DOT
        fn request_replace(origin, old_vault: T::AccountId, amount: PolkaBTC<T>, timeout: T::BlockNumber, griefing_collateral: DOT<T>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_request_replace(requester, old_vault, amount, timeout, griefing_collateral)?;
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

        fn execute_replace(origin, new_vault_id: T::AccountId, replace_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_execute_replace(new_vault_id, replace_id, tx_id, tx_block_height, merkle_proof, raw_tx)?;
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
        requester: T::AccountId,
        vault_id: T::AccountId,
        mut amount: PolkaBTC<T>,
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
        let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // step 3: check vault is not banned
        let height = Self::current_height();
        vault.ensure_not_banned(height)?;
        // step 4: check that the amount doesn't exceed the remaining available tokens
        if amount > vault.no_issuable_tokens() {
            amount = vault.no_issuable_tokens();
        }
        // step 5: If the request is not for the entire BTC holdings, check that the remaining DOT collateral of the Vault is higher than MinimumCollateralVault
        let vault_collateral = ext::collateral::get_collateral_from_account::<T>(vault_id.clone());
        let over_threshold = ext::vault_registry::is_over_minimum_collateral::<T>(vault_collateral);
        ensure!(
            amount != vault.no_issuable_tokens() && !over_threshold,
            Error::InsufficientCollateral
        );
        // step 6: Check that the griefingCollateral is greater or equal ReplaceGriefingCollateral
        ensure!(
            griefing_collateral >= <ReplaceGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );
        // step 7: Lock the oldVault’s griefing collateral
        ext::collateral::lock_collateral::<T>(requester.clone(), griefing_collateral)?;
        // step 8: Call the increaseToBeRedeemedTokens function with the oldVault and the btcAmount to ensure that the oldVault’s tokens cannot be redeemed when a replace procedure is happening.
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(&vault_id, amount.clone())?;
        // step 9: Generate a replaceId by hashing a random seed, a nonce, and the address of the Requester.
        let replace_id = ext::security::gen_secure_id::<T>(requester);
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
        // step 11: Emit RequestReplace(vault, btcAmount, timeout, replaceId)
        Self::deposit_event(<Event<T>>::RequestReplace(
            vault_id, amount, timeout, replace_id,
        ));
        // step 12: return replace key
        Ok(replace_id)
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
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from ReplaceRequests. Return ERR_REPLACE_ID_NOT_FOUND error if no such ReplaceRequest was found.
        let mut replace = Self::get_replace_request(replace_id)?;
        // step 2: Retrieve the Vault as per the newVault parameter from Vaults in the VaultRegistry
        let vault = ext::vault_registry::get_vault_from_id::<T>(&new_vault_id)?;
        // step 3: Check that the newVault is currently not banned
        let height = Self::current_height();
        if vault.is_banned(height) {
            return Err(Error::VaultBanned);
        }
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
        let replace_id = ext::security::gen_secure_id::<T>(new_vault_id.clone());
        let height = <system::Module<T>>::block_number();
        Self::insert_replace_request(
            replace_id,
            Replace {
                new_vault: Some(new_vault_id.clone()),
                old_vault: old_vault_id.clone(),
                open_time: height,
                accept_time: Some(height),
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
            height,
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
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_replace_request(replace_id)?;
        // step 2: Check that the current Parachain block height minus the ReplacePeriod is smaller than the opentime of the ReplaceRequest
        let replace_period = Self::replace_period();
        let height = Self::current_height();
        if replace.open_time > height - replace_period {
            return Err(Error::ReplacePeriodExpired);
        }
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
        // step 1: Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_replace_request(replace_id)?;
        // step 2: Check that the current Parachain block height minus the ReplacePeriod is greater than the opentime of the ReplaceRequest
        let current_height = Self::current_height();
        let replace_period = Self::replace_period();
        if current_height - replace_period >= replace.open_time {
            return Err(Error::ReplacePeriodNotExpired);
        }
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
    fn set_issue_griefing_collateral(amount: DOT<T>) {
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
