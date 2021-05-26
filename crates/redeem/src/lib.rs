//! # Redeem Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/redeem.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
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
pub use crate::types::{RedeemRequest, RedeemRequestStatus};

use crate::types::{Collateral, Version, Wrapped};
use btc_relay::BtcAddress;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
};
use frame_system::{ensure_root, ensure_signed};
use sp_core::H256;
use sp_runtime::traits::*;
use sp_std::{convert::TryInto, vec::Vec};
use vault_registry::CurrencySource;

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + vault_registry::Config
    + currency::Config<currency::Collateral>
    + currency::Config<currency::Wrapped>
    + btc_relay::Config
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
    trait Store for Module<T: Config> as Redeem {
        /// The time difference in number of blocks between a redeem request is created and required completion time by a vault.
        /// The redeem period has an upper limit to ensure the user gets their BTC in time and to potentially punish a vault for inactivity or stealing BTC.
        RedeemPeriod get(fn redeem_period) config(): T::BlockNumber;

        /// Users create redeem requests to receive BTC in return for their previously issued tokens.
        /// This mapping provides access from a unique hash redeemId to a Redeem struct.
        RedeemRequests: map hasher(blake2_128_concat) H256 => RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>;

        /// The minimum amount of btc that is accepted for redeem requests; any lower values would
        /// risk the bitcoin client to reject the payment
        RedeemBtcDustValue get(fn redeem_btc_dust_value) config(): Wrapped<T>;

        /// the expected size in bytes of the redeem bitcoin transfer
        RedeemTransactionSize get(fn redeem_transaction_size) config(): u32;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V2): Version = Version::V0;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Wrapped = Wrapped<T>,
        Collateral = Collateral<T>,
    {
        RequestRedeem(
            H256,       // redeem_id
            AccountId,  // redeemer
            Wrapped,    // redeem_amount_wrapped
            Wrapped,    // fee_wrapped
            Collateral, // premium
            AccountId,  // vault_id
            BtcAddress, // user btc_address
            Wrapped,    // transfer_fee_btc
        ),
        // [redeemer, amount_wrapped]
        LiquidationRedeem(AccountId, Wrapped),
        // [redeem_id, redeemer, amount_wrapped, fee_wrapped, vault]
        ExecuteRedeem(H256, AccountId, Wrapped, Wrapped, AccountId),
        // [redeem_id, redeemer, vault_id, slashing_amount_in_collateral, reimburse]
        CancelRedeem(H256, AccountId, AccountId, Collateral, bool),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// Initializes a request to burn issued tokens against a Vault with sufficient tokens. It will
        /// also ensure that the Parachain status is RUNNING.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of issued tokens
        /// * `btc_address` - the address to receive BTC
        /// * `vault_id` - address of the vault
        #[weight = <T as Config>::WeightInfo::request_redeem()]
        #[transactional]
        fn request_redeem(origin, #[compact] amount_wrapped: Wrapped<T>, btc_address: BtcAddress, vault_id: T::AccountId)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_request_redeem(redeemer, amount_wrapped, btc_address, vault_id)?;
            Ok(())
        }

        /// When a Vault is liquidated, its collateral is slashed up to 150% of the liquidated BTC value.
        /// To re-establish the physical 1:1 peg, the bridge allows users to burn issued tokens in return for
        /// collateral at a premium rate.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount_wrapped` - amount of issued tokens to burn
        #[weight = <T as Config>::WeightInfo::liquidation_redeem()]
        #[transactional]
        fn liquidation_redeem(origin, #[compact] amount_wrapped: Wrapped<T>) -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_liquidation_redeem(redeemer, amount_wrapped)?;
            Ok(())
        }

        /// A Vault calls this function after receiving an RequestRedeem event with their public key.
        /// Before calling the function, the Vault transfers the specific amount of BTC to the BTC address
        /// given in the original redeem request. The Vault completes the redeem with this function.
        ///
        /// # Arguments
        ///
        /// * `origin` - anyone executing this redeem request
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        /// * `tx_id` - transaction hash
        /// * `tx_block_height` - block number of collateral chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        #[weight = <T as Config>::WeightInfo::execute_redeem()]
        #[transactional]
        fn execute_redeem(origin, redeem_id: H256, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_execute_redeem(redeem_id, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// If a redeem request is not completed on time, the redeem request can be cancelled.
        /// The user that initially requested the redeem process calls this function to obtain
        /// the Vault’s collateral as compensation for not refunding the BTC back to their address.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        /// * `reimburse` - specifying if the user wishes to be reimbursed in collateral
        /// and slash the Vault, or wishes to keep the tokens (and retry
        /// Redeem with another Vault)
        #[weight = if *reimburse { <T as Config>::WeightInfo::cancel_redeem_reimburse() } else { <T as Config>::WeightInfo::cancel_redeem_retry() }]
        #[transactional]
        fn cancel_redeem(origin, redeem_id: H256, reimburse: bool)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_cancel_redeem(redeemer, redeem_id, reimburse)?;
            Ok(())
        }

        /// Set the default redeem period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::set_redeem_period()]
        #[transactional]
        fn set_redeem_period(origin, period: T::BlockNumber) {
            ensure_root(origin)?;
            <RedeemPeriod<T>>::set(period);
        }

        /// Mint tokens for a redeem that was cancelled with reimburse=true. This is
        /// only possible if at the time of the cancel_redeem, the vault did not have
        /// sufficient collateral after being slashed to back the tokens that the user
        /// used to hold.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::set_redeem_period()]
        #[transactional]
        fn mint_tokens_for_reimbursed_redeem(origin, redeem_id: H256)
            -> DispatchResult
        {
        let redeemer = ensure_signed(origin)?;
            Self::_mint_tokens_for_reimbursed_redeem(redeemer, redeem_id)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn _request_redeem(
        redeemer: T::AccountId,
        amount_wrapped: Wrapped<T>,
        btc_address: BtcAddress,
        vault_id: T::AccountId,
    ) -> Result<H256, DispatchError> {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(amount_wrapped <= redeemer_balance, Error::<T>::AmountExceedsUserBalance);

        let fee_wrapped = ext::fee::get_redeem_fee::<T>(amount_wrapped)?;
        let inclusion_fee = Self::get_current_inclusion_fee()?;

        let vault_to_be_burned_tokens = amount_wrapped
            .checked_sub(&fee_wrapped)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        // this can overflow for small requested values. As such return AmountBelowDustAmount when this happens
        let user_to_be_received_btc = vault_to_be_burned_tokens
            .checked_sub(&inclusion_fee)
            .ok_or(Error::<T>::AmountBelowDustAmount)?;

        ext::vault_registry::ensure_not_banned::<T>(&vault_id)?;

        // only allow requests of amount above above the minimum
        let dust_value = <RedeemBtcDustValue<T>>::get();
        ensure!(
            // this is the amount the vault will send (minus fee)
            user_to_be_received_btc >= dust_value,
            Error::<T>::AmountBelowDustAmount
        );

        // vault will get rid of the btc + btc_inclusion_fee
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<T>(&vault_id, vault_to_be_burned_tokens)?;

        // lock full amount (inc. fee)
        ext::treasury::lock::<T>(redeemer.clone(), amount_wrapped)?;
        let redeem_id = ext::security::get_secure_id::<T>(&redeemer);

        let below_premium_redeem = ext::vault_registry::is_vault_below_premium_threshold::<T>(&vault_id)?;
        let premium_collateral = if below_premium_redeem {
            let redeem_amount_wrapped_in_collateral = ext::oracle::wrapped_to_collateral::<T>(user_to_be_received_btc)?;
            ext::fee::get_premium_redeem_fee::<T>(redeem_amount_wrapped_in_collateral)?
        } else {
            Collateral::<T>::zero()
        };

        // decrease to-be-replaced tokens - when the vault requests tokens to be replaced, it
        // want to get rid of tokens, and it does not matter whether this is through a redeem,
        // or a replace. As such, we decrease the to-be-replaced tokens here. This call will
        // never fail due to insufficient to-be-replaced tokens
        let (_, griefing_collateral) =
            ext::vault_registry::decrease_to_be_replaced_tokens::<T>(&vault_id, vault_to_be_burned_tokens)?;
        // release the griefing collateral that is locked for the replace request
        if !griefing_collateral.is_zero() {
            ext::collateral::release_collateral::<T>(&vault_id, griefing_collateral)?;
        }

        Self::insert_redeem_request(
            redeem_id,
            RedeemRequest {
                vault: vault_id.clone(),
                opentime: ext::security::active_block_number::<T>(),
                fee: fee_wrapped,
                transfer_fee_btc: inclusion_fee,
                amount_btc: user_to_be_received_btc,
                premium: premium_collateral,
                period: Self::redeem_period(),
                redeemer: redeemer.clone(),
                btc_address,
                btc_height: ext::btc_relay::get_best_block_height::<T>(),
                status: RedeemRequestStatus::Pending,
            },
        );

        // TODO: add fee to redeem event
        Self::deposit_event(<Event<T>>::RequestRedeem(
            redeem_id,
            redeemer,
            user_to_be_received_btc,
            fee_wrapped,
            premium_collateral,
            vault_id,
            btc_address,
            inclusion_fee,
        ));

        Ok(redeem_id)
    }

    fn _liquidation_redeem(redeemer: T::AccountId, amount_wrapped: Wrapped<T>) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(amount_wrapped <= redeemer_balance, Error::<T>::AmountExceedsUserBalance);

        ext::treasury::lock::<T>(redeemer.clone(), amount_wrapped)?;
        ext::treasury::burn::<T>(redeemer.clone(), amount_wrapped)?;
        ext::vault_registry::redeem_tokens_liquidation::<T>(&redeemer, amount_wrapped)?;

        // vault-registry emits `RedeemTokensLiquidation` with collateral amount
        Self::deposit_event(<Event<T>>::LiquidationRedeem(redeemer, amount_wrapped));

        Ok(())
    }

    fn _execute_redeem(redeem_id: H256, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;

        // only executable before the request has expired
        ensure!(
            !ext::security::has_expired::<T>(redeem.opentime, Self::redeem_period().max(redeem.period))?,
            Error::<T>::CommitPeriodExpired
        );

        let amount: usize = redeem.amount_btc.try_into().map_err(|_e| Error::<T>::TryIntoIntError)?;

        // check the transaction inclusion and validity
        ext::btc_relay::verify_and_validate_transaction::<T>(
            merkle_proof,
            raw_tx,
            redeem.btc_address,
            Some(amount as i64),
            Some(redeem_id),
            None,
        )?;

        // burn amount (without parachain fee, but including transfer fee)
        let burn_amount = redeem
            .amount_btc
            .checked_add(&redeem.transfer_fee_btc)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ext::treasury::burn::<T>(redeem.redeemer.clone(), burn_amount)?;

        // send fees to pool
        ext::treasury::unlock_and_transfer::<T>(
            redeem.redeemer.clone(),
            ext::fee::fee_pool_account_id::<T>(),
            redeem.fee,
        )?;
        ext::fee::distribute_wrapped_rewards::<T>(redeem.fee)?;

        ext::vault_registry::redeem_tokens::<T>(&redeem.vault, burn_amount, redeem.premium, &redeem.redeemer)?;

        Self::set_redeem_status(redeem_id, RedeemRequestStatus::Completed);
        Self::deposit_event(<Event<T>>::ExecuteRedeem(
            redeem_id,
            redeem.redeemer,
            redeem.amount_btc,
            redeem.fee,
            redeem.vault,
        ));
        Ok(())
    }

    fn _cancel_redeem(redeemer: T::AccountId, redeem_id: H256, reimburse: bool) -> DispatchResult {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;
        ensure!(redeemer == redeem.redeemer, Error::<T>::UnauthorizedUser);

        // only cancellable after the request has expired
        ensure!(
            ext::security::has_expired::<T>(redeem.opentime, Self::redeem_period().max(redeem.period))?,
            Error::<T>::TimeNotExpired
        );

        let vault = ext::vault_registry::get_vault_from_id::<T>(&redeem.vault)?;
        let vault_id = redeem.vault.clone();

        let vault_to_be_burned_tokens = redeem
            .amount_btc
            .checked_add(&redeem.transfer_fee_btc)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let amount_wrapped_in_collateral = ext::oracle::wrapped_to_collateral::<T>(vault_to_be_burned_tokens)?;
        let punishment_fee_in_collateral = ext::fee::get_punishment_fee::<T>(amount_wrapped_in_collateral)?;

        // now update the collateral; the logic is different for liquidated vaults.
        let slashed_amount = if vault.is_liquidated() {
            let confiscated_collateral = ext::vault_registry::calculate_collateral::<T>(
                ext::vault_registry::get_liquidated_collateral::<T>(&redeem.vault)?,
                vault_to_be_burned_tokens,
                vault.to_be_redeemed_tokens, // note: this is the value read at start of function
            )?;

            let slashing_destination = if reimburse {
                CurrencySource::FreeBalance(redeemer.clone())
            } else {
                CurrencySource::LiquidationVault
            };
            ext::vault_registry::decrease_liquidated_collateral::<T>(&vault_id, confiscated_collateral)?;
            ext::vault_registry::transfer_funds::<T>(
                CurrencySource::ReservedBalance(vault_id.clone()),
                slashing_destination,
                confiscated_collateral,
            )?;

            confiscated_collateral
        } else {
            // not liquidated
            let slashed_collateral = if reimburse {
                // user requested to be reimbursed in collateral
                let reimburse_in_collateral = amount_wrapped_in_collateral
                    .checked_add(&punishment_fee_in_collateral)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ext::vault_registry::transfer_funds_saturated::<T>(
                    CurrencySource::Collateral(vault_id.clone()),
                    CurrencySource::FreeBalance(redeem.redeemer.clone()),
                    reimburse_in_collateral,
                )?
            } else {
                // user chose to keep their issued tokens - only transfer it the punishment fee
                // returns the amount actually slashed
                ext::vault_registry::transfer_funds_saturated::<T>(
                    CurrencySource::Collateral(vault_id.clone()),
                    CurrencySource::FreeBalance(redeemer.clone()),
                    punishment_fee_in_collateral,
                )?
            };
            // calculate additional amount to slash, a high SLA means we slash less
            let slashing_amount_in_collateral =
                ext::vault_registry::calculate_slashed_amount::<T>(&vault_id, amount_wrapped_in_collateral, reimburse)?;

            // slash the remaining amount from the vault to the fee pool
            let remaining_collateral_to_be_slashed = slashing_amount_in_collateral
                .checked_sub(&slashed_collateral)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            if remaining_collateral_to_be_slashed > Collateral::<T>::zero() {
                let slashed_to_fee_pool = ext::vault_registry::transfer_funds_saturated::<T>(
                    CurrencySource::Collateral(vault_id.clone()),
                    CurrencySource::FreeBalance(ext::fee::fee_pool_account_id::<T>()),
                    remaining_collateral_to_be_slashed,
                )?;
                ext::fee::distribute_collateral_rewards::<T>(slashed_to_fee_pool)?;
            }
            let _ = ext::vault_registry::ban_vault::<T>(vault_id.clone());

            slashing_amount_in_collateral
        };

        // first update the issued tokens; this logic is the same regardless of whether or not the vault is liquidated
        if reimburse {
            // Transfer the transaction fee to the pool. Even though the redeem was not
            // successful, the user receives a premium in collateral, so it's to take the fee.
            ext::treasury::unlock_and_transfer::<T>(
                redeem.redeemer.clone(),
                ext::fee::fee_pool_account_id::<T>(),
                redeem.fee,
            )?;
            ext::fee::distribute_wrapped_rewards::<T>(redeem.fee)?;

            if ext::vault_registry::is_vault_below_secure_threshold::<T>(&redeem.vault)? {
                // vault can not afford to back the tokens that he would receive, so we burn it
                ext::treasury::burn::<T>(redeemer.clone(), vault_to_be_burned_tokens)?;
                ext::vault_registry::decrease_tokens::<T>(&redeem.vault, &redeem.redeemer, vault_to_be_burned_tokens)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(false));
            } else {
                // Transfer the rest of the user's issued tokens (i.e. excluding fee) to the vault
                ext::treasury::unlock_and_transfer::<T>(
                    redeem.redeemer.clone(),
                    redeem.vault.clone(),
                    vault_to_be_burned_tokens,
                )?;
                ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, vault_to_be_burned_tokens)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(true));
            }
        } else {
            // unlock user's issued tokens, including fee
            let total_wrapped = redeem
                .amount_btc
                .checked_add(&redeem.fee)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&redeem.transfer_fee_btc)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ext::treasury::unlock::<T>(redeemer.clone(), total_wrapped)?;
            ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, vault_to_be_burned_tokens)?;
            Self::set_redeem_status(redeem_id, RedeemRequestStatus::Retried);
        }

        ext::sla::event_update_vault_sla::<T>(&vault_id, ext::sla::VaultEvent::RedeemFailure)?;
        Self::deposit_event(<Event<T>>::CancelRedeem(
            redeem_id,
            redeemer,
            redeem.vault,
            slashed_amount,
            reimburse,
        ));

        Ok(())
    }

    fn _mint_tokens_for_reimbursed_redeem(vault_id: T::AccountId, redeem_id: H256) -> DispatchResult {
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;
        ensure!(
            <RedeemRequests<T>>::contains_key(&redeem_id),
            Error::<T>::RedeemIdNotFound
        );
        let redeem = <RedeemRequests<T>>::get(&redeem_id);
        ensure!(
            matches!(redeem.status, RedeemRequestStatus::Reimbursed(false)),
            Error::<T>::RedeemCancelled
        );

        ensure!(redeem.vault == vault_id, Error::<T>::UnauthorizedUser);

        let reimbursed_amount = redeem
            .amount_btc
            .checked_add(&redeem.transfer_fee_btc)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&vault_id, reimbursed_amount)?;
        ext::vault_registry::issue_tokens::<T>(&vault_id, reimbursed_amount)?;
        ext::treasury::mint::<T>(vault_id, reimbursed_amount);

        Self::set_redeem_status(redeem_id, RedeemRequestStatus::Completed);

        Ok(())
    }

    /// get current inclusion fee based on the expected number of bytes in the transaction, and
    /// the inclusion fee rate reported by the oracle
    fn get_current_inclusion_fee() -> Result<Wrapped<T>, DispatchError> {
        {
            let size: u32 = Self::redeem_transaction_size();
            let satoshi_per_bytes: u32 = ext::oracle::satoshi_per_bytes::<T>().fast;

            let fee = (size as u64)
                .checked_mul(satoshi_per_bytes as u64)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            fee.try_into().map_err(|_| Error::<T>::TryIntoIntError.into())
        }
    }

    /// Insert a new redeem request into state.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    /// * `value` - the redeem request
    fn insert_redeem_request(key: H256, value: RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>) {
        <RedeemRequests<T>>::insert(key, value)
    }

    fn set_redeem_status(id: H256, status: RedeemRequestStatus) {
        // TODO: delete redeem request from storage
        <RedeemRequests<T>>::mutate(id, |request| {
            request.status = status;
        });
    }

    /// Fetch all redeem requests for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_redeem_requests_for_account(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>,
    )> {
        <RedeemRequests<T>>::iter()
            .filter(|(_, request)| request.redeemer == account_id)
            .collect::<Vec<_>>()
    }

    /// Fetch all redeem requests for the specified vault.
    ///
    /// # Arguments
    ///
    /// * `account_id` - vault account id
    pub fn get_redeem_requests_for_vault(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>,
    )> {
        <RedeemRequests<T>>::iter()
            .filter(|(_, request)| request.vault == account_id)
            .collect::<Vec<_>>()
    }

    /// Fetch a pre-existing redeem request or throw. Completed or cancelled
    /// requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `redeem_id` - 256-bit identifier of the redeem request
    pub fn get_open_redeem_request_from_id(
        redeem_id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>, DispatchError> {
        ensure!(
            <RedeemRequests<T>>::contains_key(redeem_id),
            Error::<T>::RedeemIdNotFound
        );
        // NOTE: temporary workaround until we delete

        let request = <RedeemRequests<T>>::get(redeem_id);
        match request.status {
            RedeemRequestStatus::Pending => Ok(request),
            RedeemRequestStatus::Completed => Err(Error::<T>::RedeemCompleted.into()),
            RedeemRequestStatus::Reimbursed(_) | RedeemRequestStatus::Retried => {
                Err(Error::<T>::RedeemCancelled.into())
            }
        }
    }

    /// Fetch a pre-existing open or completed redeem request or throw.
    /// Cancelled requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `redeem_id` - 256-bit identifier of the redeem request
    pub fn get_open_or_completed_redeem_request_from_id(
        redeem_id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>, DispatchError> {
        ensure!(
            <RedeemRequests<T>>::contains_key(*redeem_id),
            Error::<T>::RedeemIdNotFound
        );

        let request = <RedeemRequests<T>>::get(*redeem_id);

        ensure!(
            matches!(
                request.status,
                RedeemRequestStatus::Pending | RedeemRequestStatus::Completed
            ),
            Error::<T>::RedeemCancelled
        );
        Ok(request)
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        AmountExceedsUserBalance,
        AmountExceedsVaultBalance,
        CommitPeriodExpired,
        UnauthorizedUser,
        TimeNotExpired,
        RedeemCancelled,
        RedeemCompleted,
        RedeemIdNotFound,
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        AmountBelowDustAmount,
    }
}
