//! # PolkaBTC Vault Registry Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod types;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, EncodeLike};
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::Randomness;
use frame_support::transactional;
use frame_support::weights::Weight;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, IterableStorageMap,
};
use frame_system::ensure_signed;
use primitive_types::U256;
use security::ErrorCode;
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

use crate::types::{
    BtcAddress, DefaultSystemVault, DefaultVault, Inner, PolkaBTC, RichSystemVault, RichVault,
    UnsignedFixedPoint, UpdatableVault, Version, DOT,
};
#[doc(inline)]
pub use crate::types::{BtcPublicKey, CurrencySource, SystemVault, Vault, VaultStatus, Wallet};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RegisterRequest<AccountId, DateTime> {
    registration_id: H256,
    vault: AccountId,
    timeout: DateTime,
}

pub trait WeightInfo {
    fn register_vault() -> Weight;
    fn lock_additional_collateral() -> Weight;
    fn withdraw_collateral() -> Weight;
    fn update_public_key() -> Weight;
    fn register_address() -> Weight;
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + collateral::Config
    + treasury::Config
    + exchange_rate_oracle::Config
    + security::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type RandomnessSource: Randomness<H256>;

    type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as VaultRegistry {
        /// ## Storage
        /// The minimum collateral (DOT) a Vault needs to provide
        /// to participate in the issue process.
        MinimumCollateralVault get(fn minimum_collateral_vault) config(): DOT<T>;

        /// If a Vault fails to execute a correct redeem or replace,
        /// it is temporarily banned from further issue, redeem or replace requests.
        PunishmentDelay get(fn punishment_delay) config(): T::BlockNumber;

        /// Determines the over-collateralization rate for DOT collateral locked
        /// by Vaults, necessary for issuing PolkaBTC. Must to be strictly
        /// greater than 100000 and LiquidationCollateralThreshold.
        SecureCollateralThreshold get(fn secure_collateral_threshold) config(): UnsignedFixedPoint<T>;

        /// Determines the rate for the collateral rate of Vaults, at which the
        /// BTC backed by the Vault are opened up for auction to other Vaults
        AuctionCollateralThreshold get(fn auction_collateral_threshold) config(): UnsignedFixedPoint<T>;

        /// Determines the rate for the collateral rate of Vaults,
        /// at which users receive a premium in DOT, allocated from the
        /// Vault’s collateral, when performing a redeem with this Vault.
        /// Must to be strictly greater than 100000 and LiquidationCollateralThreshold.
        PremiumRedeemThreshold get(fn premium_redeem_threshold) config(): UnsignedFixedPoint<T>;

        /// Determines the lower bound for the collateral rate in PolkaBTC.
        /// Must be strictly greater than 100000. If a Vault’s collateral rate
        /// drops below this, automatic liquidation (forced Redeem) is triggered.
        LiquidationCollateralThreshold get(fn liquidation_collateral_threshold) config(): UnsignedFixedPoint<T>;

        /// Account identifier of an artificial Vault maintained by the VaultRegistry
        /// to handle polkaBTC balances and DOT collateral of liquidated Vaults.
        /// That is, when a Vault is liquidated, its balances are transferred to
        /// LiquidationVault and claims are later handled via the LiquidationVault.
        LiquidationVaultAccountId get(fn liquidation_vault_account_id) config(): T::AccountId;

        LiquidationVault get(fn liquidation_vault) build(|config: &GenesisConfig<T>| {
            SystemVault {
                id: config.liquidation_vault_account_id.clone(),
                to_be_issued_tokens: Default::default(),
                issued_tokens: Default::default(),
                to_be_redeemed_tokens: Default::default(),
            }
        }): SystemVault<T::AccountId, PolkaBTC<T>>;

        /// Mapping of Vaults, using the respective Vault account identifier as key.
        Vaults: map hasher(blake2_128_concat) T::AccountId => Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;

        /// Mapping of reserved BTC addresses to the registered account
        ReservedAddresses: map hasher(blake2_128_concat) BtcAddress => T::AccountId;

        /// Total collateral used for backing polkabtc by active vaults, excluding the liquidation vault
        TotalUserVaultBackingCollateral: DOT<T>;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            Self::_on_runtime_upgrade();

            0
        }

        /// Initiates the registration procedure for a new Vault.
        /// The Vault provides its BTC address and locks up DOT collateral,
        /// which is to be used to the issuing process.
        ///
        /// # Arguments
        /// * `collateral` - the amount of collateral to lock
        /// * `public_key` - the BTC public key of the vault to register
        ///
        /// # Errors
        /// * `InsufficientVaultCollateralAmount` - if the collateral is below the minimum threshold
        /// * `VaultAlreadyRegistered` - if a vault is already registered for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[weight = <T as Config>::WeightInfo::register_vault()]
        #[transactional]
        fn register_vault(origin, collateral: DOT<T>, public_key: BtcPublicKey) -> DispatchResult {
            Self::_register_vault(&ensure_signed(origin)?, collateral, public_key)
        }

        /// Locks additional collateral as a security against stealing the
        /// Bitcoin locked with it.
        ///
        /// # Arguments
        /// * `amount` - the amount of extra collateral to lock
        ///
        /// # Errors
        /// * `VaultNotFound` - if no vault exists for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[weight = <T as Config>::WeightInfo::lock_additional_collateral()]
        #[transactional]
        fn lock_additional_collateral(origin, amount: DOT<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::check_parachain_not_shutdown_and_not_errors([ErrorCode::OracleOffline].to_vec())?;

            Self::try_lock_additional_collateral(&sender, amount)?;

            let vault = Self::get_active_rich_vault_from_id(&sender)?;

            Self::deposit_event(Event::<T>::LockAdditionalCollateral(
                vault.id(),
                amount,
                vault.get_collateral(),
                vault.get_free_collateral()?,
            ));
            Ok(())
        }

        /// Withdraws `amount` of the collateral from the amount locked by
        /// the vault corresponding to the origin account
        /// The collateral left after withdrawal must be more
        /// (free or used in backing issued PolkaBTC) than MinimumCollateralVault
        /// and above the SecureCollateralThreshold. Collateral that is currently
        /// being used to back issued PolkaBTC remains locked until the Vault
        /// is used for a redeem request (full release can take multiple redeem requests).
        ///
        /// # Arguments
        /// * `amount` - the amount of collateral to withdraw
        ///
        /// # Errors
        /// * `VaultNotFound` - if no vault exists for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[weight = <T as Config>::WeightInfo::withdraw_collateral()]
        #[transactional]
        fn withdraw_collateral(origin, amount: DOT<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;

            Self::try_withdraw_collateral(&sender, amount)?;

            let vault = Self::get_active_rich_vault_from_id(&sender)?;

            Self::deposit_event(Event::<T>::WithdrawCollateral(
                sender.clone(),
                amount,
                vault.get_collateral(),
            ));
            Ok(())
        }

        /// Registers a new Bitcoin address for the vault.
        ///
        /// # Arguments
        /// * `public_key` - the BTC public key of the vault to update
        #[weight = <T as Config>::WeightInfo::update_public_key()]
        #[transactional]
        fn update_public_key(origin, public_key: BtcPublicKey) -> DispatchResult {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            let mut vault = Self::get_active_rich_vault_from_id(&account_id)?;
            vault.update_public_key(public_key.clone());
            Self::deposit_event(Event::<T>::UpdatePublicKey(account_id, public_key));
            Ok(())
        }

        #[weight = <T as Config>::WeightInfo::register_address()]
        #[transactional]
        fn register_address(origin, btc_address: BtcAddress) -> DispatchResult {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::insert_vault_deposit_address(&account_id, btc_address.clone())?;
            Self::deposit_event(Event::<T>::RegisterAddress(account_id, btc_address));
            Ok(())
        }

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn begin_block(_height: T::BlockNumber) -> DispatchResult {
        Self::liquidate_undercollateralized_vaults();
        Ok(())
    }

    fn _on_runtime_upgrade() {
        // initialize TotalUserVaultBackingCollateral
        let total = <Vaults<T>>::iter()
            .map(|(_, vault)| vault.backing_collateral)
            .fold(Some(0u32.into()), |total: Option<DOT<T>>, x| {
                total?.checked_add(&x)
            })
            .unwrap_or(0u32.into());

        <TotalUserVaultBackingCollateral<T>>::set(total);
    }

    /// Public functions

    pub fn _register_vault(
        vault_id: &T::AccountId,
        collateral: DOT<T>,
        public_key: BtcPublicKey,
    ) -> DispatchResult {
        ext::security::ensure_parachain_status_running::<T>()?;

        ensure!(
            collateral >= Self::get_minimum_collateral_vault(),
            Error::<T>::InsufficientVaultCollateralAmount
        );
        ensure!(
            !Self::vault_exists(vault_id),
            Error::<T>::VaultAlreadyRegistered
        );

        let vault = Vault::new(vault_id.clone(), public_key).into();
        Self::insert_vault(vault_id, vault);

        Self::try_lock_additional_collateral(vault_id, collateral)?;

        Self::deposit_event(Event::<T>::RegisterVault(vault_id.clone(), collateral));

        Ok(())
    }

    pub fn get_vault_from_id(vault_id: &T::AccountId) -> Result<DefaultVault<T>, DispatchError> {
        ensure!(Self::vault_exists(&vault_id), Error::<T>::VaultNotFound);
        let vault = <Vaults<T>>::get(vault_id);
        Ok(vault)
    }

    pub fn get_backing_collateral(vault_id: &T::AccountId) -> Result<DOT<T>, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;
        Ok(vault.backing_collateral)
    }

    /// Like get_vault_from_id, but additionally checks that the vault is active
    pub fn get_active_vault_from_id(
        vault_id: &T::AccountId,
    ) -> Result<DefaultVault<T>, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;
        ensure!(
            vault.status == VaultStatus::Active,
            Error::<T>::VaultNotFound
        );
        Ok(vault)
    }

    pub fn get_liquidation_vault() -> DefaultSystemVault<T> {
        <LiquidationVault<T>>::get()
    }

    pub fn try_lock_additional_collateral(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;

        // will fail if free_balance is insufficient
        ext::collateral::lock::<T>(vault_id, amount)?;
        Module::<T>::increase_total_backing_collateral(amount)?;

        vault.increase_backing_collateral(amount)?;

        Ok(())
    }

    pub fn force_withdraw_collateral(vault_id: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(vault_id)?;

        ext::collateral::release_collateral::<T>(vault_id, amount)?;
        Module::<T>::decrease_total_backing_collateral(amount)?;

        vault.decrease_backing_collateral(amount)?;

        Ok(())
    }

    pub fn try_withdraw_collateral(vault_id: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        ensure!(
            Self::is_allowed_to_withdraw_collateral(vault_id, amount)?,
            Error::<T>::InsufficientCollateral
        );

        Self::force_withdraw_collateral(vault_id, amount)?;

        Ok(())
    }

    /// checks if the vault would be above the secure threshold after withdrawing collateral
    pub fn is_allowed_to_withdraw_collateral(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<bool, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;

        let new_collateral = match vault.backing_collateral.checked_sub(&amount) {
            Some(x) => x,
            None => return Ok(false),
        };

        let tokens = vault
            .issued_tokens
            .checked_add(&vault.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let is_below_threshold =
            Module::<T>::is_collateral_below_secure_threshold(new_collateral, tokens)?;

        Ok(!is_below_threshold)
    }

    pub fn slash_collateral_saturated(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let available_amount = from.current_balance()?;
        let amount = if available_amount < amount {
            available_amount
        } else {
            amount
        };
        Self::slash_collateral(from, to, amount)?;
        Ok(amount)
    }

    pub fn slash_collateral(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: DOT<T>,
    ) -> DispatchResult {
        // move funds to free balance of the source
        match from {
            CurrencySource::Backing(ref account) => {
                Self::force_withdraw_collateral(account, amount)?;
            }
            CurrencySource::Griefing(_) | CurrencySource::LiquidationVault => {
                ext::collateral::release_collateral::<T>(&from.account_id(), amount)?;
            }
            CurrencySource::FreeBalance(_) => {
                // do nothing
            }
        };

        // move from sender's free balance to receiver's free balance
        ext::collateral::transfer::<T>(&from.account_id(), &to.account_id(), amount)?;

        // move funds to free balance of the source
        match to {
            CurrencySource::Backing(ref account) => {
                Self::try_lock_additional_collateral(account, amount)?;
            }
            CurrencySource::Griefing(_) | CurrencySource::LiquidationVault => {
                ext::collateral::lock::<T>(&to.account_id(), amount)?;
            }
            CurrencySource::FreeBalance(_) => {
                // do nothing
            }
        };

        Ok(())
    }
    /// Checks if the vault has sufficient collateral to increase the to-be-issued tokens, and
    /// if so, increases it
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-issued tokens
    /// * `tokens` - the amount of tokens to be reserved
    pub fn try_increase_to_be_issued_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;

        let issuable_tokens = vault.issuable_tokens()?;
        ensure!(issuable_tokens >= tokens, Error::<T>::ExceedingVaultLimit);
        vault.increase_to_be_issued(tokens)?;

        Self::deposit_event(Event::<T>::IncreaseToBeIssuedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Registers a btc address
    ///
    /// # Arguments
    /// * `issue_id` - secure id for generating deposit address
    pub fn register_deposit_address(
        vault_id: &T::AccountId,
        issue_id: H256,
    ) -> Result<BtcAddress, DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let btc_address = vault.new_deposit_address(issue_id)?;
        Self::deposit_event(Event::<T>::RegisterAddress(vault.id(), btc_address));
        Ok(btc_address)
    }

    /// checks if there are sufficient issued tokens that can be replaced. If so, it
    /// is increased.
    pub fn try_increase_to_be_replaced_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;

        let new_to_be_replaced = vault
            .data
            .to_be_replaced_tokens
            .checked_add(&tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let required_tokens = new_to_be_replaced
            .checked_add(&vault.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        ensure!(
            vault.data.issued_tokens >= required_tokens,
            Error::<T>::InsufficientTokensCommitted
        );

        vault.increase_to_be_replaced(tokens)?;

        Self::deposit_event(Event::<T>::IncreaseToBeReplacedTokens(vault.id(), tokens));

        Ok(())
    }

    pub fn decrease_to_be_replaced_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.decrease_to_be_replaced(tokens)?;

        Ok(())
    }

    /// Decreases the amount of tokens to be issued in the next issue request from the
    /// vault, or from the liquidation vault if the vault is liquidated
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease to-be-issued tokens
    /// * `tokens` - the amount of tokens to be unreserved
    pub fn decrease_to_be_issued_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;

        let mut vault = Self::get_rich_vault_from_id(vault_id)?;
        vault.decrease_to_be_issued(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseToBeIssuedTokens(
            vault_id.clone(),
            tokens,
        ));
        Ok(())
    }

    /// Issues an amount of `tokens` tokens for the given `vault_id`
    /// At this point, the to-be-issued tokens assigned to a vault are decreased
    /// and the issued tokens balance is increased by the amount of issued tokens.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to issue tokens
    /// * `tokens` - the amount of tokens to issue
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of tokens reserved is too low
    pub fn issue_tokens(vault_id: &T::AccountId, tokens: PolkaBTC<T>) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.issue_tokens(tokens)?;
        Self::deposit_event(Event::<T>::IssueTokens(vault.id(), tokens));
        Ok(())
    }

    /// Adds an amount tokens to the to-be-redeemed tokens balance of a vault.
    /// This function serves as a prevention against race conditions in the
    /// redeem and replace procedures. If, for example, a vault would receive
    /// two redeem requests at the same time that have a higher amount of tokens
    ///  to be issued than his issuedTokens balance, one of the two redeem
    /// requests should be rejected.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-redeemed tokens
    /// * `tokens` - the amount of tokens to be redeemed
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of redeemable tokens is too low
    pub fn try_increase_to_be_redeemed_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let redeemable = vault
            .data
            .issued_tokens
            .checked_sub(&vault.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        ensure!(
            redeemable >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );

        vault.increase_to_be_redeemed(tokens)?;

        Self::deposit_event(Event::<T>::IncreaseToBeRedeemedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Subtracts an amount tokens from the to-be-redeemed tokens balance of a vault.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease to-be-redeemed tokens
    /// * `tokens` - the amount of tokens to be redeemed
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of to-be-redeemed tokens is too low
    pub fn decrease_to_be_redeemed_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;

        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.decrease_to_be_redeemed(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseToBeRedeemedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Decreases the amount of tokens f a redeem request is not fulfilled
    /// Removes the amount of tokens assigned to the to-be-redeemed tokens.
    /// At this point, we consider the tokens lost and the issued tokens are
    /// removed from the vault
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease tokens
    /// * `tokens` - the amount of tokens to be decreased
    /// * `user_id` - the id of the user making the redeem request
    pub fn decrease_tokens(
        vault_id: &T::AccountId,
        user_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;
        // decrease to-be-redeemed and issued
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.decrease_tokens(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseTokens(
            vault.id(),
            user_id.clone(),
            tokens,
        ));
        Ok(())
    }

    /// Reduces the to-be-redeemed tokens when a redeem request completes
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to redeem tokens
    /// * `tokens` - the amount of tokens to be decreased
    /// * `premium` - amount of DOT to be rewarded to the redeemer if the vault is not liquidated yet
    /// * `redeemer_id` - the id of the redeemer
    pub fn redeem_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
        premium: DOT<T>,
        redeemer_id: &T::AccountId,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;

        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;

        // need to read before we decrease it
        let to_be_redeemed_tokens = vault.data.to_be_redeemed_tokens;

        vault.decrease_to_be_redeemed(tokens)?;
        vault.decrease_issued(tokens)?;

        if !vault.data.is_liquidated() {
            if premium.is_zero() {
                Self::deposit_event(Event::<T>::RedeemTokens(vault.id(), tokens));
            } else {
                Self::slash_collateral(
                    CurrencySource::Backing(vault_id.clone()),
                    CurrencySource::FreeBalance(redeemer_id.clone()),
                    premium,
                )?;

                Self::deposit_event(Event::<T>::RedeemTokensPremium(
                    vault_id.clone(),
                    tokens,
                    premium,
                    redeemer_id.clone(),
                ));
            }
        } else {
            // withdraw vault collateral
            let amount = Self::calculate_collateral(
                CurrencySource::Backing::<T>(vault_id.clone()).current_balance()?,
                tokens,
                to_be_redeemed_tokens,
            )?;
            Self::force_withdraw_collateral(vault_id, amount)?;

            Self::deposit_event(Event::<T>::RedeemTokensLiquidatedVault(
                vault_id.clone(),
                tokens,
                amount,
            ));
        }

        Ok(())
    }

    /// Handles redeem requests which are executed against the LiquidationVault.
    /// Reduces the issued token of the LiquidationVault and slashes the
    /// corresponding amount of DOT collateral.
    ///
    /// # Arguments
    /// * `redeemer_id` - the account of the user redeeming PolkaBTC
    /// * `tokens` - the amount of PolkaBTC to be redeemed in DOT with the
    ///              LiquidationVault, denominated in BTC
    ///
    /// # Errors
    /// * `InsufficientTokensCommitted` - if the amount of tokens issued by the liquidation vault is too low
    /// * `InsufficientFunds` - if the liquidation vault does not have enough collateral to transfer
    pub fn redeem_tokens_liquidation(
        redeemer_id: &T::AccountId,
        amount_btc: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [ErrorCode::InvalidBTCRelay, ErrorCode::OracleOffline].to_vec(),
        )?;

        let mut liquidation_vault = Self::get_rich_liquidation_vault();

        ensure!(
            liquidation_vault.redeemable_tokens()? >= amount_btc,
            Error::<T>::InsufficientTokensCommitted
        );

        // transfer liquidated collateral to redeemer
        let to_transfer = Self::calculate_collateral(
            CurrencySource::<T>::LiquidationVault.current_balance()?,
            amount_btc,
            liquidation_vault.backed_tokens()?,
        )?;

        Self::slash_collateral(
            CurrencySource::LiquidationVault,
            CurrencySource::FreeBalance(redeemer_id.clone()),
            to_transfer,
        )?;

        liquidation_vault.decrease_issued(amount_btc)?;

        Self::deposit_event(Event::<T>::RedeemTokensLiquidation(
            redeemer_id.clone(),
            amount_btc,
            to_transfer,
        ));

        Ok(())
    }

    /// Replaces the old vault by the new vault by transferring tokens
    /// from the old vault to the new one
    ///
    /// # Arguments
    /// * `old_vault_id` - the id of the old vault
    /// * `new_vault_id` - the id of the new vault
    /// * `tokens` - the amount of tokens to be transferred from the old to the new vault
    /// * `collateral` - the collateral to be locked by the new vault
    ///
    /// # Errors
    /// * `VaultNotFound` - if either the old or new vault does not exist
    /// * `InsufficientTokensCommitted` - if the amount of tokens of the old vault is too low
    /// * `InsufficientFunds` - if the new vault does not have enough collateral to lock
    pub fn replace_tokens(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;

        let mut old_vault = Self::get_rich_vault_from_id(&old_vault_id)?;
        let mut new_vault = Self::get_rich_vault_from_id(&new_vault_id)?;

        if old_vault.data.is_liquidated() {
            // release old-vault's collateral
            let current_backing =
                CurrencySource::<T>::Backing(old_vault_id.clone()).current_balance()?;
            let to_be_released = Self::calculate_collateral(
                current_backing,
                tokens,
                old_vault.data.to_be_redeemed_tokens,
            )?;
            Self::force_withdraw_collateral(&old_vault_id, to_be_released)?;
        }

        old_vault.decrease_tokens(tokens)?;
        new_vault.issue_tokens(tokens)?;

        Self::deposit_event(Event::<T>::ReplaceTokens(
            old_vault_id.clone(),
            new_vault_id.clone(),
            tokens,
            collateral,
        ));
        Ok(())
    }

    /// Cancels a replace - which in the normal case decreases the old-vault's
    /// to-be-redeemed tokens, and the new-vault's to-be-issued tokens.
    /// When one or both of the vaults have been liquidated, this function also
    /// updates the liquidation vault.
    ///
    /// # Arguments
    /// * `old_vault_id` - the id of the old vault
    /// * `new_vault_id` - the id of the new vault
    /// * `tokens` - the amount of tokens to be transferred from the old to the new vault
    pub fn cancel_replace_tokens(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;

        let mut old_vault = Self::get_rich_vault_from_id(&old_vault_id)?;
        let mut new_vault = Self::get_rich_vault_from_id(&new_vault_id)?;

        if old_vault.data.is_liquidated() {
            let old_vault_backing = CurrencySource::<T>::Backing(old_vault_id.clone());

            // transfer old-vault's collateral to liquidation_vault
            let to_be_transfered_collateral = Self::calculate_collateral(
                old_vault_backing.current_balance()?,
                tokens,
                old_vault.data.to_be_redeemed_tokens,
            )?;
            Self::slash_collateral(
                old_vault_backing,
                CurrencySource::LiquidationVault,
                to_be_transfered_collateral,
            )?;
        }

        old_vault.decrease_to_be_redeemed(tokens)?;
        new_vault.decrease_to_be_issued(tokens)?;

        Ok(())
    }

    /// Automatically liquidates all vaults under the secure threshold
    fn liquidate_undercollateralized_vaults() {
        // TODO: report system undercollateralization to security
        let vaults_to_liquidate = <Vaults<T>>::iter()
            .filter_map(|(vault_id, _)| {
                if Self::is_vault_below_liquidation_threshold(&vault_id)
                    .ok()
                    .unwrap_or(false)
                {
                    Some(vault_id)
                } else {
                    None
                }
            })
            .collect::<Vec<T::AccountId>>();

        for vault_id in vaults_to_liquidate {
            // ignore conversion errors since we cannot do anything
            // other than liquidate remaining vaults
            let _ = Self::liquidate_vault(&vault_id);
        }
    }

    /// Liquidates a vault, transferring all of its token balances to the `LiquidationVault`.
    /// Delegates to `liquidate_vault_with_status`, using `Liquidated` status
    pub fn liquidate_vault(vault_id: &T::AccountId) -> DispatchResult {
        Self::liquidate_vault_with_status(vault_id, VaultStatus::Liquidated)
    }

    /// Liquidates a vault, transferring all of its token balances to the
    /// `LiquidationVault`, as well as the DOT collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to liquidate
    /// * `status` - status with which to liquidate the vault
    ///
    /// # Errors
    /// * `VaultNotFound` - if the vault to liquidate does not exist
    pub fn liquidate_vault_with_status(
        vault_id: &T::AccountId,
        status: VaultStatus,
    ) -> DispatchResult {
        // Parachain must not be shutdown
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;

        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let vault_orig = vault.data.clone();
        let mut liquidation_vault = Self::get_rich_liquidation_vault();

        vault.liquidate(&mut liquidation_vault, status)?;

        Self::deposit_event(Event::<T>::LiquidateVault(
            vault_id.clone(),
            vault_orig.issued_tokens,
            vault_orig.to_be_issued_tokens,
            vault_orig.to_be_redeemed_tokens,
            vault_orig.to_be_replaced_tokens,
            vault_orig.backing_collateral,
            status,
        ));
        Ok(())
    }

    pub(crate) fn increase_total_backing_collateral(amount: DOT<T>) -> DispatchResult {
        let new = <TotalUserVaultBackingCollateral<T>>::get()
            .checked_add(&amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        <TotalUserVaultBackingCollateral<T>>::set(new);

        Ok(())
    }

    pub(crate) fn decrease_total_backing_collateral(amount: DOT<T>) -> DispatchResult {
        let new = <TotalUserVaultBackingCollateral<T>>::get()
            .checked_sub(&amount)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        <TotalUserVaultBackingCollateral<T>>::set(new);

        Ok(())
    }

    /// returns the total number of issued tokens
    pub fn get_total_issued_tokens(
        include_liquidation_vault: bool,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        if include_liquidation_vault {
            Ok(ext::treasury::total_issued::<T>())
        } else {
            ext::treasury::total_issued::<T>()
                .checked_sub(&Self::get_liquidation_vault().issued_tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow.into())
        }
    }

    /// returns the total locked collateral, _
    pub fn get_total_backing_collateral(
        include_liquidation_vault: bool,
    ) -> Result<DOT<T>, DispatchError> {
        let liquidated_collateral = CurrencySource::<T>::LiquidationVault.current_balance()?;
        let total = if include_liquidation_vault {
            <TotalUserVaultBackingCollateral<T>>::get()
                .checked_add(&liquidated_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?
        } else {
            <TotalUserVaultBackingCollateral<T>>::get()
        };

        Ok(total)
    }

    pub fn insert_vault(
        id: &T::AccountId,
        vault: Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <Vaults<T>>::insert(id, vault)
    }

    pub fn ban_vault(vault_id: T::AccountId) -> DispatchResult {
        let height = <frame_system::Module<T>>::block_number();
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.ban_until(height + Self::punishment_delay());
        Ok(())
    }

    pub fn _ensure_not_banned(vault_id: &T::AccountId, height: T::BlockNumber) -> DispatchResult {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.ensure_not_banned(height)
    }

    /// Threshold checks
    pub fn is_vault_below_secure_threshold(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, <SecureCollateralThreshold<T>>::get())
    }

    pub fn is_vault_liquidated(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(Self::get_vault_from_id(&vault_id)?.is_liquidated())
    }

    pub fn is_vault_below_auction_threshold(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, <AuctionCollateralThreshold<T>>::get())
    }

    pub fn is_vault_below_premium_threshold(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, <PremiumRedeemThreshold<T>>::get())
    }

    /// check if the vault is below the liquidation threshold. In contrast to other thresholds,
    /// this is checked as ratio of `collateral / (issued - to_be_redeemed)`.
    pub fn is_vault_below_liquidation_threshold(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let vault = Self::get_rich_vault_from_id(&vault_id)?;

        // the current locked backing collateral by the vault
        let collateral = Self::get_backing_collateral(vault_id)?;

        // the currently issued tokens in PolkaBTC
        let tokens = vault
            .data
            .issued_tokens
            .checked_sub(&vault.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        Self::is_collateral_below_threshold(
            collateral,
            tokens,
            <LiquidationCollateralThreshold<T>>::get(),
        )
    }

    pub fn is_collateral_below_secure_threshold(
        collateral: DOT<T>,
        btc_amount: PolkaBTC<T>,
    ) -> Result<bool, DispatchError> {
        let threshold = <SecureCollateralThreshold<T>>::get();
        Self::is_collateral_below_threshold(collateral, btc_amount, threshold)
    }

    pub fn set_secure_collateral_threshold(threshold: UnsignedFixedPoint<T>) {
        <SecureCollateralThreshold<T>>::set(threshold);
    }

    pub fn set_auction_collateral_threshold(threshold: UnsignedFixedPoint<T>) {
        <AuctionCollateralThreshold<T>>::set(threshold);
    }

    pub fn set_premium_redeem_threshold(threshold: UnsignedFixedPoint<T>) {
        <PremiumRedeemThreshold<T>>::set(threshold);
    }

    pub fn set_liquidation_collateral_threshold(threshold: UnsignedFixedPoint<T>) {
        <LiquidationCollateralThreshold<T>>::set(threshold);
    }

    pub fn get_premium_redeem_threshold() -> UnsignedFixedPoint<T> {
        <PremiumRedeemThreshold<T>>::get()
    }

    pub fn get_liquidation_collateral_threshold() -> UnsignedFixedPoint<T> {
        <LiquidationCollateralThreshold<T>>::get()
    }

    pub fn is_over_minimum_collateral(amount: DOT<T>) -> bool {
        amount > Self::get_minimum_collateral_vault()
    }

    /// return (collateral * Numerator) / denominator, used when dealing with liquidated vaults
    pub fn calculate_collateral(
        collateral: DOT<T>,
        numerator: PolkaBTC<T>,
        denominator: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        if numerator.is_zero() && denominator.is_zero() {
            return Ok(collateral);
        }

        let collateral: U256 = Self::dot_to_u128(collateral)?.into();
        let numerator: U256 = Self::polkabtc_to_u128(numerator)?.into();
        let denominator: U256 = Self::polkabtc_to_u128(denominator)?.into();

        let amount = collateral
            .checked_mul(numerator)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_div(denominator)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        Self::u128_to_dot(amount.try_into().map_err(|_| Error::<T>::TryIntoIntError)?)
    }

    /// RPC

    /// Get the total collateralization of the system.
    pub fn get_total_collateralization() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let issued_tokens = Self::get_total_issued_tokens(true)?;
        let total_collateral = Self::get_total_backing_collateral(true)?;

        // convert the issued_tokens to the raw amount
        let raw_issued_tokens = Self::polkabtc_to_u128(issued_tokens)?;
        ensure!(raw_issued_tokens != 0, Error::<T>::NoTokensIssued);

        // convert the collateral to polkabtc
        let collateral_in_polka_btc = ext::oracle::dots_to_btc::<T>(total_collateral)?;
        let raw_collateral_in_polka_btc = Self::polkabtc_to_u128(collateral_in_polka_btc)?;

        Self::get_collateralization(raw_collateral_in_polka_btc, raw_issued_tokens)
    }

    /// Get the first available vault with sufficient collateral to fulfil an issue request
    /// with the specified amount of PolkaBTC.
    pub fn get_first_vault_with_sufficient_collateral(
        amount: PolkaBTC<T>,
    ) -> Result<T::AccountId, DispatchError> {
        // find all vault accounts with sufficient collateral
        let suitable_vaults = <Vaults<T>>::iter()
            .filter_map(|v| {
                // iterator returns tuple of (AccountId, Vault<T>), we check the vault and return the accountid
                let vault = Into::<RichVault<T>>::into(v.1);
                let issuable_tokens = vault.issuable_tokens().ok()?;
                if issuable_tokens >= amount {
                    Some(v.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultWithSufficientCollateral.into())
        } else {
            let idx = Self::pseudo_rand_index(amount, suitable_vaults.len());
            Ok(suitable_vaults[idx].clone())
        }
    }

    /// Get the first available vault with sufficient locked PolkaBTC to fulfil a redeem request.
    pub fn get_first_vault_with_sufficient_tokens(
        amount: PolkaBTC<T>,
    ) -> Result<T::AccountId, DispatchError> {
        // find all vault accounts with sufficient collateral
        let suitable_vaults = <Vaults<T>>::iter()
            .filter_map(|v| {
                // iterator returns tuple of (AccountId, Vault<T>), we check the vault and return the accountid
                let vault = Into::<RichVault<T>>::into(v.1);
                let redeemable_tokens = vault.redeemable_tokens().ok()?;
                if redeemable_tokens >= amount {
                    Some(v.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultWithSufficientTokens.into())
        } else {
            let idx = Self::pseudo_rand_index(amount, suitable_vaults.len());
            Ok(suitable_vaults[idx].clone())
        }
    }

    /// Get all vaults below the premium redeem threshold
    /// Checks three conditions:
    /// 1. the vault must have tokens issued
    /// 2. the vault must be available to redeem tokens (not all issued tokens currently bein part of redeem/replace processes)
    /// 3. the vault must be below the premium redeem threshold
    ///
    /// Maybe returns a tuple of (VaultId, RedeemableTokens)
    /// The redeemable tokens are the currently vault.issued_tokens - the vault.to_be_redeemed_tokens
    pub fn get_premium_redeem_vaults() -> Result<Vec<(T::AccountId, PolkaBTC<T>)>, DispatchError> {
        let mut suitable_vaults = <Vaults<T>>::iter()
            .filter_map(|(account_id, vault)| {
                // iterator returns tuple of (AccountId, Vault<T>),
                if !vault.issued_tokens.is_zero()
                    && !vault
                        .issued_tokens
                        .saturating_sub(vault.to_be_redeemed_tokens)
                        .is_zero()
                    && Self::is_vault_below_premium_threshold(&account_id).unwrap_or(false)
                {
                    Some((
                        account_id,
                        vault
                            .issued_tokens
                            .saturating_sub(vault.to_be_redeemed_tokens),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<(_, _)>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultUnderThePremiumRedeemThreshold.into())
        } else {
            suitable_vaults.sort_by(|a, b| b.1.cmp(&a.1));
            Ok(suitable_vaults)
        }
    }

    /// Get all vaults with non-zero issuable tokens, ordered in descending order of this amount
    pub fn get_vaults_with_issuable_tokens(
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>)>, DispatchError> {
        let mut vaults_with_issuable_tokens = <Vaults<T>>::iter()
            .filter_map(|(account_id, _vault)| {
                // iterator returns tuple of (AccountId, Vault<T>),
                match Self::get_issuable_tokens_from_vault(account_id.clone()).ok() {
                    Some(issuable_tokens) => {
                        if !issuable_tokens.is_zero() {
                            Some((account_id, issuable_tokens))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            })
            .collect::<Vec<(_, _)>>();

        if vaults_with_issuable_tokens.is_empty() {
            Err(Error::<T>::NoVaultWithIssuableTokens.into())
        } else {
            vaults_with_issuable_tokens.sort_by(|a, b| b.1.cmp(&a.1));
            Ok(vaults_with_issuable_tokens)
        }
    }

    /// Get the amount of tokens a vault can issue
    pub fn get_issuable_tokens_from_vault(
        vault_id: T::AccountId,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.issuable_tokens()
    }

    /// Get the current collateralization of a vault
    pub fn get_collateralization_from_vault(
        vault_id: T::AccountId,
        only_issued: bool,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let collateral = vault.get_collateral();
        Self::get_collateralization_from_vault_and_collateral(vault_id, collateral, only_issued)
    }

    pub fn get_collateralization_from_vault_and_collateral(
        vault_id: T::AccountId,
        collateral: DOT<T>,
        only_issued: bool,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let issued_tokens = if only_issued {
            vault.data.issued_tokens
        } else {
            vault.data.issued_tokens + vault.data.to_be_issued_tokens
        };

        // convert the issued_tokens to the raw amount
        let raw_issued_tokens = Self::polkabtc_to_u128(issued_tokens)?;
        ensure!(raw_issued_tokens != 0, Error::<T>::NoTokensIssued);

        // convert the collateral to polkabtc
        let collateral_in_polka_btc = ext::oracle::dots_to_btc::<T>(collateral)?;
        let raw_collateral_in_polka_btc = Self::polkabtc_to_u128(collateral_in_polka_btc)?;

        Self::get_collateralization(raw_collateral_in_polka_btc, raw_issued_tokens)
    }

    /// Gets the minimum amount of collateral required for the given amount of btc
    /// with the current threshold and exchange rate
    ///
    /// # Arguments
    /// * `amount_btc` - the amount of polkabtc
    pub fn get_required_collateral_for_polkabtc(
        amount_btc: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let threshold = <SecureCollateralThreshold<T>>::get();
        let collateral =
            Self::get_required_collateral_for_polkabtc_with_threshold(amount_btc, threshold)?;
        Ok(collateral)
    }

    /// Get the amount of collateral required for the given vault to be at the
    /// current SecureCollateralThreshold with the current exchange rate
    pub fn get_required_collateral_for_vault(
        vault_id: T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let issued_tokens = vault.data.issued_tokens + vault.data.to_be_issued_tokens;

        let required_collateral = Self::get_required_collateral_for_polkabtc(issued_tokens)?;

        Ok(required_collateral)
    }

    /// Private getters and setters

    fn get_rich_vault_from_id(vault_id: &T::AccountId) -> Result<RichVault<T>, DispatchError> {
        Ok(Self::get_vault_from_id(vault_id)?.into())
    }

    /// Like get_rich_vault_from_id, but only returns active vaults
    fn get_active_rich_vault_from_id(
        vault_id: &T::AccountId,
    ) -> Result<RichVault<T>, DispatchError> {
        Ok(Self::get_active_vault_from_id(vault_id)?.into())
    }

    fn get_rich_liquidation_vault() -> RichSystemVault<T> {
        Into::<RichSystemVault<T>>::into(Self::get_liquidation_vault())
    }

    fn vault_exists(id: &T::AccountId) -> bool {
        <Vaults<T>>::contains_key(id)
    }

    fn get_minimum_collateral_vault() -> DOT<T> {
        <MinimumCollateralVault<T>>::get()
    }

    // Other helpers

    /// get a psuedorandom value between 0 (inclusive) and `limit` (exclusive), based on
    /// the hashes of the last 81 blocks, and the given subject.
    ///
    /// # Arguments
    ///
    /// * `subject` - an extra value to feed into the pseudorandom number generator
    /// * `limit` - the limit of the returned value
    fn pseudo_rand_index(subject: PolkaBTC<T>, limit: usize) -> usize {
        let raw_subject = Self::polkabtc_to_u128(subject).unwrap_or(0 as u128);

        // convert into a slice. Endianness of the conversion function is arbitrary chosen
        let bytes = &raw_subject.to_be_bytes();

        let rand_hash = T::RandomnessSource::random(bytes);

        let ret = rand_hash.to_low_u64_le() % (limit as u64);
        ret as usize
    }

    /// Ensure that the parachain is NOT shutdown and DOES NOT have the given errors
    ///
    /// # Arguments
    ///
    ///   * `error_codes` - list of `ErrorCode` to be checked
    ///
    fn check_parachain_not_shutdown_and_not_errors(error_codes: Vec<ErrorCode>) -> DispatchResult {
        // Parachain must not be shutdown
        ext::security::ensure_parachain_status_not_shutdown::<T>()?;
        // There must not be in InvalidBTCRelay, OracleOffline or Liquidation error state
        ext::security::ensure_parachain_does_not_have_errors::<T>(error_codes)
    }

    /// calculate the collateralization as a ratio of the issued tokens to the
    /// amount of provided collateral at the current exchange rate.
    fn get_collateralization(
        raw_collateral_in_polka_btc: u128,
        raw_issued_tokens: u128,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let collateralization = UnsignedFixedPoint::<T>::checked_from_rational(
            raw_collateral_in_polka_btc,
            raw_issued_tokens,
        )
        .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(collateralization)
    }

    fn is_vault_below_threshold(
        vault_id: &T::AccountId,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<bool, DispatchError> {
        let vault = Self::get_rich_vault_from_id(&vault_id)?;

        // the currently issued tokens in PolkaBTC
        let issued_tokens = vault.data.issued_tokens;

        // the current locked backing collateral by the vault
        let collateral = Self::get_backing_collateral(vault_id)?;

        Self::is_collateral_below_threshold(collateral, issued_tokens, threshold)
    }

    fn is_collateral_below_threshold(
        collateral: DOT<T>,
        btc_amount: PolkaBTC<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<bool, DispatchError> {
        let max_tokens =
            Self::calculate_max_polkabtc_from_collateral_for_threshold(collateral, threshold)?;
        // check if the max_tokens are below the issued tokens
        Ok(max_tokens < btc_amount)
    }

    /// Gets the minimum amount of collateral required for the given amount of btc
    /// with the current exchange rate and the given threshold. This function is the
    /// inverse of calculate_max_polkabtc_from_collateral_for_threshold
    ///
    /// # Arguments
    /// * `amount_btc` - the amount of polkabtc
    /// * `threshold` - the required secure collateral threshold
    fn get_required_collateral_for_polkabtc_with_threshold(
        btc: PolkaBTC<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<DOT<T>, DispatchError> {
        // Step 1: inverse of the scaling applied in calculate_max_polkabtc_from_collateral_for_threshold
        let btc = Self::polkabtc_to_u128(btc)?;
        let btc = threshold
            .checked_mul_int_rounded_up(btc)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        let btc = Self::u128_to_polkabtc(btc)?;

        // Step 2: convert the amount to dots
        let amount_in_dot = ext::oracle::btc_to_dots::<T>(btc)?;
        Ok(amount_in_dot)
    }

    fn calculate_max_polkabtc_from_collateral_for_threshold(
        collateral: DOT<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        // convert the collateral to polkabtc
        let collateral_in_polka_btc = ext::oracle::dots_to_btc::<T>(collateral)?;
        let collateral_in_polka_btc = Self::polkabtc_to_u128(collateral_in_polka_btc)?;

        // calculate how many tokens should be maximally issued given the threshold.
        let collateral_as_inner = TryInto::<Inner<T>>::try_into(collateral_in_polka_btc)
            .map_err(|_| Error::<T>::TryIntoIntError)?;
        let max_btc_as_inner = UnsignedFixedPoint::<T>::checked_from_integer(collateral_as_inner)
            .ok_or(Error::<T>::TryIntoIntError)?
            .checked_div(&threshold)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .into_inner()
            .checked_div(&UnsignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        let max_btc_raw = UniqueSaturatedInto::<u128>::unique_saturated_into(max_btc_as_inner);

        Ok(Self::u128_to_polkabtc(max_btc_raw)?)
    }

    fn polkabtc_to_u128(x: PolkaBTC<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn dot_to_u128(x: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_polkabtc(x: u128) -> Result<PolkaBTC<T>, DispatchError> {
        TryInto::<PolkaBTC<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    pub fn insert_vault_deposit_address(
        vault_id: &T::AccountId,
        btc_address: BtcAddress,
    ) -> DispatchResult {
        ensure!(
            !<ReservedAddresses<T>>::contains_key(&btc_address),
            Error::<T>::ReservedDepositAddress
        );
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        vault.insert_deposit_address(btc_address);
        <ReservedAddresses<T>>::insert(btc_address, vault_id);
        Ok(())
    }

    pub fn new_vault_deposit_address(
        vault_id: &T::AccountId,
        secure_id: H256,
    ) -> Result<BtcAddress, DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        let btc_address = vault.new_deposit_address(secure_id)?;
        Ok(btc_address)
    }
}

decl_event! {
    /// ## Events
    pub enum Event<T> where
            AccountId = <T as frame_system::Config>::AccountId,
            DOT = DOT<T>,
            PolkaBTC = PolkaBTC<T> {
        RegisterVault(AccountId, DOT),
        /// id, new collateral, total collateral, free collateral
        LockAdditionalCollateral(AccountId, DOT, DOT, DOT),
        /// id, withdrawn collateral, total collateral
        WithdrawCollateral(AccountId, DOT, DOT),
        UpdatePublicKey(AccountId, BtcPublicKey),
        RegisterAddress(AccountId, BtcAddress),
        IncreaseToBeIssuedTokens(AccountId, PolkaBTC),
        DecreaseToBeIssuedTokens(AccountId, PolkaBTC),
        IssueTokens(AccountId, PolkaBTC),
        IncreaseToBeRedeemedTokens(AccountId, PolkaBTC),
        DecreaseToBeRedeemedTokens(AccountId, PolkaBTC),
        IncreaseToBeReplacedTokens(AccountId, PolkaBTC),
        DecreaseTokens(AccountId, AccountId, PolkaBTC),
        RedeemTokens(AccountId, PolkaBTC),
        RedeemTokensPremium(AccountId, PolkaBTC, DOT, AccountId),
        RedeemTokensLiquidatedVault(AccountId, PolkaBTC, DOT),
        RedeemTokensLiquidation(AccountId, PolkaBTC, DOT),
        ReplaceTokens(AccountId, AccountId, PolkaBTC, DOT),
        LiquidateVault(AccountId, PolkaBTC, PolkaBTC, PolkaBTC, PolkaBTC, DOT, VaultStatus),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        InsufficientCollateral,
        /// The amount of tokens to be issued is higher than the issuable amount by the vault
        ExceedingVaultLimit,
        InsufficientTokensCommitted,
        VaultBanned,
        /// Returned if the collateral amount to register a vault was too low
        InsufficientVaultCollateralAmount,
        // FIXME: ERR_MIN_AMOUNT in spec
        /// Returned if a vault tries to register while already being registered
        VaultAlreadyRegistered,
        VaultNotFound,
        /// The Bitcoin Address has already been registered
        ReservedDepositAddress,
        /// Collateralization is infinite if no tokens are issued
        NoTokensIssued,
        NoVaultWithSufficientCollateral,
        NoVaultWithSufficientTokens,
        NoVaultUnderThePremiumRedeemThreshold,
        NoVaultWithIssuableTokens,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        /// Unable to convert value
        TryIntoIntError,
        InvalidSecretKey,
        InvalidPublicKey,
    }
}

trait CheckedMulIntRoundedUp {
    /// Like checked_mul_int, but this version rounds the result up instead of down.
    fn checked_mul_int_rounded_up(self, n: u128) -> Option<u128>;
}
impl<T: FixedPointNumber> CheckedMulIntRoundedUp for T {
    fn checked_mul_int_rounded_up(self, n: u128) -> Option<u128> {
        // convert n into fixed_point
        let n_inner = TryInto::<T::Inner>::try_into(n).ok()?;
        let n_fixed_point = T::checked_from_integer(n_inner)?;

        // do the multiplication
        let product = self.checked_mul(&n_fixed_point)?;

        // convert to inner
        let product_inner =
            UniqueSaturatedInto::<u128>::unique_saturated_into(product.into_inner());

        // convert to u128 by dividing by a rounded up division by accuracy
        let accuracy = UniqueSaturatedInto::<u128>::unique_saturated_into(T::accuracy());
        product_inner
            .checked_add(accuracy)?
            .checked_sub(1)?
            .checked_div(accuracy)
    }
}
