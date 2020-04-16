#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

/// # Vault Registry implementation
/// This is the implementation of the Vault Registry following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage /*, ensure */};
// use std::time::SystemTime;
// use system::ensure_signed;
// use frame_support::dispatch::DispatchResult;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use node_primitives::{AccountId, BlockNumber};
use sp_core::H160;

use xclaim_core::Error;

type DOT<T> = <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;
type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Trait: system::Trait + collateral::Trait + treasury::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Granularity of `SecureCollateralThreshold`, `AuctionCollateralThreshold`,
/// `LiquidationCollateralThreshold`, and `PunishmentFee`
pub const GRANULARITY: u128 = 5;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<PolkaBTC, DOT> {
    // Account identifier of the Vault
    vault: AccountId,
    // Number of PolkaBTC tokens pending issue
    to_be_issued_tokens: PolkaBTC,
    // Number of issued PolkaBTC tokens
    issued_tokens: PolkaBTC,
    // Number of PolkaBTC tokens pending redeem
    to_be_redeemed_tokens: PolkaBTC,
    // DOT collateral locked by this Vault
    collateral: DOT,
    // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    btc_address: H160,
    // Block height until which this Vault is banned from being
    // used for Issue, Redeem (except during automatic liquidation) and Replace .
    banned_until: BlockNumber,
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as VaultRegistry {
    /// ## Storage
        /// The minimum collateral (DOT) a Vault needs to provide
        /// to participate in the issue process.
        MinimumCollateralVault: DOT<T>;

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
        Vaults: map hasher(blake2_128_concat) AccountId => Vault<PolkaBTC<T>, DOT<T>>;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Public getters

    pub fn get_vault_from_id(id: AccountId) -> Vault<PolkaBTC<T>, DOT<T>> {
        <Vaults<T>>::get(id)
    }

    pub fn mutate_vault_from_id(id: AccountId, vault: Vault<PolkaBTC<T>, DOT<T>>) {
        <Vaults<T>>::mutate(id, |v| *v = vault)
    }

    pub fn insert_vault(id: AccountId, vault: Vault<PolkaBTC<T>, DOT<T>>) {
        <Vaults<T>>::insert(id, vault)
    }

    pub fn increase_to_be_issued_tokens(
        id: &AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<H160, Error> {
        <Vaults<T>>::mutate(id, |v| v.to_be_issued_tokens += tokens);
        Ok(<Vaults<T>>::get(id).btc_address)
    }

    pub fn decrease_to_be_issued_tokens(id: &AccountId, tokens: PolkaBTC<T>) -> Result<(), Error> {
        <Vaults<T>>::mutate(id, |v| v.to_be_issued_tokens -= tokens);
        Ok(())
    }

    pub fn issue_tokens(id: &AccountId, tokens: PolkaBTC<T>) -> Result<(), Error> {
        Self::decrease_to_be_issued_tokens(id, tokens)?;
        <Vaults<T>>::mutate(id, |v| v.issued_tokens += tokens);
        Ok(())
    }

    /// Private getters and setters

    /// Other helpers
    /// Returns an error if the parachain is not in running state
    fn _ensure_parachain_running() -> Result<(), Error> {
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
            Balance = PolkaBTC<T> {
        RegisterVault(AccountId, Balance),
    }
}
