#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

/// # Vault Registry implementation
/// This is the implementation of the Vault Registry following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html
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
use frame_support::weights::Weight;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, IterableStorageMap,
};
use frame_system::ensure_signed;
use security::ErrorCode;
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use util::transactional;

use crate::types::{
    BtcAddress, DefaultSystemVault, DefaultVault, Inner, PolkaBTC, RichSystemVault, RichVault,
    UnsignedFixedPoint, UpdatableVault, Version, DOT,
};
pub use crate::types::{BtcPublicKey, SystemVault, Vault, VaultStatus, Wallet};

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
        Vaults: map hasher(blake2_128_concat) T::AccountId => Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>;

        /// Mapping of reserved BTC addresses to the registered account
        ReservedAddresses: map hasher(blake2_128_concat) BtcAddress => T::AccountId;

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
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;

            ensure!(collateral >= Self::get_minimum_collateral_vault(),
                    Error::<T>::InsufficientVaultCollateralAmount);
            ensure!(!Self::vault_exists(&sender), Error::<T>::VaultAlreadyRegistered);

            ext::collateral::lock::<T>(&sender, collateral)?;
            let vault = Vault::new(sender.clone(), public_key);
            Self::insert_vault(&sender, vault.clone());

            Self::deposit_event(Event::<T>::RegisterVault(vault.id, collateral));

            Ok(())
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

            let vault = Self::get_active_rich_vault_from_id(&sender)?;
            vault.increase_collateral(amount)?;
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
            let vault = Self::get_active_rich_vault_from_id(&sender)?;
            vault.withdraw_collateral(amount)?;
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

    /// Public functions

    pub fn get_vault_from_id(vault_id: &T::AccountId) -> Result<DefaultVault<T>, DispatchError> {
        ensure!(Self::vault_exists(&vault_id), Error::<T>::VaultNotFound);
        let vault = <Vaults<T>>::get(vault_id);
        Ok(vault)
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

    /// Increases the amount of tokens to be issued in the next issue request
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-issued tokens
    /// * `secure_id` - secure id for generating deposit address
    /// * `tokens` - the amount of tokens to be reserved
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `ExceedingVaultLimit` - if the amount of tokens to be issued is higher than the issuable amount by the vault
    pub fn increase_to_be_issued_tokens(
        vault_id: &T::AccountId,
        secure_id: H256,
        tokens: PolkaBTC<T>,
    ) -> Result<BtcAddress, DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.increase_to_be_issued(tokens)?;
        Self::deposit_event(Event::<T>::IncreaseToBeIssuedTokens(vault.id(), tokens));
        let btc_address = vault.new_deposit_address(secure_id)?;
        Self::deposit_event(Event::<T>::RegisterAddress(vault.id(), btc_address));
        Ok(btc_address)
    }

    pub fn force_increase_to_be_issued_tokens(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.increase_to_be_issued(tokens)?;
        Self::deposit_event(Event::<T>::IncreaseToBeIssuedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Decreases the amount of tokens to be issued in the next issue request
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease to-be-issued tokens
    /// * `tokens` - the amount of tokens to be unreserved
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of tokens reserved is too low
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

        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.decrease_to_be_issued(tokens)?;
        Self::deposit_event(Event::<T>::DecreaseToBeIssuedTokens(vault.id(), tokens));
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
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
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
    pub fn increase_to_be_redeemed_tokens(
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
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
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
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of to-be-redeemed tokens
    ///                                   or issued tokens is too low
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
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.decrease_tokens(tokens)?;
        Self::deposit_event(Event::<T>::DecreaseTokens(
            vault.id(),
            user_id.clone(),
            tokens,
        ));
        Ok(())
    }

    /// Reduces the to-be-redeemed tokens when a redeem request successfully completes
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to redeem tokens
    /// * `tokens` - the amount of tokens to be decreased
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of to-be-redeemed tokens
    ///                                   or issued tokens is too low
    pub fn redeem_tokens(vault_id: &T::AccountId, tokens: PolkaBTC<T>) -> DispatchResult {
        Self::check_parachain_not_shutdown_and_not_errors(
            [
                ErrorCode::InvalidBTCRelay,
                ErrorCode::OracleOffline,
                ErrorCode::Liquidation,
            ]
            .to_vec(),
        )?;
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.redeem_tokens(tokens)?;
        Self::deposit_event(Event::<T>::RedeemTokens(vault.id(), tokens));
        Ok(())
    }

    /// Handles a redeem request, where a user is paid a premium in DOT.
    /// Calls redeem_tokens and then allocates the corresponding amount of DOT
    /// to the redeemer using the Vault’s free collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to redeem premiums
    /// * `tokens` - the amount of tokens redeemed
    /// * `premium` - the amount of DOT to be paid to the user as a premium
    ///               using the Vault’s released collateral.
    /// * `user` - the user redeeming at a premium
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of to-be-redeemed tokens
    ///                                   or issued tokens is too low
    /// * `InsufficientFunds` - if the vault does not have `premium` amount of collateral
    pub fn redeem_tokens_premium(
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
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.redeem_tokens(tokens)?;
        if premium > 0.into() {
            ext::collateral::slash_collateral::<T>(vault_id, redeemer_id, premium)?;
        }

        Self::deposit_event(Event::<T>::RedeemTokensPremium(
            vault_id.clone(),
            tokens,
            premium,
            redeemer_id.clone(),
        ));
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
        liquidation_vault.decrease_issued(amount_btc)?;

        // transfer liquidated collateral to redeemer
        let amount_dot = ext::oracle::btc_to_dots::<T>(amount_btc)?;
        ext::collateral::slash_collateral::<T>(
            &liquidation_vault.data.id,
            &redeemer_id,
            amount_dot,
        )?;
        ext::collateral::release_collateral::<T>(&redeemer_id, amount_dot)?;

        Self::deposit_event(Event::<T>::RedeemTokensLiquidation(
            redeemer_id.clone(),
            amount_btc,
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

        let mut old_vault = Self::get_active_rich_vault_from_id(&old_vault_id)?;
        let mut new_vault = Self::get_active_rich_vault_from_id(&new_vault_id)?;
        old_vault.transfer(&mut new_vault, tokens)?;
        new_vault.increase_collateral(collateral)?;

        Self::deposit_event(Event::<T>::ReplaceTokens(
            old_vault_id.clone(),
            new_vault_id.clone(),
            tokens,
            collateral,
        ));
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
        let mut liquidation_vault = Self::get_rich_liquidation_vault();

        vault.liquidate(&mut liquidation_vault, status)?;

        Self::deposit_event(Event::<T>::LiquidateVault(vault_id.clone()));
        Ok(())
    }

    pub fn insert_vault(
        id: &T::AccountId,
        vault: Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
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

    pub fn is_vault_below_liquidation_threshold(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, <LiquidationCollateralThreshold<T>>::get())
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

    /// RPC

    /// Get the total collateralization of the system.
    pub fn get_total_collateralization() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let issued_tokens = ext::treasury::total_issued::<T>();
        let total_collateral = ext::collateral::total_locked::<T>();

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
                if v.1.issued_tokens >= amount {
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

        // the current locked collateral by the vault
        let collateral = ext::collateral::for_account::<T>(&vault_id);

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
            BTCBalance = PolkaBTC<T> {
        RegisterVault(AccountId, DOT),
        /// id, new collateral, total collateral, free collateral
        LockAdditionalCollateral(AccountId, DOT, DOT, DOT),
        /// id, withdrawn collateral, total collateral
        WithdrawCollateral(AccountId, DOT, DOT),
        UpdatePublicKey(AccountId, BtcPublicKey),
        RegisterAddress(AccountId, BtcAddress),
        IncreaseToBeIssuedTokens(AccountId, BTCBalance),
        DecreaseToBeIssuedTokens(AccountId, BTCBalance),
        IssueTokens(AccountId, BTCBalance),
        IncreaseToBeRedeemedTokens(AccountId, BTCBalance),
        DecreaseToBeRedeemedTokens(AccountId, BTCBalance),
        DecreaseTokens(AccountId, AccountId, BTCBalance),
        RedeemTokens(AccountId, BTCBalance),
        RedeemTokensPremium(AccountId, BTCBalance, DOT, AccountId),
        RedeemTokensLiquidation(AccountId, BTCBalance),
        ReplaceTokens(AccountId, AccountId, BTCBalance, DOT),
        LiquidateVault(AccountId),
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
