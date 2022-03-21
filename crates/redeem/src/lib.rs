//! # Redeem Pallet
//! Based on the [specification](https://spec.interlay.io/spec/redeem.html).

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
pub use crate::types::{DefaultRedeemRequest, RedeemRequest, RedeemRequestStatus};

use crate::types::{BalanceOf, RedeemRequestExt, Version};
use btc_relay::BtcAddress;
use currency::Amount;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
};
use frame_system::{ensure_root, ensure_signed};
use oracle::OracleKey;
use sp_core::H256;
use sp_runtime::{ArithmeticError, FixedPointNumber};
use sp_std::{convert::TryInto, vec::Vec};
use types::DefaultVaultId;
use vault_registry::{
    types::{CurrencyId, DefaultVaultCurrencyPair},
    CurrencySource,
};

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
        frame_system::Config + vault_registry::Config + btc_relay::Config + fee::Config<UnsignedInner = BalanceOf<Self>>
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RequestRedeem {
            redeem_id: H256,
            redeemer: T::AccountId,
            vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            premium: BalanceOf<T>,
            btc_address: BtcAddress,
            transfer_fee: BalanceOf<T>,
        },
        LiquidationRedeem {
            redeemer: T::AccountId,
            amount: BalanceOf<T>,
        },
        ExecuteRedeem {
            redeem_id: H256,
            redeemer: T::AccountId,
            vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            transfer_fee: BalanceOf<T>,
        },
        CancelRedeem {
            redeem_id: H256,
            redeemer: T::AccountId,
            vault_id: DefaultVaultId<T>,
            slashed_amount: BalanceOf<T>,
            status: RedeemRequestStatus,
        },
        MintTokensForReimbursedRedeem {
            redeem_id: H256,
            vault_id: DefaultVaultId<T>,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account has insufficient balance.
        AmountExceedsUserBalance,
        /// Unexpected redeem account.
        UnauthorizedRedeemer,
        /// Unexpected vault account.
        UnauthorizedVault,
        /// Redeem request has not expired.
        TimeNotExpired,
        /// Redeem request already cancelled.
        RedeemCancelled,
        /// Redeem request already completed.
        RedeemCompleted,
        /// Redeem request not found.
        RedeemIdNotFound,
        /// Unable to convert value.
        TryIntoIntError,
        /// Redeem amount is too small.
        AmountBelowDustAmount,
    }

    /// The time difference in number of blocks between a redeem request is created and required completion time by a
    /// vault. The redeem period has an upper limit to ensure the user gets their BTC in time and to potentially
    /// punish a vault for inactivity or stealing BTC.
    #[pallet::storage]
    #[pallet::getter(fn redeem_period)]
    pub(super) type RedeemPeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Users create redeem requests to receive BTC in return for their previously issued tokens.
    /// This mapping provides access from a unique hash redeemId to a Redeem struct.
    #[pallet::storage]
    #[pallet::getter(fn redeem_requests)]
    pub(super) type RedeemRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, DefaultRedeemRequest<T>, OptionQuery>;

    /// The minimum amount of btc that is accepted for redeem requests; any lower values would
    /// risk the bitcoin client to reject the payment
    #[pallet::storage]
    #[pallet::getter(fn redeem_btc_dust_value)]
    pub(super) type RedeemBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// the expected size in bytes of the redeem bitcoin transfer
    #[pallet::storage]
    #[pallet::getter(fn redeem_transaction_size)]
    pub(super) type RedeemTransactionSize<T: Config> = StorageValue<_, u32, ValueQuery>;

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
        pub redeem_period: T::BlockNumber,
        pub redeem_btc_dust_value: BalanceOf<T>,
        pub redeem_transaction_size: u32,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                redeem_period: Default::default(),
                redeem_btc_dust_value: Default::default(),
                redeem_transaction_size: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            RedeemPeriod::<T>::put(self.redeem_period);
            RedeemBtcDustValue::<T>::put(self.redeem_btc_dust_value);
            RedeemTransactionSize::<T>::put(self.redeem_transaction_size);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initializes a request to burn issued tokens against a Vault with sufficient tokens. It will
        /// also ensure that the Parachain status is RUNNING.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of issued tokens
        /// * `btc_address` - the address to receive BTC
        /// * `vault_id` - address of the vault
        #[pallet::weight(<T as Config>::WeightInfo::request_redeem())]
        #[transactional]
        pub fn request_redeem(
            origin: OriginFor<T>,
            #[pallet::compact] amount_wrapped: BalanceOf<T>,
            btc_address: BtcAddress,
            vault_id: DefaultVaultId<T>,
        ) -> DispatchResultWithPostInfo {
            let redeemer = ensure_signed(origin)?;
            Self::_request_redeem(redeemer, amount_wrapped, btc_address, vault_id)?;
            Ok(().into())
        }

        /// When a Vault is liquidated, its collateral is slashed up to 150% of the liquidated BTC value.
        /// To re-establish the physical 1:1 peg, the bridge allows users to burn issued tokens in return for
        /// collateral at a premium rate.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `collateral_currency` - currency to be received
        /// * `wrapped_currency` - currency of the wrapped token to burn
        /// * `amount_wrapped` - amount of issued tokens to burn
        #[pallet::weight(<T as Config>::WeightInfo::liquidation_redeem())]
        #[transactional]
        pub fn liquidation_redeem(
            origin: OriginFor<T>,
            currencies: DefaultVaultCurrencyPair<T>,
            #[pallet::compact] amount_wrapped: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let redeemer = ensure_signed(origin)?;
            Self::_liquidation_redeem(redeemer, currencies, amount_wrapped)?;
            Ok(().into())
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
        #[pallet::weight(<T as Config>::WeightInfo::execute_redeem())]
        #[transactional]
        pub fn execute_redeem(
            origin: OriginFor<T>,
            redeem_id: H256,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;
            Self::_execute_redeem(redeem_id, merkle_proof, raw_tx)?;

            // Don't take tx fees on success. If the vault had to pay for this function, it would
            // have been vulnerable to a griefing attack where users would redeem amounts just
            // above the dust value.
            Ok(Pays::No.into())
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
        #[pallet::weight(if *reimburse { <T as Config>::WeightInfo::cancel_redeem_reimburse() } else { <T as Config>::WeightInfo::cancel_redeem_retry() })]
        #[transactional]
        pub fn cancel_redeem(origin: OriginFor<T>, redeem_id: H256, reimburse: bool) -> DispatchResultWithPostInfo {
            let redeemer = ensure_signed(origin)?;
            Self::_cancel_redeem(redeemer, redeem_id, reimburse)?;
            Ok(().into())
        }

        /// Set the default redeem period for tx verification.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - default period for new requests
        ///
        /// # Weight: `O(1)`
        #[pallet::weight(<T as Config>::WeightInfo::set_redeem_period())]
        #[transactional]
        pub fn set_redeem_period(origin: OriginFor<T>, period: T::BlockNumber) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <RedeemPeriod<T>>::set(period);
            Ok(().into())
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
        #[pallet::weight(<T as Config>::WeightInfo::set_redeem_period())]
        #[transactional]
        pub fn mint_tokens_for_reimbursed_redeem(
            origin: OriginFor<T>,
            currency_pair: DefaultVaultCurrencyPair<T>,
            redeem_id: H256,
        ) -> DispatchResultWithPostInfo {
            let vault_id = VaultId::new(ensure_signed(origin)?, currency_pair.collateral, currency_pair.wrapped);
            Self::_mint_tokens_for_reimbursed_redeem(vault_id, redeem_id)?;
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn _request_redeem(
        redeemer: T::AccountId,
        amount_wrapped: BalanceOf<T>,
        btc_address: BtcAddress,
        vault_id: DefaultVaultId<T>,
    ) -> Result<H256, DispatchError> {
        let amount_wrapped = Amount::new(amount_wrapped, vault_id.wrapped_currency());

        ext::security::ensure_parachain_status_running::<T>()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(&redeemer, vault_id.wrapped_currency());
        ensure!(
            amount_wrapped.le(&redeemer_balance)?,
            Error::<T>::AmountExceedsUserBalance
        );

        // todo: currently allowed to redeem from one currency to the other for free - decide if this is desirable
        let fee_wrapped = if redeemer == vault_id.account_id {
            Amount::zero(vault_id.wrapped_currency())
        } else {
            ext::fee::get_redeem_fee::<T>(&amount_wrapped)?
        };
        let inclusion_fee = Self::get_current_inclusion_fee(vault_id.wrapped_currency())?;

        let vault_to_be_burned_tokens = amount_wrapped.checked_sub(&fee_wrapped)?;

        // this can overflow for small requested values. As such return AmountBelowDustAmount when this happens
        let user_to_be_received_btc = vault_to_be_burned_tokens
            .checked_sub(&inclusion_fee)
            .map_err(|_| Error::<T>::AmountBelowDustAmount)?;

        ext::vault_registry::ensure_not_banned::<T>(&vault_id)?;

        // only allow requests of amount above above the minimum
        ensure!(
            // this is the amount the vault will send (minus fee)
            user_to_be_received_btc.ge(&Self::get_dust_value(vault_id.wrapped_currency()))?,
            Error::<T>::AmountBelowDustAmount
        );

        // vault will get rid of the btc + btc_inclusion_fee
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<T>(&vault_id, &vault_to_be_burned_tokens)?;

        // lock full amount (inc. fee)
        amount_wrapped.lock_on(&redeemer)?;
        let redeem_id = ext::security::get_secure_id::<T>(&redeemer);

        let below_premium_redeem = ext::vault_registry::is_vault_below_premium_threshold::<T>(&vault_id)?;
        let currency_id = vault_id.collateral_currency();

        let premium_collateral = if below_premium_redeem {
            let redeem_amount_wrapped_in_collateral = user_to_be_received_btc.convert_to(currency_id)?;
            ext::fee::get_premium_redeem_fee::<T>(&redeem_amount_wrapped_in_collateral)?
        } else {
            Amount::zero(currency_id)
        };

        // decrease to-be-replaced tokens - when the vault requests tokens to be replaced, it
        // want to get rid of tokens, and it does not matter whether this is through a redeem,
        // or a replace. As such, we decrease the to-be-replaced tokens here. This call will
        // never fail due to insufficient to-be-replaced tokens
        let (_, griefing_collateral) =
            ext::vault_registry::decrease_to_be_replaced_tokens::<T>(&vault_id, &vault_to_be_burned_tokens)?;
        // release the griefing collateral that is locked for the replace request
        if !griefing_collateral.is_zero() {
            ext::vault_registry::transfer_funds(
                CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
                CurrencySource::FreeBalance(vault_id.account_id.clone()),
                &griefing_collateral,
            )?;
        }

        Self::insert_redeem_request(
            &redeem_id,
            &RedeemRequest {
                vault: vault_id.clone(),
                opentime: ext::security::active_block_number::<T>(),
                fee: fee_wrapped.amount(),
                transfer_fee_btc: inclusion_fee.amount(),
                amount_btc: user_to_be_received_btc.amount(),
                premium: premium_collateral.amount(),
                period: Self::redeem_period(),
                redeemer: redeemer.clone(),
                btc_address,
                btc_height: ext::btc_relay::get_best_block_height::<T>(),
                status: RedeemRequestStatus::Pending,
            },
        );

        Self::deposit_event(Event::<T>::RequestRedeem {
            redeem_id,
            redeemer,
            amount: user_to_be_received_btc.amount(),
            fee: fee_wrapped.amount(),
            premium: premium_collateral.amount(),
            vault_id,
            btc_address,
            transfer_fee: inclusion_fee.amount(),
        });

        Ok(redeem_id)
    }

    fn _liquidation_redeem(
        redeemer: T::AccountId,
        currencies: DefaultVaultCurrencyPair<T>,
        amount_wrapped: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let amount_wrapped = Amount::new(amount_wrapped, currencies.wrapped);

        let redeemer_balance = ext::treasury::get_balance::<T>(&redeemer, currencies.wrapped);
        ensure!(
            amount_wrapped.le(&redeemer_balance)?,
            Error::<T>::AmountExceedsUserBalance
        );

        amount_wrapped.lock_on(&redeemer)?;
        amount_wrapped.burn_from(&redeemer)?;
        ext::vault_registry::redeem_tokens_liquidation::<T>(currencies.collateral, &redeemer, &amount_wrapped)?;

        // vault-registry emits `RedeemTokensLiquidation` with collateral amount
        Self::deposit_event(Event::<T>::LiquidationRedeem {
            redeemer,
            amount: amount_wrapped.amount(),
        });

        Ok(())
    }

    fn _execute_redeem(redeem_id: H256, raw_merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> Result<(), DispatchError> {
        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;

        // check the transaction inclusion and validity
        let transaction = ext::btc_relay::parse_transaction::<T>(&raw_tx)?;
        let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(&raw_merkle_proof)?;
        ext::btc_relay::verify_and_validate_op_return_transaction::<T, _>(
            merkle_proof,
            transaction,
            redeem.btc_address,
            redeem.amount_btc,
            redeem_id,
        )?;

        // burn amount (without parachain fee, but including transfer fee)
        let burn_amount = redeem.amount_btc().checked_add(&redeem.transfer_fee_btc())?;
        burn_amount.burn_from(&redeem.redeemer)?;

        // send fees to pool
        let fee = redeem.fee();
        fee.unlock_on(&redeem.redeemer)?;
        fee.transfer(&redeem.redeemer, &ext::fee::fee_pool_account_id::<T>())?;
        ext::fee::distribute_rewards::<T>(&fee)?;

        ext::vault_registry::redeem_tokens::<T>(&redeem.vault, &burn_amount, &redeem.premium()?, &redeem.redeemer)?;

        Self::set_redeem_status(redeem_id, RedeemRequestStatus::Completed);
        Self::deposit_event(Event::<T>::ExecuteRedeem {
            redeem_id,
            redeemer: redeem.redeemer,
            vault_id: redeem.vault,
            amount: redeem.amount_btc,
            fee: redeem.fee,
            transfer_fee: redeem.transfer_fee_btc,
        });
        Ok(())
    }

    fn _cancel_redeem(redeemer: T::AccountId, redeem_id: H256, reimburse: bool) -> DispatchResult {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;
        ensure!(redeemer == redeem.redeemer, Error::<T>::UnauthorizedRedeemer);

        // only cancellable after the request has expired
        ensure!(
            ext::btc_relay::has_request_expired::<T>(
                redeem.opentime,
                redeem.btc_height,
                Self::redeem_period().max(redeem.period)
            )?,
            Error::<T>::TimeNotExpired
        );

        let vault = ext::vault_registry::get_vault_from_id::<T>(&redeem.vault)?;
        let vault_to_be_redeemed_tokens = Amount::new(vault.to_be_redeemed_tokens, redeem.vault.wrapped_currency());
        let vault_id = redeem.vault.clone();

        let vault_to_be_burned_tokens = redeem.amount_btc().checked_add(&redeem.transfer_fee_btc())?;

        let amount_wrapped_in_collateral = vault_to_be_burned_tokens.convert_to(vault_id.collateral_currency())?;

        // now update the collateral; the logic is different for liquidated vaults.
        let slashed_amount = if vault.is_liquidated() {
            let confiscated_collateral = ext::vault_registry::calculate_collateral::<T>(
                &ext::vault_registry::get_liquidated_collateral::<T>(&redeem.vault)?,
                &vault_to_be_burned_tokens,
                &vault_to_be_redeemed_tokens, // note: this is the value read prior to making changes
            )?;

            let slashing_destination = if reimburse {
                CurrencySource::FreeBalance(redeemer.clone())
            } else {
                CurrencySource::LiquidationVault(vault_id.currencies.clone())
            };
            ext::vault_registry::decrease_liquidated_collateral::<T>(&vault_id, &confiscated_collateral)?;
            ext::vault_registry::transfer_funds::<T>(
                CurrencySource::LiquidatedCollateral(vault_id.clone()),
                slashing_destination,
                &confiscated_collateral,
            )?;

            confiscated_collateral
        } else {
            // not liquidated

            // calculate the punishment fee (e.g. 10%)
            let punishment_fee_in_collateral = ext::fee::get_punishment_fee::<T>(&amount_wrapped_in_collateral)?;

            let amount_to_slash = if reimburse {
                // 100% + punishment fee on reimburse
                amount_wrapped_in_collateral.checked_add(&punishment_fee_in_collateral)?
            } else {
                punishment_fee_in_collateral
            };

            ext::vault_registry::transfer_funds_saturated::<T>(
                CurrencySource::Collateral(vault_id.clone()),
                CurrencySource::FreeBalance(redeemer.clone()),
                &amount_to_slash,
            )?;

            let _ = ext::vault_registry::ban_vault::<T>(&vault_id);

            amount_to_slash
        };

        // first update the issued tokens; this logic is the same regardless of whether or not the vault is liquidated
        let new_status = if reimburse {
            // Transfer the transaction fee to the pool. Even though the redeem was not
            // successful, the user receives a premium in collateral, so it's OK to take the fee.
            let fee = redeem.fee();
            fee.unlock_on(&redeem.redeemer)?;
            fee.transfer(&redeem.redeemer, &ext::fee::fee_pool_account_id::<T>())?;
            ext::fee::distribute_rewards::<T>(&fee)?;

            if ext::vault_registry::is_vault_below_secure_threshold::<T>(&redeem.vault)? {
                // vault can not afford to back the tokens that it would receive, so we burn it
                vault_to_be_burned_tokens.burn_from(&redeemer)?;
                ext::vault_registry::decrease_tokens::<T>(&redeem.vault, &redeem.redeemer, &vault_to_be_burned_tokens)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(false))
            } else {
                // Transfer the rest of the user's issued tokens (i.e. excluding fee) to the vault
                vault_to_be_burned_tokens.unlock_on(&redeem.redeemer)?;
                vault_to_be_burned_tokens.transfer(&redeem.redeemer, &redeem.vault.account_id)?;
                ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, &vault_to_be_burned_tokens)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(true))
            }
        } else {
            // unlock user's issued tokens, including fee
            let total_wrapped: Amount<T> = redeem
                .amount_btc()
                .checked_add(&redeem.fee())?
                .checked_add(&redeem.transfer_fee_btc())?;
            total_wrapped.unlock_on(&redeemer)?;
            ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, &vault_to_be_burned_tokens)?;
            Self::set_redeem_status(redeem_id, RedeemRequestStatus::Retried)
        };

        Self::deposit_event(Event::<T>::CancelRedeem {
            redeem_id,
            redeemer,
            vault_id: redeem.vault,
            slashed_amount: slashed_amount.amount(),
            status: new_status,
        });

        Ok(())
    }

    fn _mint_tokens_for_reimbursed_redeem(vault_id: DefaultVaultId<T>, redeem_id: H256) -> DispatchResult {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeem = RedeemRequests::<T>::try_get(&redeem_id).or(Err(Error::<T>::RedeemIdNotFound))?;
        ensure!(
            matches!(redeem.status, RedeemRequestStatus::Reimbursed(false)),
            Error::<T>::RedeemCancelled
        );

        ensure!(redeem.vault == vault_id, Error::<T>::UnauthorizedVault);

        let reimbursed_amount = redeem.amount_btc().checked_add(&redeem.transfer_fee_btc())?;

        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&vault_id, &reimbursed_amount)?;
        ext::vault_registry::issue_tokens::<T>(&vault_id, &reimbursed_amount)?;
        reimbursed_amount.mint_to(&vault_id.account_id)?;

        Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(true));

        Self::deposit_event(Event::<T>::MintTokensForReimbursedRedeem {
            redeem_id,
            vault_id: redeem.vault,
            amount: reimbursed_amount.amount(),
        });

        Ok(())
    }

    /// Insert a new redeem request into state.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    /// * `value` - the redeem request
    fn insert_redeem_request(key: &H256, value: &DefaultRedeemRequest<T>) {
        <RedeemRequests<T>>::insert(key, value)
    }

    fn set_redeem_status(id: H256, status: RedeemRequestStatus) -> RedeemRequestStatus {
        <RedeemRequests<T>>::mutate_exists(id, |request| {
            *request = request.clone().map(|request| DefaultRedeemRequest::<T> {
                status: status.clone(),
                ..request
            });
        });

        status
    }

    /// get current inclusion fee based on the expected number of bytes in the transaction, and
    /// the inclusion fee rate reported by the oracle
    pub fn get_current_inclusion_fee(wrapped_currency: CurrencyId<T>) -> Result<Amount<T>, DispatchError> {
        let size: u32 = Self::redeem_transaction_size();
        let satoshi_per_bytes = ext::oracle::get_price::<T>(OracleKey::FeeEstimation)?;

        let fee = satoshi_per_bytes
            .checked_mul_int(size)
            .ok_or(ArithmeticError::Overflow)?;
        let amount = fee.try_into().map_err(|_| Error::<T>::TryIntoIntError)?;
        Ok(Amount::new(amount, wrapped_currency))
    }

    pub fn get_dust_value(currency_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(<RedeemBtcDustValue<T>>::get(), currency_id)
    }
    /// Fetch all redeem requests for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_redeem_requests_for_account(account_id: T::AccountId) -> Vec<H256> {
        <RedeemRequests<T>>::iter()
            .filter(|(_, request)| request.redeemer == account_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    /// Fetch all redeem requests for the specified vault.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - vault account id
    pub fn get_redeem_requests_for_vault(vault_id: T::AccountId) -> Vec<H256> {
        <RedeemRequests<T>>::iter()
            .filter(|(_, request)| request.vault.account_id == vault_id)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
    }

    /// Fetch a pre-existing redeem request or throw. Completed or cancelled
    /// requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `redeem_id` - 256-bit identifier of the redeem request
    pub fn get_open_redeem_request_from_id(redeem_id: &H256) -> Result<DefaultRedeemRequest<T>, DispatchError> {
        let request = RedeemRequests::<T>::try_get(redeem_id).or(Err(Error::<T>::RedeemIdNotFound))?;

        // NOTE: temporary workaround until we delete
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
    ) -> Result<DefaultRedeemRequest<T>, DispatchError> {
        let request = RedeemRequests::<T>::try_get(redeem_id).or(Err(Error::<T>::RedeemIdNotFound))?;

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
