#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
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

use crate::types::{PolkaBTC, Redeem, DOT};
use bitcoin::types::H256Le;
/// # PolkaBTC Redeem implementation
/// The Redeem module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/redeem.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use primitive_types::H256;
use security::{ErrorCode, StatusCode};
use sp_core::H160;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use system::ensure_signed;
use x_core::Error;

/// The redeem module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"i/redeem");

/// The pallet's configuration trait.
pub trait Trait:
    system::Trait + vault_registry::Trait + collateral::Trait + btc_relay::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Redeem {
        /// The time difference in number of blocks between a redeem request is created and required completion time by a vault.
        /// The redeem period has an upper limit to ensure the user gets their BTC in time and to potentially punish a vault for inactivity or stealing BTC.
        RedeemPeriod get(fn redeem_period): T::BlockNumber;

        /// Users create redeem requests to receive BTC in return for PolkaBTC.
        /// This mapping provides access from a unique hash redeemId to a Redeem struct.
        RedeemRequests: map hasher(blake2_128_concat) H256 => Redeem<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
    {
        RequestRedeem(H256, AccountId, PolkaBTC, AccountId, H160),
        ExecuteRedeem(H256, AccountId, AccountId),
        CancelRedeem(H256, AccountId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// A user requests to start the redeem procedure. This function checks the BTC Parachain
        /// status in Security and decides how the Redeem process is to be executed.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `btc_address` - the address to receive BTC
        /// * `vault` - address of the vault
        fn request_redeem(origin, amount_polka_btc: PolkaBTC<T>, btc_address: H160, vault_id: T::AccountId)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;

            Self::ensure_parachain_running_or_error_liquidated()?;

            let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
            ensure!(
                amount_polka_btc <= redeemer_balance,
                Error::AmountExceedsUserBalance
            );
            let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
            let height = <system::Module<T>>::block_number();
            vault.ensure_not_banned(height)?;
            ensure!(
                amount_polka_btc <= vault.issued_tokens,
                Error::AmountExceedsVaultBalance
            );
            let (amount_btc, amount_dot): (u128, u128) =
                if ext::security::get_parachain_status::<T>() == StatusCode::Error {
                    let btcdot_rate: u128 = ext::oracle::get_exchange_rate::<T>()?;
                    let raw_amount_polka_btc = Self::btc_to_u128(amount_polka_btc)?;
                    let amount_dot_in_btc: u128 = raw_amount_polka_btc
                        .checked_mul(Self::get_partial_redeem_factor()?).ok_or(Error::RuntimeError)?
                        .checked_div(100_000).ok_or(Error::RuntimeError)?;
                    let amount_btc: u128 = raw_amount_polka_btc - amount_dot_in_btc;
                    let amount_dot: u128 = amount_dot_in_btc * btcdot_rate;
                    (
                        amount_btc,
                        amount_dot.try_into().map_err(|_e| Error::RuntimeError)?,
                    )
                } else {
                    (Self::btc_to_u128(amount_polka_btc)?, 0)
                };
            ext::vault_registry::increase_to_be_redeemed_tokens::<T>(
                &vault_id,
                amount_btc.try_into().map_err(|_e| Error::RuntimeError)?,
            )?;
            if amount_dot > 0 {
                ext::vault_registry::redeem_tokens_liquidation::<T>(
                    &vault_id,
                    amount_dot.try_into().map_err(|_e| Error::RuntimeError)?,
                )?;
            }
            ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
            let redeem_id = ext::security::get_secure_id::<T>(&redeemer);
            let premium_redeem_threshold: u128 =
                <vault_registry::Module<T>>::_premium_redeem_threshold();
            let premium_dot = if amount_dot > premium_redeem_threshold {
                // FIXME: check if the vault is above the PremiumRedeemThreshold
                <vault_registry::Module<T>>::_redeem_premium_fee()
            } else {
                0
            };
            let premium_dot: DOT<T> = premium_dot.try_into().map_err(|_e| Error::RuntimeError)?;
            Self::insert_redeem_request(
                redeem_id,
                Redeem {
                    vault: vault_id.clone(),
                    opentime: height,
                    amount_polka_btc,
                    amount_btc: amount_btc.try_into().map_err(|_e| Error::RuntimeError)?,
                    amount_dot: amount_dot.try_into().map_err(|_e| Error::RuntimeError)?,
                    premium_dot,
                    redeemer: redeemer.clone(),
                    btc_address,
                },
            );
            Self::deposit_event(<Event<T>>::RequestRedeem(
                redeem_id,
                redeemer,
                amount_polka_btc,
                vault_id,
                btc_address,
            ));

            Ok(())
        }

        /// A Vault calls this function after receiving an RequestRedeem event with their public key.
        /// Before calling the function, the Vault transfers the specific amount of BTC to the BTC address
        /// given in the original redeem request. The Vault completes the redeem with this function.
        ///
        /// # Arguments
        ///
        /// * `origin` - the vault responsible for executing this redeem request
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        /// * `tx_id` - transaction hash
        /// * `tx_block_height` - block number of backing chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        fn execute_redeem(origin, redeem_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let vault_id = ensure_signed(origin)?;

            Self::ensure_parachain_running()?;

            let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
            ensure!(vault_id == redeem.vault, Error::UnauthorizedVault);
            let height = <system::Module<T>>::block_number();
            let period = Self::redeem_period();
            ensure!(
                period < height && redeem.opentime < height - period,
                Error::CommitPeriodExpired
            );
            let amount: usize = redeem
                .amount_btc
                .try_into()
                .map_err(|_e| Error::RuntimeError)?;
            ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, tx_block_height, merkle_proof)?;
            ext::btc_relay::validate_transaction::<T>(
                raw_tx,
                amount as i64,
                redeem.btc_address.as_bytes().to_vec(),
                redeem_id.clone().as_bytes().to_vec(),
            )?;
            ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;
            if redeem.premium_dot > 0.into() {
                ext::vault_registry::redeem_tokens_premium::<T>(
                    &redeem.vault,
                    redeem.amount_polka_btc,
                    redeem.premium_dot,
                    &redeem.redeemer,
                )?;
            } else {
                ext::vault_registry::redeem_tokens::<T>(&redeem.vault, redeem.amount_polka_btc)?;
            }
            <RedeemRequests<T>>::remove(redeem_id);
            Self::deposit_event(<Event<T>>::ExecuteRedeem(
                redeem_id,
                redeem.redeemer,
                redeem.vault,
            ));
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
        /// * `reimburse` - specifying if the user wishes to be reimbursed in DOT
        /// and slash the Vault, or wishes to keep the PolkaBTC (and retry
        /// Redeem with another Vault)
        fn cancel_redeem(origin, redeem_id: H256, reimburse: bool)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            let redeem = Self::get_redeem_request_from_id(&redeem_id)?;

            let height = <system::Module<T>>::block_number();
            let period = Self::redeem_period();
            ensure!(redeem.opentime + period > height, Error::TimeNotExpired);

            let btcdot_spot_rate = ext::oracle::get_exchange_rate::<T>()?;
            let punishment_fee = ext::vault_registry::punishment_fee::<T>();
            let raw_amount_polka_btc = Self::btc_to_u128(redeem.amount_polka_btc)?;
            if reimburse {
                ext::vault_registry::decrease_tokens::<T>(
                    &redeem.vault,
                    &redeem.redeemer,
                    redeem.amount_polka_btc,
                )?;
                ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;
                let reimburse_in_btc = raw_amount_polka_btc
                    .checked_mul(btcdot_spot_rate * (100_000 + punishment_fee)).ok_or(Error::RuntimeError)?
                    .checked_div(100_000).ok_or(Error::RuntimeError)?;
                let reimburse_amount: DOT<T> = reimburse_in_btc
                    .try_into()
                    .map_err(|_| Error::RuntimeError)?;
                ext::collateral::slash_collateral::<T>(
                    &redeem.redeemer,
                    &redeem.vault,
                    reimburse_amount,
                )?;
            } else {
                let slash_in_btc = raw_amount_polka_btc
                    .checked_mul(btcdot_spot_rate * punishment_fee).ok_or(Error::RuntimeError)?
                    .checked_div(100_000).ok_or(Error::RuntimeError)?;
                let slash_amount: DOT<T> = slash_in_btc.try_into().map_err(|_| Error::RuntimeError)?;
                ext::collateral::slash_collateral::<T>(&redeem.redeemer, &redeem.vault, slash_amount)?;
            }
            ext::vault_registry::ban_vault::<T>(redeem.vault, height);
            <RedeemRequests<T>>::remove(redeem_id);
            Self::deposit_event(<Event<T>>::CancelRedeem(redeem_id, redeemer));

            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Insert a new redeem request into state.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    /// * `value` - the redeem request
    fn insert_redeem_request(
        key: H256,
        value: Redeem<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <RedeemRequests<T>>::insert(key, value)
    }

    /// Fetch a pre-existing redeem request or throw.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    fn get_redeem_request_from_id(
        key: &H256,
    ) -> Result<Redeem<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        ensure!(
            <RedeemRequests<T>>::contains_key(*key),
            Error::RedeemIdNotFound
        );
        Ok(<RedeemRequests<T>>::get(*key))
    }

    /// Ensure that the security module reports the parachain is healthy.
    fn ensure_parachain_running() -> Result<(), Error> {
        ext::security::ensure_parachain_status_running::<T>()
    }

    /// Ensure that the parachain is running or vault is liquidated.
    fn ensure_parachain_running_or_error_liquidated() -> Result<(), Error> {
        let status_code = ext::security::get_parachain_status::<T>();
        ensure!(
            status_code == StatusCode::Running || status_code == StatusCode::Error,
            Error::ParachainNotRunningOrLiquidation,
        );
        let errors = ext::security::get_errors::<T>();
        if status_code == StatusCode::Error {
            ensure!(
                errors.len() == 1 && errors.contains(&ErrorCode::Liquidation),
                Error::ParachainNotRunningOrLiquidation,
            );
        }
        Ok(())
    }

    /// Calculates the fraction of BTC to be redeemed in DOT when the
    /// BTC Parachain state is in ERROR state due to a LIQUIDATION error.
    fn get_partial_redeem_factor() -> Result<u128, Error> {
        let total_liquidation_value = ext::vault_registry::total_liquidation_value::<T>()?;
        let total_supply = Self::btc_to_u128(ext::treasury::get_total_supply::<T>())?;
        Ok(total_liquidation_value / total_supply)
    }

    fn btc_to_u128(amount: PolkaBTC<T>) -> Result<u128, Error> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::RuntimeError)
    }
}
