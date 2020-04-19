#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

/// # Vault Registry implementation
/// This is the implementation of the Vault Registry following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, HasCompact};
use frame_support::dispatch::DispatchResult;
use frame_support::traits::Currency;
use frame_support::{decl_event, decl_module, decl_storage /*, ensure */};
use primitive_types::H256;
use sp_core::H160;
use system::ensure_signed;

use x_core::Error;

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Trait: system::Trait + collateral::Trait + treasury::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Granularity of `SecureCollateralThreshold`, `AuctionCollateralThreshold`,
/// `LiquidationCollateralThreshold`, and `PunishmentFee`
pub const GRANULARITY: u128 = 5;

type DOT<T> = <T as collateral::Trait>::DOT;
type DOTBalance<T> = <DOT<T> as Currency<<T as system::Trait>::AccountId>>::Balance;

type PolkaBTC<T> = <T as treasury::Trait>::PolkaBTC;
type PolkaBTCBalance<T> = <PolkaBTC<T> as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, PolkaBTCBalance: HasCompact, DOTBalance: HasCompact> {
    // Account identifier of the Vault
    pub id: AccountId,
    // Number of PolkaBTC tokens pending issue
    pub to_be_issued_tokens: PolkaBTCBalance,
    // Number of issued PolkaBTC tokens
    pub issued_tokens: PolkaBTCBalance,
    // Number of PolkaBTC tokens pending redeem
    pub to_be_redeemed_tokens: PolkaBTCBalance,
    // DOT collateral locked by this Vault
    pub collateral: DOTBalance,
    // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub btc_address: H160,
    // Block height until which this Vault is banned from being
    // used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
}

impl<AccountId, BlockNumber, PolkaBTCBalance: HasCompact + Default, DOTBalance: HasCompact>
    Vault<AccountId, BlockNumber, PolkaBTCBalance, DOTBalance>
{
    fn new(
        id: AccountId,
        collateral: DOTBalance,
        btc_address: H160,
    ) -> Vault<AccountId, BlockNumber, PolkaBTCBalance, DOTBalance> {
        Vault {
            id,
            collateral,
            btc_address,
            to_be_issued_tokens: Default::default(),
            issued_tokens: Default::default(),
            to_be_redeemed_tokens: Default::default(),
            banned_until: None,
        }
    }
}

type DefaultVault<T> = Vault<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
    PolkaBTCBalance<T>,
    DOTBalance<T>,
>;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RegisterRequest<AccountId, DateTime> {
    registration_id: H256,
    vault: AccountId,
    timeout: DateTime,
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
        Vaults: map hasher(blake2_128_concat) T::AccountId => Vault<T::AccountId, T::BlockNumber, PolkaBTCBalance<T>, DOTBalance<T>>;
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
            if collateral < Self::get_minimum_collateral_vault() {
                return Err(Error::InsuficientVaultCollateralAmount.into());
            }
            if Self::vault_exists(&sender) {
                return Err(Error::VaultAlreadyRegistered.into());
            }
            let vault: DefaultVault<T> = Vault::new(sender.clone(), collateral, btc_address);
            Self::insert_vault(sender.clone(), vault);

            Self::deposit_event(Event::<T>::RegisterVault(sender.clone(), collateral));

            Ok(())
        }

        fn lock_additional_collateral(origin, amount: DOTBalance<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::ensure_parachain_running()?;
            Self::increase_collateral(&sender, amount)?;
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Public getters

    pub fn get_vault_from_id(id: &T::AccountId) -> Result<DefaultVault<T>, Error> {
        if Self::vault_exists(&id) {
            Ok(<Vaults<T>>::get(id))
        } else {
            Err(Error::VaultNotFound)
        }
    }

    pub fn vault_exists(id: &T::AccountId) -> bool {
        <Vaults<T>>::contains_key(id)
    }

    /// Private getters and setters
    fn _mutate_vault_from_id(id: T::AccountId, vault: DefaultVault<T>) {
        <Vaults<T>>::mutate(id, |v| *v = vault)
    }

    fn increase_collateral(id: &T::AccountId, collateral: DOTBalance<T>) -> Result<(), Error> {
        if !Self::vault_exists(id) {
            return Err(Error::VaultNotFound);
        }
        Ok(<Vaults<T>>::mutate(id, |v| v.collateral += collateral))
    }

    pub fn increase_to_be_issued_tokens(
        id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
    ) -> Result<H160, Error> {
        <Vaults<T>>::mutate(id.clone(), |v| v.to_be_issued_tokens += tokens);
        Ok(<Vaults<T>>::get(id).btc_address)
    }

    pub fn decrease_to_be_issued_tokens(
        id: &T::AccountId,
        tokens: PolkaBTCBalance<T>,
    ) -> Result<(), Error> {
        <Vaults<T>>::mutate(id, |v| v.to_be_issued_tokens -= tokens);
        Ok(())
    }

    pub fn issue_tokens(id: &T::AccountId, tokens: PolkaBTCBalance<T>) -> Result<(), Error> {
        Self::decrease_to_be_issued_tokens(id, tokens)?;
        <Vaults<T>>::mutate(id, |v| v.issued_tokens += tokens);
        Ok(())
    }

    fn get_minimum_collateral_vault() -> DOTBalance<T> {
        <MinimumCollateralVault<T>>::get()
    }

    pub fn insert_vault(id: T::AccountId, vault: DefaultVault<T>) {
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
            Balance = DOTBalance<T> {
        RegisterVault(AccountId, Balance),
    }
}
