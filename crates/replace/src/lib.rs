//! # Replace Pallet
//! Based on the [specification](https://spec.interlay.io/spec/replace.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
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

use crate::types::{BalanceOf, ReplaceRequestExt, Version};
pub use crate::types::{DefaultReplaceRequest, ReplaceRequest, ReplaceRequestStatus};
use btc_relay::BtcAddress;
use currency::Amount;
pub use default_weights::WeightInfo;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    transactional,
};
use frame_system::{ensure_root, ensure_signed};
use sp_core::H256;
use sp_std::vec::Vec;
use types::DefaultVaultId;
use vault_registry::{types::CurrencyId, CurrencySource};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use primitives::VaultId;
    use vault_registry::types::DefaultVaultCurrencyPair;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + vault_registry::Config
        + btc_relay::Config
        + oracle::Config
        + fee::Config
        + nomination::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RequestReplace {
            old_vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            griefing_collateral: BalanceOf<T>,
        },
        WithdrawReplace {
            old_vault_id: DefaultVaultId<T>,
            withdrawn_tokens: BalanceOf<T>,
            withdrawn_griefing_collateral: BalanceOf<T>,
        },
        AcceptReplace {
            replace_id: H256,
            old_vault_id: DefaultVaultId<T>,
            new_vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            collateral: BalanceOf<T>,
            btc_address: BtcAddress,
        },
        ExecuteReplace {
            replace_id: H256,
            old_vault_id: DefaultVaultId<T>,
            new_vault_id: DefaultVaultId<T>,
        },
        CancelReplace {
            replace_id: H256,
            new_vault_id: DefaultVaultId<T>,
            old_vault_id: DefaultVaultId<T>,
            griefing_collateral: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Replace requires non-zero increase.
        ReplaceAmountZero,
        /// Replace amount is too small.
        AmountBelowDustAmount,
        /// No replace request found.
        NoPendingRequest,
        /// Unexpected vault account.
        UnauthorizedVault,
        /// Cannot replace self.
        ReplaceSelfNotAllowed,
        /// Cannot replace with nominated collateral.
        VaultHasEnabledNomination,
        /// Replace request has not expired.
        ReplacePeriodNotExpired,
        /// Replace request already completed.
        ReplaceCompleted,
        /// Replace request already cancelled.
        ReplaceCancelled,
        /// Replace request not found.
        ReplaceIdNotFound,
        /// Vault cannot replace different currency.
        InvalidWrappedCurrency,
    }

    /// Vaults create replace requests to transfer locked collateral.
    /// This mapping provides access from a unique hash to a `ReplaceRequest`.
    #[pallet::storage]
    pub(super) type ReplaceRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, DefaultReplaceRequest<T>, OptionQuery>;

    /// The time difference in number of blocks between when a replace request is created
    /// and required completion time by a vault. The replace period has an upper limit
    /// to prevent griefing of vault collateral.
    #[pallet::storage]
    #[pallet::getter(fn replace_period)]
    pub(super) type ReplacePeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// The minimum amount of btc that is accepted for replace requests; any lower values would
    /// risk the bitcoin client to reject the payment
    #[pallet::storage]
    #[pallet::getter(fn replace_btc_dust_value)]
    pub(super) type ReplaceBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::type_value]
    pub(super) fn DefaultForStorageVersion() -> Version {
        Version::V0
    }

    /// Build storage at V1 (requires default 0).
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(super) type StorageVersion<T: Config> = StorageValue<_, Version, ValueQuery, DefaultForStorageVersion>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub replace_period: T::BlockNumber,
        pub replace_btc_dust_value: BalanceOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                replace_period: Default::default(),
                replace_btc_dust_value: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            ReplacePeriod::<T>::put(self.replace_period);
            ReplaceBtcDustValue::<T>::put(self.replace_btc_dust_value);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request the replacement of a new vault ownership
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of issued tokens
        /// * `griefing_collateral` - amount of collateral
        #[pallet::weight(<T as Config>::WeightInfo::request_replace())]
        #[transactional]
        pub fn request_replace(
            origin: OriginFor<T>,
            currency_pair: DefaultVaultCurrencyPair<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let old_vault = VaultId::new(ensure_signed(origin)?, currency_pair.collateral, currency_pair.wrapped);
            Self::_request_replace(old_vault, amount)?;
            Ok(().into())
        }

        /// Withdraw a request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the old vault
        #[pallet::weight(<T as Config>::WeightInfo::withdraw_replace())]
        #[transactional]
        pub fn withdraw_replace(
            origin: OriginFor<T>,
            currency_pair: DefaultVaultCurrencyPair<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let old_vault = VaultId::new(ensure_signed(origin)?, currency_pair.collateral, currency_pair.wrapped);
            Self::_withdraw_replace_request(old_vault, amount)?;
            Ok(().into())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - the initiator of the transaction: the new vault
        /// * `old_vault` - id of the old vault that we are (possibly partially) replacing
        /// * `collateral` - the collateral for replacement
        /// * `btc_address` - the address that old-vault should transfer the btc to
        #[pallet::weight(<T as Config>::WeightInfo::accept_replace())]
        #[transactional]
        pub fn accept_replace(
            origin: OriginFor<T>,
            currency_pair: DefaultVaultCurrencyPair<T>,
            old_vault: DefaultVaultId<T>,
            #[pallet::compact] amount_btc: BalanceOf<T>,
            #[pallet::compact] collateral: BalanceOf<T>,
            btc_address: BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let new_vault = VaultId::new(ensure_signed(origin)?, currency_pair.collateral, currency_pair.wrapped);
            Self::_accept_replace(old_vault, new_vault, amount_btc, collateral, btc_address)?;
            Ok(().into())
        }

        /// Execute vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `replace_id` - the ID of the replacement request
        /// * 'merkle_proof' - the merkle root of the block
        /// * `raw_tx` - the transaction id in bytes
        #[pallet::weight(<T as Config>::WeightInfo::execute_replace())]
        #[transactional]
        pub fn execute_replace(
            origin: OriginFor<T>,
            replace_id: H256,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;
            Self::_execute_replace(replace_id, merkle_proof, raw_tx)?;
            Ok(().into())
        }

        /// Cancel vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction: the new vault
        /// * `replace_id` - the ID of the replacement request
        #[pallet::weight(<T as Config>::WeightInfo::cancel_replace())]
        #[transactional]
        pub fn cancel_replace(origin: OriginFor<T>, replace_id: H256) -> DispatchResultWithPostInfo {
            let new_vault = ensure_signed(origin)?;
            Self::_cancel_replace(new_vault, replace_id)?;
            Ok(().into())
        }

        /// Set the default replace period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[pallet::weight(<T as Config>::WeightInfo::set_replace_period())]
        #[transactional]
        pub fn set_replace_period(origin: OriginFor<T>, period: T::BlockNumber) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <ReplacePeriod<T>>::set(period);
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn _request_replace(vault_id: DefaultVaultId<T>, amount_btc: BalanceOf<T>) -> DispatchResult {
        // check vault is not banned
        ext::vault_registry::ensure_not_banned::<T>(&vault_id)?;

        let amount_btc = Amount::new(amount_btc, vault_id.wrapped_currency());

        ensure!(
            !ext::nomination::is_nominatable::<T>(&vault_id)?,
            Error::<T>::VaultHasEnabledNomination
        );

        let requestable_tokens = ext::vault_registry::requestable_to_be_replaced_tokens::<T>(&vault_id)?;
        let to_be_replaced_increase = amount_btc.min(&requestable_tokens)?;

        ensure!(!to_be_replaced_increase.is_zero(), Error::<T>::ReplaceAmountZero);

        // increase to-be-replaced tokens. This will fail if the vault does not have enough tokens available
        let total_to_be_replaced =
            ext::vault_registry::try_increase_to_be_replaced_tokens::<T>(&vault_id, &to_be_replaced_increase)?;

        // check that total-to-be-replaced is above the minimum
        ensure!(
            total_to_be_replaced.ge(&Self::dust_value(vault_id.wrapped_currency()))?,
            Error::<T>::AmountBelowDustAmount
        );

        // get the griefing collateral increase
        let griefing_collateral = ext::fee::get_replace_griefing_collateral::<T>(
            &to_be_replaced_increase.convert_to(T::GetGriefingCollateralCurrencyId::get())?,
        )?;

        // Lock the oldVault’s griefing collateral
        ext::vault_registry::transfer_funds(
            CurrencySource::FreeBalance(vault_id.account_id.clone()),
            CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
            &griefing_collateral,
        )?;

        // Emit RequestReplace event
        Self::deposit_event(Event::<T>::RequestReplace {
            old_vault_id: vault_id,
            amount: to_be_replaced_increase.amount(),
            griefing_collateral: griefing_collateral.amount(),
        });
        Ok(())
    }

    fn _withdraw_replace_request(vault_id: DefaultVaultId<T>, amount: BalanceOf<T>) -> Result<(), DispatchError> {
        let amount = Amount::new(amount, vault_id.wrapped_currency());
        // decrease to-be-replaced tokens, so that the vault is free to use its issued tokens again.
        let (withdrawn_tokens, to_withdraw_collateral) =
            ext::vault_registry::decrease_to_be_replaced_tokens::<T>(&vault_id, &amount)?;

        // release the used collateral
        ext::vault_registry::transfer_funds(
            CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
            CurrencySource::FreeBalance(vault_id.account_id.clone()),
            &to_withdraw_collateral,
        )?;

        if withdrawn_tokens.is_zero() {
            return Err(Error::<T>::NoPendingRequest.into());
        }

        // Emit WithdrawReplaceRequest event.
        Self::deposit_event(Event::<T>::WithdrawReplace {
            old_vault_id: vault_id,
            withdrawn_tokens: withdrawn_tokens.amount(),
            withdrawn_griefing_collateral: to_withdraw_collateral.amount(),
        });
        Ok(())
    }

    fn accept_replace_tokens(
        old_vault_id: &DefaultVaultId<T>,
        new_vault_id: &DefaultVaultId<T>,
        redeemable_tokens: &Amount<T>,
    ) -> DispatchResult {
        // increase old-vault's to-be-redeemed tokens - this should never fail
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<T>(old_vault_id, redeemable_tokens)?;

        // increase new-vault's to-be-issued tokens - this will fail if there is insufficient collateral
        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(new_vault_id, redeemable_tokens)?;

        Ok(())
    }

    fn _accept_replace(
        old_vault_id: DefaultVaultId<T>,
        new_vault_id: DefaultVaultId<T>,
        amount_btc: BalanceOf<T>,
        collateral: BalanceOf<T>,
        btc_address: BtcAddress,
    ) -> Result<(), DispatchError> {
        let new_vault_currency_id = new_vault_id.collateral_currency();
        let amount_btc = Amount::new(amount_btc, old_vault_id.wrapped_currency());
        let collateral = Amount::new(collateral, new_vault_currency_id);

        // don't allow vaults to replace themselves
        ensure!(old_vault_id != new_vault_id, Error::<T>::ReplaceSelfNotAllowed);

        // probably this check is not strictly required, but it's better to give an
        // explicit error rather than insufficient balance
        ensure!(
            old_vault_id.wrapped_currency() == new_vault_id.wrapped_currency(),
            Error::<T>::InvalidWrappedCurrency
        );

        // Check that new vault is not currently banned
        ext::vault_registry::ensure_not_banned::<T>(&new_vault_id)?;

        // Add the new replace address to the vault's wallet,
        // this should also verify that the vault exists
        ext::vault_registry::insert_vault_deposit_address::<T>(new_vault_id.clone(), btc_address)?;

        // decrease old-vault's to-be-replaced tokens
        let (redeemable_tokens, griefing_collateral) =
            ext::vault_registry::decrease_to_be_replaced_tokens::<T>(&old_vault_id, &amount_btc)?;

        // check amount_btc is above the minimum
        ensure!(
            redeemable_tokens.ge(&Self::dust_value(old_vault_id.wrapped_currency()))?,
            Error::<T>::AmountBelowDustAmount
        );

        // Calculate and lock the new-vault's additional collateral
        let actual_new_vault_collateral =
            ext::vault_registry::calculate_collateral::<T>(&collateral, &redeemable_tokens, &amount_btc)?;

        ext::vault_registry::try_deposit_collateral::<T>(&new_vault_id, &actual_new_vault_collateral)?;

        Self::accept_replace_tokens(&old_vault_id, &new_vault_id, &redeemable_tokens)?;

        ext::vault_registry::transfer_funds(
            CurrencySource::AvailableReplaceCollateral(old_vault_id.clone()),
            CurrencySource::ActiveReplaceCollateral(old_vault_id.clone()),
            &griefing_collateral,
        )?;

        let replace_id = ext::security::get_secure_id::<T>(&old_vault_id.account_id);

        let replace = ReplaceRequest {
            old_vault: old_vault_id,
            new_vault: new_vault_id,
            accept_time: ext::security::active_block_number::<T>(),
            collateral: actual_new_vault_collateral.amount(),
            btc_address,
            griefing_collateral: griefing_collateral.amount(),
            amount: redeemable_tokens.amount(),
            period: Self::replace_period(),
            btc_height: ext::btc_relay::get_best_block_height::<T>(),
            status: ReplaceRequestStatus::Pending,
        };

        Self::insert_replace_request(&replace_id, &replace);

        // Emit AcceptReplace event
        Self::deposit_event(Event::<T>::AcceptReplace {
            replace_id: replace_id,
            old_vault_id: replace.old_vault,
            new_vault_id: replace.new_vault,
            amount: replace.amount,
            collateral: replace.collateral,
            btc_address: replace.btc_address,
        });

        Ok(())
    }

    fn _execute_replace(replace_id: H256, raw_merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
        // retrieve the replace request using the id parameter
        // we can still execute cancelled requests
        let replace = Self::get_open_or_cancelled_replace_request(&replace_id)?;

        let griefing_collateral: Amount<T> = replace.griefing_collateral();
        let amount = replace.amount();
        let collateral = replace.collateral()?;

        // NOTE: anyone can call this method provided the proof is correct
        let new_vault_id = replace.new_vault;
        let old_vault_id = replace.old_vault;

        // check the transaction inclusion and validity
        let transaction = ext::btc_relay::parse_transaction::<T>(&raw_tx)?;
        let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(&raw_merkle_proof)?;
        ext::btc_relay::verify_and_validate_op_return_transaction::<T, _>(
            merkle_proof,
            transaction,
            replace.btc_address,
            replace.amount,
            replace_id,
        )?;

        // only return griefing collateral if not already slashed
        let collateral = match replace.status {
            ReplaceRequestStatus::Pending => {
                // give old-vault the griefing collateral
                ext::vault_registry::transfer_funds(
                    CurrencySource::ActiveReplaceCollateral(old_vault_id.clone()),
                    CurrencySource::FreeBalance(old_vault_id.account_id.clone()),
                    &griefing_collateral,
                )?;
                // NOTE: this is just the additional collateral already locked on accept
                // it is only used in the ReplaceTokens event
                collateral
            }
            ReplaceRequestStatus::Cancelled => {
                // we need to re-accept first, this will check that the vault is over the secure threshold
                Self::accept_replace_tokens(&old_vault_id, &new_vault_id, &amount)?;
                // no additional collateral locked for this
                Amount::zero(collateral.currency())
            }
            ReplaceRequestStatus::Completed => {
                // we never enter this branch as completed requests are filtered
                return Err(Error::<T>::ReplaceCompleted.into());
            }
        };

        // decrease old-vault's issued & to-be-redeemed tokens, and
        // change new-vault's to-be-issued tokens to issued tokens
        ext::vault_registry::replace_tokens::<T>(&old_vault_id, &new_vault_id, &amount, &collateral)?;

        // Emit ExecuteReplace event.
        Self::deposit_event(Event::<T>::ExecuteReplace {
            replace_id: replace_id,
            old_vault_id: old_vault_id,
            new_vault_id: new_vault_id,
        });

        // Remove replace request
        Self::set_replace_status(&replace_id, ReplaceRequestStatus::Completed);
        Ok(())
    }

    fn _cancel_replace(caller: T::AccountId, replace_id: H256) -> Result<(), DispatchError> {
        // Retrieve the ReplaceRequest as per the replaceId parameter from Vaults in the VaultRegistry
        let replace = Self::get_open_replace_request(&replace_id)?;

        let griefing_collateral: Amount<T> = replace.griefing_collateral();
        let amount = replace.amount();
        let collateral = replace.collateral()?;

        // only cancellable after the request has expired
        ensure!(
            ext::btc_relay::has_request_expired::<T>(
                replace.accept_time,
                replace.btc_height,
                Self::replace_period().max(replace.period)
            )?,
            Error::<T>::ReplacePeriodNotExpired
        );

        let new_vault_id = replace.new_vault;

        // only cancellable by new_vault
        ensure!(caller == new_vault_id.account_id, Error::<T>::UnauthorizedVault);

        // decrease old-vault's to-be-redeemed tokens, and
        // decrease new-vault's to-be-issued tokens
        ext::vault_registry::cancel_replace_tokens::<T>(&replace.old_vault, &new_vault_id, &amount)?;

        // slash old-vault's griefing collateral
        ext::vault_registry::transfer_funds::<T>(
            CurrencySource::ActiveReplaceCollateral(replace.old_vault.clone()),
            CurrencySource::FreeBalance(new_vault_id.account_id.clone()),
            &griefing_collateral,
        )?;

        // if the new_vault locked additional collateral especially for this replace,
        // release it if it does not cause them to be undercollateralized
        if !ext::vault_registry::is_vault_liquidated::<T>(&new_vault_id)?
            && ext::vault_registry::is_allowed_to_withdraw_collateral::<T>(&new_vault_id, &collateral)?
        {
            ext::vault_registry::force_withdraw_collateral::<T>(&new_vault_id, &collateral)?;
        }

        // Remove the ReplaceRequest from ReplaceRequests
        Self::set_replace_status(&replace_id, ReplaceRequestStatus::Cancelled);

        // Emit CancelReplace event.
        Self::deposit_event(Event::<T>::CancelReplace {
            replace_id: replace_id,
            new_vault_id: new_vault_id,
            old_vault_id: replace.old_vault,
            griefing_collateral: replace.griefing_collateral,
        });
        Ok(())
    }

    /// Fetch all replace requests from the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_replace_requests_for_old_vault(vault_id: T::AccountId) -> Vec<H256> {
        <ReplaceRequests<T>>::iter()
            .filter(|(_, request)| request.old_vault.account_id == vault_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    /// Fetch all replace requests to the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_replace_requests_for_new_vault(vault_id: T::AccountId) -> Vec<H256> {
        <ReplaceRequests<T>>::iter()
            .filter(|(_, request)| request.new_vault.account_id == vault_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    /// Get a replace request by id. Completed or cancelled requests are not returned.
    pub fn get_open_replace_request(replace_id: &H256) -> Result<DefaultReplaceRequest<T>, DispatchError> {
        let request = ReplaceRequests::<T>::try_get(replace_id).or(Err(Error::<T>::ReplaceIdNotFound))?;

        // NOTE: temporary workaround until we delete
        match request.status {
            ReplaceRequestStatus::Pending => Ok(request),
            ReplaceRequestStatus::Completed => Err(Error::<T>::ReplaceCompleted.into()),
            ReplaceRequestStatus::Cancelled => Err(Error::<T>::ReplaceCancelled.into()),
        }
    }

    /// Get a open or completed replace request by id. Cancelled requests are not returned.
    pub fn get_open_or_completed_replace_request(id: &H256) -> Result<DefaultReplaceRequest<T>, DispatchError> {
        let request = <ReplaceRequests<T>>::get(id).ok_or(Error::<T>::ReplaceIdNotFound)?;
        match request.status {
            ReplaceRequestStatus::Pending | ReplaceRequestStatus::Completed => Ok(request),
            ReplaceRequestStatus::Cancelled => Err(Error::<T>::ReplaceCancelled.into()),
        }
    }

    /// Get a open or cancelled replace request by id. Completed requests are not returned.
    pub fn get_open_or_cancelled_replace_request(id: &H256) -> Result<DefaultReplaceRequest<T>, DispatchError> {
        let request = <ReplaceRequests<T>>::get(id).ok_or(Error::<T>::ReplaceIdNotFound)?;
        match request.status {
            ReplaceRequestStatus::Pending | ReplaceRequestStatus::Cancelled => Ok(request),
            ReplaceRequestStatus::Completed => Err(Error::<T>::ReplaceCompleted.into()),
        }
    }

    fn insert_replace_request(key: &H256, value: &DefaultReplaceRequest<T>) {
        <ReplaceRequests<T>>::insert(key, value)
    }

    fn set_replace_status(key: &H256, status: ReplaceRequestStatus) {
        <ReplaceRequests<T>>::mutate_exists(key, |request| {
            *request = request.clone().map(|request| DefaultReplaceRequest::<T> {
                status: status.clone(),
                ..request
            });
        });
    }

    pub fn dust_value(currency_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(ReplaceBtcDustValue::<T>::get(), currency_id)
    }
}
