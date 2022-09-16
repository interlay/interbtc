//! # Issue Pallet
//! Based on the [specification](https://spec.interlay.io/spec/issue.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

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

#[doc(inline)]
pub use crate::types::{DefaultIssueRequest, IssueRequest, IssueRequestStatus};

use crate::types::{BalanceOf, DefaultVaultId, Version};
use btc_relay::{BtcAddress, BtcPublicKey};
use currency::Amount;
use frame_support::{dispatch::DispatchError, ensure, traits::Get, transactional};
use frame_system::{ensure_root, ensure_signed};
pub use pallet::*;
use sp_core::H256;
use sp_runtime::traits::{Convert, Saturating};
use sp_std::vec::Vec;
use types::IssueRequestExt;
use vault_registry::{types::CurrencyId, CurrencySource, VaultStatus};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + vault_registry::Config
        + btc_relay::Config
        + oracle::Config
        + fee::Config<UnsignedInner = BalanceOf<Self>>
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RequestIssue {
            issue_id: H256,
            requester: T::AccountId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            griefing_collateral: BalanceOf<T>,
            vault_id: DefaultVaultId<T>,
            vault_address: BtcAddress,
            vault_public_key: BtcPublicKey,
        },
        IssueAmountChange {
            issue_id: H256,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            confiscated_griefing_collateral: BalanceOf<T>,
        },
        ExecuteIssue {
            issue_id: H256,
            requester: T::AccountId,
            vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        CancelIssue {
            issue_id: H256,
            requester: T::AccountId,
            griefing_collateral: BalanceOf<T>,
        },
        IssuePeriodChange {
            period: T::BlockNumber,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Issue request not found.
        IssueIdNotFound,
        /// Issue request has expired.
        CommitPeriodExpired,
        /// Issue request has not expired.
        TimeNotExpired,
        /// Issue request already completed.
        IssueCompleted,
        /// Issue request already cancelled.
        IssueCancelled,
        /// Vault is not active.
        VaultNotAcceptingNewIssues,
        /// Relay is not initialized.
        WaitingForRelayerInitialization,
        /// Not expected origin.
        InvalidExecutor,
        /// Issue amount is too small.
        AmountBelowDustAmount,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            ensure!(Self::storage_version() == Version::V0, "Wrong storage version.");
            let issue_count_before = crate::IssueRequests::<T>::iter().count() as u32;
            log::info!("issue count before: {:?}", issue_count_before);
            crate::types::v4::IssueCountBefore::<T>::put(issue_count_before);

            Ok(())
        }

        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            crate::types::v4::migrate_v0_to_v4::<T>()
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            ensure!(Self::storage_version() == Version::V4, "Wrong storage version.");
            let issue_count_before = crate::types::v4::IssueCountBefore::<T>::take();
            let issue_count_after = crate::IssueRequests::<T>::iter().count() as u32;
            log::info!("issue count after: {:?}", issue_count_after,);
            ensure!(issue_count_before == issue_count_after, "Not all issues were migrated.");
            Ok(())
        }
    }

    /// Users create issue requests to issue tokens. This mapping provides access
    /// from a unique hash `IssueId` to an `IssueRequest` struct.
    #[pallet::storage]
    #[pallet::getter(fn issue_requests)]
    pub(super) type IssueRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, DefaultIssueRequest<T>, OptionQuery>;

    /// The time difference in number of blocks between an issue request is created
    /// and required completion time by a user. The issue period has an upper limit
    /// to prevent griefing of vault collateral.
    #[pallet::storage]
    #[pallet::getter(fn issue_period)]
    pub(super) type IssuePeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// The minimum amount of btc that is required for issue requests; lower values would
    /// risk the rejection of payment on Bitcoin.
    #[pallet::storage]
    pub(super) type IssueBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
        pub issue_period: T::BlockNumber,
        pub issue_btc_dust_value: BalanceOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                issue_period: Default::default(),
                issue_btc_dust_value: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            IssuePeriod::<T>::put(self.issue_period);
            IssueBtcDustValue::<T>::put(self.issue_btc_dust_value);
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request the issuance of tokens
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of BTC the user wants to convert to issued tokens. Note that the
        /// amount of issued tokens received will be less, because a fee is subtracted.
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of collateral
        #[pallet::weight(<T as Config>::WeightInfo::request_issue())]
        #[transactional]
        pub fn request_issue(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
            vault_id: DefaultVaultId<T>,
        ) -> DispatchResultWithPostInfo {
            let requester = ensure_signed(origin)?;
            Self::_request_issue(requester, amount, vault_id)?;
            Ok(().into())
        }

        /// Finalize the issuance of tokens
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `issue_id` - identifier of issue request as output from request_issue
        /// * `tx_block_height` - block number of collateral chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        #[pallet::weight(<T as Config>::WeightInfo::execute_issue())]
        #[transactional]
        pub fn execute_issue(
            origin: OriginFor<T>,
            issue_id: H256,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let executor = ensure_signed(origin)?;
            Self::_execute_issue(executor, issue_id, merkle_proof, raw_tx)?;
            Ok(().into())
        }

        /// Cancel the issuance of tokens if expired
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `issue_id` - identifier of issue request as output from request_issue
        #[pallet::weight(<T as Config>::WeightInfo::cancel_issue())]
        #[transactional]
        pub fn cancel_issue(origin: OriginFor<T>, issue_id: H256) -> DispatchResultWithPostInfo {
            let requester = ensure_signed(origin)?;
            Self::_cancel_issue(requester, issue_id)?;
            Ok(().into())
        }

        /// Set the default issue period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[pallet::weight(<T as Config>::WeightInfo::set_issue_period())]
        #[transactional]
        pub fn set_issue_period(origin: OriginFor<T>, period: T::BlockNumber) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssuePeriod<T>>::set(period);
            Self::deposit_event(Event::IssuePeriodChange { period });
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    /// Requests CBA issuance, returns unique tracking ID.
    fn _request_issue(
        requester: T::AccountId,
        amount_requested: BalanceOf<T>,
        vault_id: DefaultVaultId<T>,
    ) -> Result<H256, DispatchError> {
        let amount_requested = Amount::new(amount_requested, vault_id.wrapped_currency());

        ensure!(
            ext::btc_relay::is_fully_initialized::<T>()?,
            Error::<T>::WaitingForRelayerInitialization
        );

        let vault = ext::vault_registry::get_active_vault_from_id::<T>(&vault_id)?;

        // ensure that the vault is accepting new issues
        ensure!(
            vault.status == VaultStatus::Active(true),
            Error::<T>::VaultNotAcceptingNewIssues
        );

        // Check that the vault is currently not banned
        ext::vault_registry::ensure_not_banned::<T>(&vault_id)?;

        // calculate griefing collateral based on the total amount of tokens to be issued
        let amount_collateral = amount_requested.convert_to(T::GetGriefingCollateralCurrencyId::get())?;
        let griefing_collateral = Amount::<T>::new(
            ext::fee::get_issue_griefing_collateral::<T>(&amount_collateral)?.amount(),
            T::GetGriefingCollateralCurrencyId::get(),
        );
        griefing_collateral.lock_on(&requester)?;

        // only continue if the payment is above the dust value
        ensure!(
            amount_requested.ge(&Self::issue_btc_dust_value(vault_id.wrapped_currency()))?,
            Error::<T>::AmountBelowDustAmount
        );

        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&vault_id, &amount_requested)?;

        let fee = ext::fee::get_issue_fee::<T>(&amount_requested)?;
        // calculate the amount of tokens that will be transferred to the user upon execution
        let amount_user = amount_requested.checked_sub(&fee)?;

        let issue_id = ext::security::get_secure_id::<T>(&requester);
        let btc_address = ext::vault_registry::register_deposit_address::<T>(&vault_id, issue_id)?;
        let btc_public_key = ext::vault_registry::get_bitcoin_public_key::<T>(&vault_id.account_id)?;

        let request = IssueRequest {
            vault: vault_id,
            opentime: ext::security::active_block_number::<T>(),
            requester,
            btc_address,
            btc_public_key,
            amount: amount_user.amount(),
            fee: fee.amount(),
            griefing_collateral: griefing_collateral.amount(),
            period: Self::issue_period(),
            btc_height: ext::btc_relay::get_best_block_height::<T>(),
            status: IssueRequestStatus::Pending,
        };
        Self::insert_issue_request(&issue_id, &request);

        Self::deposit_event(Event::RequestIssue {
            issue_id,
            requester: request.requester,
            amount: request.amount,
            fee: request.fee,
            griefing_collateral: request.griefing_collateral,
            vault_id: request.vault,
            vault_address: request.btc_address,
            vault_public_key: request.btc_public_key,
        });
        Ok(issue_id)
    }

    /// Completes CBA issuance, removing request from storage and minting token.
    fn _execute_issue(
        executor: T::AccountId,
        issue_id: H256,
        raw_merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        let mut issue = Self::get_issue_request_from_id(&issue_id)?;
        // allow anyone to complete issue request
        let requester = issue.requester.clone();

        let transaction = ext::btc_relay::parse_transaction::<T>(&raw_tx)?;
        let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(&raw_merkle_proof)?;
        let amount_transferred = ext::btc_relay::get_and_verify_issue_payment::<T, BalanceOf<T>>(
            merkle_proof,
            transaction,
            issue.btc_address,
        )?;
        let amount_transferred = Amount::new(amount_transferred, issue.vault.wrapped_currency());

        let expected_total_amount = issue.amount().checked_add(&issue.fee())?;

        match issue.status {
            IssueRequestStatus::Completed => return Err(Error::<T>::IssueCompleted.into()),
            IssueRequestStatus::Cancelled => {
                // if vault is not accepting new issues, we don't allow the execution of cancelled
                // issues, since this would drop the collateralization rate unexpectedly
                ext::vault_registry::ensure_accepting_new_issues::<T>(&issue.vault)?;

                // first try to increase the to-be-issued tokens - if the vault does not
                // have sufficient collateral then this aborts
                ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&issue.vault, &amount_transferred)?;

                if amount_transferred.lt(&expected_total_amount)? {
                    ensure!(requester == executor, Error::<T>::InvalidExecutor);
                }
                if amount_transferred.ne(&expected_total_amount)? {
                    // griefing collateral and to_be_issued already decreased in cancel
                    let slashed = Amount::zero(T::GetGriefingCollateralCurrencyId::get());
                    Self::set_issue_amount(&issue_id, &mut issue, amount_transferred, slashed)?;
                }
            }
            IssueRequestStatus::Pending => {
                let to_release_griefing_collateral = if amount_transferred.lt(&expected_total_amount)? {
                    // only the requester of the issue can execute payments with insufficient amounts
                    ensure!(requester == executor, Error::<T>::InvalidExecutor);
                    Self::decrease_issue_amount(&issue_id, &mut issue, amount_transferred, expected_total_amount)?
                } else {
                    if amount_transferred.gt(&expected_total_amount)?
                        && !ext::vault_registry::is_vault_liquidated::<T>(&issue.vault)?
                    {
                        Self::try_increase_issue_amount(
                            &issue_id,
                            &mut issue,
                            amount_transferred,
                            expected_total_amount,
                        )?;
                    }
                    issue.griefing_collateral()
                };

                to_release_griefing_collateral.unlock_on(&requester)?;
            }
        }

        // issue struct may have been update above; recalculate the total
        let issue_amount = issue.amount();
        let issue_fee = issue.fee();
        let total = issue_amount.checked_add(&issue_fee)?;
        ext::vault_registry::issue_tokens::<T>(&issue.vault, &total)?;

        // mint issued tokens
        issue_amount.mint_to(&requester)?;

        // mint wrapped fees
        issue_fee.mint_to(&ext::fee::fee_pool_account_id::<T>())?;

        // distribute rewards
        ext::fee::distribute_rewards::<T>(&issue_fee)?;

        Self::set_issue_status(issue_id, IssueRequestStatus::Completed);

        Self::deposit_event(Event::ExecuteIssue {
            issue_id,
            requester,
            vault_id: issue.vault,
            amount: total.amount(),
            fee: issue.fee,
        });
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_issue(requester: T::AccountId, issue_id: H256) -> Result<(), DispatchError> {
        let issue = Self::get_pending_issue(&issue_id)?;

        let issue_period = Self::issue_period().max(issue.period);
        let to_be_slashed_collateral =
            if ext::btc_relay::has_request_expired::<T>(issue.opentime, issue.btc_height, issue_period)? {
                // anyone can cancel the issue request once expired
                issue.griefing_collateral()
            } else if issue.requester == requester {
                // slash/release griefing collateral proportionally to the time elapsed
                // NOTE: if global issue period increases requester will get more griefing collateral
                let blocks_elapsed = ext::security::active_block_number::<T>().saturating_sub(issue.opentime);

                let griefing_collateral = issue.griefing_collateral();
                let slashed_collateral = ext::vault_registry::calculate_collateral::<T>(
                    &griefing_collateral,
                    // NOTE: workaround since BlockNumber doesn't inherit Into<U256>
                    &Amount::new(
                        T::BlockNumberToBalance::convert(blocks_elapsed),
                        griefing_collateral.currency(),
                    ),
                    &Amount::new(
                        T::BlockNumberToBalance::convert(issue_period),
                        griefing_collateral.currency(),
                    ),
                )?
                // we can never slash more than the griefing collateral
                .min(&griefing_collateral)?;

                // refund anything not slashed
                let released_collateral = griefing_collateral.saturating_sub(&slashed_collateral)?;
                released_collateral.unlock_on(&requester)?;

                // TODO: update `issue.griefing_collateral`?
                slashed_collateral
            } else {
                return Err(Error::<T>::TimeNotExpired.into());
            };

        if ext::vault_registry::is_vault_liquidated::<T>(&issue.vault)? {
            // return slashed griefing collateral if the vault is liquidated
            to_be_slashed_collateral.unlock_on(&issue.requester)?;
        } else {
            // otherwise give all slashed griefing collateral to the vault
            ext::vault_registry::transfer_funds::<T>(
                CurrencySource::UserGriefing(issue.requester.clone()),
                CurrencySource::FreeBalance(issue.vault.account_id.clone()),
                &to_be_slashed_collateral,
            )?;
        }

        // decrease to-be-issued tokens
        let full_amount = issue.amount().checked_add(&issue.fee())?;
        ext::vault_registry::decrease_to_be_issued_tokens::<T>(&issue.vault, &full_amount)?;

        Self::set_issue_status(issue_id, IssueRequestStatus::Cancelled);

        Self::deposit_event(Event::CancelIssue {
            issue_id,
            requester,
            griefing_collateral: to_be_slashed_collateral.amount(),
        });
        Ok(())
    }

    fn decrease_issue_amount(
        issue_id: &H256,
        issue: &mut DefaultIssueRequest<T>,
        amount_transferred: Amount<T>,
        expected_total_amount: Amount<T>,
    ) -> Result<Amount<T>, DispatchError> {
        // decrease the to-be-issued tokens that will not be issued after all
        let deficit = expected_total_amount.checked_sub(&amount_transferred)?;
        ext::vault_registry::decrease_to_be_issued_tokens::<T>(&issue.vault, &deficit)?;

        // slash/release griefing collateral proportionally to the amount sent
        let to_release_collateral = ext::vault_registry::calculate_collateral::<T>(
            &issue.griefing_collateral(),
            &amount_transferred,
            &expected_total_amount,
        )?;
        let slashed_collateral = issue.griefing_collateral().checked_sub(&to_release_collateral)?;
        ext::vault_registry::transfer_funds::<T>(
            CurrencySource::UserGriefing(issue.requester.clone()),
            CurrencySource::FreeBalance(issue.vault.account_id.clone()),
            &slashed_collateral,
        )?;

        Self::set_issue_amount(&issue_id, issue, amount_transferred, slashed_collateral)?;

        Ok(to_release_collateral)
    }

    fn try_increase_issue_amount(
        issue_id: &H256,
        issue: &mut DefaultIssueRequest<T>,
        amount_transferred: Amount<T>,
        expected_total_amount: Amount<T>,
    ) -> Result<(), DispatchError> {
        let surplus_btc = amount_transferred.checked_sub(&expected_total_amount)?;
        let max_allowed = ext::vault_registry::get_issuable_tokens_from_vault::<T>(&issue.vault)?;
        let issue_amount = surplus_btc.min(&max_allowed)?;

        if let Ok(_) = ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&issue.vault, &issue_amount) {
            // Current vault can handle the surplus; update the issue request
            Self::set_issue_amount(
                &issue_id,
                issue,
                expected_total_amount.checked_add(&issue_amount)?,
                Amount::zero(T::GetGriefingCollateralCurrencyId::get()),
            )?;
        }
        // nothing to do on error
        Ok(())
    }

    /// Fetch all issue requests for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_issue_requests_for_account(account_id: T::AccountId) -> Vec<H256> {
        <IssueRequests<T>>::iter()
            .filter(|(_, request)| request.requester == account_id)
            .map(|(key, _)| key)
            .collect()
    }

    /// Fetch all issue requests for the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - vault account id
    pub fn get_issue_requests_for_vault(vault_id: T::AccountId) -> Vec<H256> {
        <IssueRequests<T>>::iter()
            .filter(|(_, request)| request.vault.account_id == vault_id)
            .map(|(key, _)| key)
            .collect()
    }

    pub fn get_issue_request_from_id(issue_id: &H256) -> Result<DefaultIssueRequest<T>, DispatchError> {
        let request = IssueRequests::<T>::try_get(issue_id).or(Err(Error::<T>::IssueIdNotFound))?;

        // NOTE: temporary workaround until we delete
        match request.status {
            IssueRequestStatus::Completed => Err(Error::<T>::IssueCompleted.into()),
            _ => Ok(request),
        }
    }

    pub fn get_pending_issue(issue_id: &H256) -> Result<DefaultIssueRequest<T>, DispatchError> {
        let request = IssueRequests::<T>::try_get(issue_id).or(Err(Error::<T>::IssueIdNotFound))?;

        // NOTE: temporary workaround until we delete
        match request.status {
            IssueRequestStatus::Completed => Err(Error::<T>::IssueCompleted.into()),
            IssueRequestStatus::Cancelled => Err(Error::<T>::IssueCancelled.into()),
            IssueRequestStatus::Pending => Ok(request),
        }
    }

    /// update the fee & amount in an issue request based on the actually transferred amount
    fn set_issue_amount(
        issue_id: &H256,
        issue: &mut DefaultIssueRequest<T>,
        transferred_btc: Amount<T>,
        confiscated_griefing_collateral: Amount<T>,
    ) -> Result<(), DispatchError> {
        // Current vault can handle the surplus; update the issue request
        issue.fee = ext::fee::get_issue_fee::<T>(&transferred_btc)?.amount();
        issue.amount = transferred_btc.checked_sub(&issue.fee())?.amount();

        // update storage
        <IssueRequests<T>>::mutate_exists(issue_id, |request| {
            *request = request.clone().map(|request| DefaultIssueRequest::<T> {
                fee: issue.fee,
                amount: issue.amount,
                // TODO: update griefing collateral
                ..request
            });
        });

        Self::deposit_event(Event::IssueAmountChange {
            issue_id: *issue_id,
            amount: issue.amount,
            fee: issue.fee,
            confiscated_griefing_collateral: confiscated_griefing_collateral.amount(),
        });

        Ok(())
    }

    fn insert_issue_request(key: &H256, value: &DefaultIssueRequest<T>) {
        <IssueRequests<T>>::insert(key, value)
    }

    fn set_issue_status(id: H256, status: IssueRequestStatus) {
        <IssueRequests<T>>::mutate_exists(id, |request| {
            *request = request
                .clone()
                .map(|request| DefaultIssueRequest::<T> { status, ..request });
        });
    }

    fn issue_btc_dust_value(currency_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(IssueBtcDustValue::<T>::get(), currency_id)
    }
}
