#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

/// # Vault Registry implementation
/// This is the implementation of the Vault Registry following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html
mod ext;
mod types;
pub mod vault;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use frame_support::{decl_event, decl_module, decl_storage, ensure};
use primitive_types::H256;
use sp_core::H160;
use system::ensure_signed;

use x_core::Error;

use crate::types::{DOTBalance, PolkaBTCBalance};
use crate::vault::DefaultVault;
pub use crate::vault::{RichVault, Vault};

/// Granularity of `SecureCollateralThreshold`, `AuctionCollateralThreshold`,
/// `LiquidationCollateralThreshold`, and `PunishmentFee`
pub const GRANULARITY: u128 = 5;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RegisterRequest<AccountId, DateTime> {
    registration_id: H256,
    vault: AccountId,
    timeout: DateTime,
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Trait:
    system::Trait + collateral::Trait + treasury::Trait + exchange_rate_oracle::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as VaultRegistry {
    /// ## Storage
        /// The minimum collateral (DOT) a Vault needs to provide
        /// to participate in the issue process.
        MinimumCollateralVault: DOTBalance<T>;

        /// If a Vault misbehaves in either the redeem or replace protocol by
        /// failing to prove that it sent the correct amount of BTC to the
        /// correct address within the time limit, a vault is punished.
        /// The punishment is the equivalent value of BTC in DOT
        /// (valued at the current exchange rate via `getExchangeRate`) plus a
        /// fixed `PunishmentFee` that is added as a percentage on top
        /// to compensate the damaged party for its loss.
        /// For example, if the `PunishmentFee` is set to 50000,
        /// it is equivalent to 50%.
        PunishmentFee: u128;

        /// If a Vault fails to execute a correct redeem or replace,
        /// it is temporarily banned from further issue, redeem or replace requests.
        PunishmentDelay: T::BlockNumber;

        /// If a Vault is running low on collateral and falls below
        /// `PremiumRedeemThreshold`, users are allocated a premium in DOT
        /// when redeeming with the Vault - as defined by this parameter.
        /// For example, if the RedeemPremiumFee is set to 5000, it is equivalent to 5%.
        RedeemPremiumFee: u128;

        /// Determines the over-collateralization rate for DOT collateral locked
        /// by Vaults, necessary for issuing PolkaBTC. Must to be strictly
        /// greater than 100000 and LiquidationCollateralThreshold.
        SecureCollateralThreshold: u128;

        /// Determines the rate for the collateral rate of Vaults, at which the
        /// BTC backed by the Vault are opened up for auction to other Vaults
        AuctionCollateralThreshold: u128;

        /// Determines the rate for the collateral rate of Vaults,
        /// at which users receive a premium in DOT, allocated from the
        /// Vault’s collateral, when performing a redeem with this Vault.
        /// Must to be strictly greater than 100000 and LiquidationCollateralThreshold.
        PremiumRedeemThreshold: u128;

        /// Determines the lower bound for the collateral rate in PolkaBTC.
        /// Must be strictly greater than 100000. If a Vault’s collateral rate
        /// drops below this, automatic liquidation (forced Redeem) is triggered.
        LiquidationCollateralThreshold: u128;

        /// Account identifier of an artificial Vault maintained by the VaultRegistry
        /// to handle polkaBTC balances and DOT collateral of liquidated Vaults.
        /// That is, when a Vault is liquidated, its balances are transferred to
        /// LiquidationVault and claims are later handled via the LiquidationVault.
        LiquidationVault: T::AccountId;

        /// Mapping of Vaults, using the respective Vault account identifier as key.
        Vaults: map hasher(blake2_128_concat) T::AccountId => Vault<T::AccountId, T::BlockNumber, PolkaBTCBalance<T>>;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;

        fn register_vault(origin, collateral: DOTBalance<T>, btc_address: H160) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::ensure_parachain_running()?;

            ensure!(collateral >= Self::get_minimum_collateral_vault(),
                    Error::InsuficientVaultCollateralAmount);
            ensure!(!Self::vault_exists(&sender), Error::VaultAlreadyRegistered);

            ext::collateral::lock::<T>(&sender, collateral)?;
            let vault = RichVault::<T>::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, &vault);

            Self::deposit_event(Event::<T>::RegisterVault(sender.clone(), collateral));

            Ok(())
        }

        fn lock_additional_collateral(origin, amount: DOTBalance<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::ensure_parachain_running()?;
            let vault = Self::rich_vault_from_id(&sender)?;
            vault.increase_collateral(amount)?;
            Self::deposit_event(Event::<T>::LockAdditionalCollateral(
                sender.clone(),
                amount,
                vault.get_collateral(),
                vault.get_free_collateral()?,
            ));
            Ok(())
        }

        fn withdraw_collateral(origin, amount: DOTBalance<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::ensure_parachain_running()?;
            let vault = Self::rich_vault_from_id(&sender)?;
            vault.withdraw_collateral(amount)?;
            Self::deposit_event(Event::<T>::WithdrawCollateral(
                sender.clone(),
                amount,
                vault.get_collateral(),
            ));
            Ok(())
        }

        pub fn increase_to_be_issued_tokens(origin, tokens: PolkaBTCBalance<T>) -> DispatchResult {
            Self::ensure_parachain_running()?;
            let sender = ensure_signed(origin)?;
            Self::internal_increase_to_be_issued_tokens(&sender, tokens)?;
            Self::deposit_event(Event::<T>::IncreaseToBeIssuedTokens(
                sender.clone(),
                tokens,
            ));
            Ok(())
        }

        pub fn decrease_to_be_issued_tokens(origin, tokens: PolkaBTCBalance<T>) -> DispatchResult {
            Self::ensure_parachain_running()?;
            let sender = ensure_signed(origin)?;
            Self::internal_decrease_to_be_issued_tokens(&sender, tokens)?;
            Self::deposit_event(Event::<T>::DecreaseToBeIssuedTokens(
                sender.clone(),
                tokens,
            ));
            Ok(())
        }

        pub fn issue_tokens(origin, tokens: PolkaBTCBalance<T>) -> DispatchResult {
            Self::ensure_parachain_running()?;
            let sender = ensure_signed(origin)?;
            Self::internal_issue_tokens(&sender, tokens)?;
            Self::deposit_event(Event::<T>::IssueTokens(sender.clone(), tokens));

            Ok(())
        }

        pub fn increase_to_be_redeemed_tokens(origin, tokens: PolkaBTCBalance<T>) -> DispatchResult {
            Self::ensure_parachain_running()?;
            let sender = ensure_signed(origin)?;
            let vault = Self::get_vault_from_id(&sender)?;
            ensure!(vault.issued_tokens - vault.to_be_redeemed_tokens >= tokens,
                    Error::InsufficientTokensCommitted);
            <Vaults<T>>::mutate(&sender, |v| v.to_be_redeemed_tokens += tokens);
            Self::deposit_event(Event::<T>::IncreaseToBeRedeemedTokens(
                sender.clone(),
                tokens,
            ));
            Ok(())
        }

        pub fn decrease_to_be_redeemed_tokens(origin, tokens: PolkaBTCBalance<T>) -> DispatchResult {
            Self::ensure_parachain_running()?;
            let sender = ensure_signed(origin)?;
            let mut vault = Self::rich_vault_from_id(&sender)?;
            vault.decrease_to_be_redeemed(tokens)?;
            Self::deposit_event(Event::<T>::DecreaseToBeRedeemedTokens(
                sender.clone(),
                tokens,
            ));
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Public getters

    pub fn get_vault_from_id(id: &T::AccountId) -> Result<DefaultVault<T>, Error> {
        ensure!(Self::vault_exists(&id), Error::VaultNotFound);
        Ok(<Vaults<T>>::get(id))
    }

    pub fn rich_vault_from_id(id: &T::AccountId) -> Result<RichVault<T>, Error> {
        let vault = Self::get_vault_from_id(id)?;
        Ok(vault.into())
    }

    pub fn vault_exists(id: &T::AccountId) -> bool {
        <Vaults<T>>::contains_key(id)
    }

    pub fn internal_increase_to_be_issued_tokens(
        id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
    ) -> Result<H160, Error> {
        let mut vault = Self::rich_vault_from_id(&id)?;
        vault.increase_to_be_issued(tokens)?;
        Ok(vault.data.btc_address)
    }

    pub fn internal_decrease_to_be_issued_tokens(
        id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
    ) -> Result<(), Error> {
        let vault = Self::rich_vault_from_id(&id)?;
        ensure!(
            vault.data.to_be_issued_tokens >= tokens,
            Error::InsufficientTokensCommitted
        );

        <Vaults<T>>::mutate(id, |v| v.to_be_issued_tokens -= tokens);
        Ok(())
    }

    pub fn decrease_tokens(
        vault_id: &T::AccountId,
        user_id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
        _collateral: DOTBalance<T>,
    ) -> Result<(), Error> {
        let mut vault: RichVault<T> = Self::rich_vault_from_id(&vault_id)?;
        vault.decrease_to_be_redeemed(tokens)?;
        vault.decrease_issued(tokens)?;

        let to_slash = ext::oracle::btc_to_dots::<T>(tokens)?;

        // FIXME: find how to work with balance types
        // if vault.get_collateral() - to_slash < <SecureCollateralThreshold>::get() {
        //     to_slash = <SecureCollateralThreshold>::get();
        // }

        // TODO: add punishment fee
        ext::collateral::slash::<T>(&vault_id, &user_id, to_slash)?;
        Ok(())
    }

    /// Private getters and setters
    pub fn internal_issue_tokens(
        id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
    ) -> Result<(), Error> {
        Self::internal_decrease_to_be_issued_tokens(id, tokens)?;
        <Vaults<T>>::mutate(id, |v| v.issued_tokens += tokens);
        Ok(())
    }

    fn get_minimum_collateral_vault() -> DOTBalance<T> {
        <MinimumCollateralVault<T>>::get()
    }

    pub fn insert_vault<V: Into<DefaultVault<T>>>(id: &T::AccountId, rich_vault: V) {
        let vault: DefaultVault<T> = rich_vault.into();
        <Vaults<T>>::insert(id, vault)
    }

    /// Other helpers
    /// Returns an error if the parachain is not in running state
    fn ensure_parachain_running() -> Result<(), Error> {
        // TODO: integrate security module
        // ensure!(
        //     !<security::Module<T>>::check_parachain_status(
        //         StatusCode::Shutdown),
        //     Error::Shutdown
        // );
        Ok(())
    }
}

decl_event! {
    /// ## Events
    pub enum Event<T> where
            AccountId = <T as system::Trait>::AccountId,
            Balance = DOTBalance<T>,
            BTCBalance = PolkaBTCBalance<T> {
        RegisterVault(AccountId, Balance),
        /// id, new collateral, total collateral, free collateral
        LockAdditionalCollateral(AccountId, Balance, Balance, Balance),
        /// id, withdrawn collateral, total collateral
        WithdrawCollateral(AccountId, Balance, Balance),

        IncreaseToBeIssuedTokens(AccountId, BTCBalance),
        DecreaseToBeIssuedTokens(AccountId, BTCBalance),
        IssueTokens(AccountId, BTCBalance),
        IncreaseToBeRedeemedTokens(AccountId, BTCBalance),
        DecreaseToBeRedeemedTokens(AccountId, BTCBalance),

    }
}
