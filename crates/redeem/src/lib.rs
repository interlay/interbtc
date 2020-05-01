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

use crate::types::{PolkaBTC, DOT};
use bitcoin::types::H256Le;
use codec::{Decode, Encode};
/// # PolkaBTC Redeem implementation
/// The Redeem module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/redeem.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::H160;
use sp_runtime::{ModuleId, Zero};
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

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Redeem<AccountId, BlockNumber, PolkaBTC, DOT> {
    vault: AccountId,
    opentime: BlockNumber,
    amount_polka_btc: PolkaBTC,
    amount_btc: PolkaBTC,
    amount_dot: DOT,
    premium_dot: DOT,
    redeemer: AccountId,
    btc_address: H160,
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Redeem {
        RedeemPeriod: T::BlockNumber;
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

        /// Request the issuance of PolkaBTC
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount_polka_btc` - amount of PolkaBTC
        /// * `btc_address` - the address to receive BTC
        /// * `vault` - address of the vault
        fn request_redeem(origin, amount: PolkaBTC<T>, btc_address: H160, vault: T::AccountId)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_request_redeem(requester, amount, btc_address, vault)?;
            Ok(())
        }

        /// Finalize the issuance of PolkaBTC
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        /// * `tx_id` - transaction hash
        /// * `tx_block_height` - block number of backing chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        fn execute_redeem(origin, redeem_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_execute_redeem(requester, redeem_id, tx_id, tx_block_height, merkle_proof, raw_tx)?;
            Ok(())
        }

        /// Cancel the issuance of PolkaBTC if expired
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
            let requester = ensure_signed(origin)?;
            Self::_cancel_redeem(requester, redeem_id, reimburse)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Requests CbA redeem, returns unique tracking ID.
    fn _request_redeem(
        requester: T::AccountId,
        amount_polka_btc: PolkaBTC<T>,
        btc_address: H160,
        vault_id: T::AccountId,
    ) -> Result<H256, Error> {
        // TODO: check precondition
        let height = <system::Module<T>>::block_number();

        // check if the user has enough funds
        let redeemer_balance = ext::treasury::get_balance(requester);
        ensure!(redeemer_balance >= amount_polka_btc,
            Error::AmountExceedsUserBalance);

        // check if the vault is not banned
        let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        match vault.banned_until {
            Some(until) => ensure!(until < height, Error::VaultBanned),
            None => (),
        };

        // check if the vault has enough tokens
        ensure!(vault.issued_tokens >= amount_polka_btc,
            Error::AmountExceedsVaultBalanace);

        // FIXME: check if parachain status is in liquidation
        // if not liquidation
        let amount_btc = amount_polka_btc;
        let amount_dot = Zero::zero();

        // increase to be redeemed tokens
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(
            &vault_id,
            amount_btc
        )?;


        if !amount_dot.is_zero() {
            //FIXME: call vault registry redeem tokens liquidation
        }

        ext::collateral::lock_collateral::<T>(&requester, griefing_collateral)?;


        let mut hasher = Sha256::default();
        // TODO: nonce from security module
        hasher.input(requester.encode());

        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        let key = H256(result);

        Self::insert_redeem_request(
            key,
            Redeem {
                vault: vault_id.clone(),
                opentime: height,
                griefing_collateral: griefing_collateral,
                amount: amount,
                requester: requester.clone(),
                btc_address: btc_address,
                completed: false,
            },
        );

        Self::deposit_event(<Event<T>>::RequestRedeem(
            key,
            requester,
            amount,
            vault_id,
            btc_address,
        ));
        Ok(key)
    }

    /// Completes CBA issuance, removing request from storage and minting token.
    fn _execute_redeem(
        requester: T::AccountId,
        redeem_id: H256,
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        // TODO: check precondition
        let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
        ensure!(requester == redeem.requester, Error::UnauthorizedUser);

        let height = <system::Module<T>>::block_number();
        let period = <RedeemPeriod<T>>::get();
        ensure!(
            period < height && redeem.opentime < height - period,
            Error::CommitPeriodExpired
        );

        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, tx_block_height, merkle_proof)?;
        // TODO: redeem.amount
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            0,
            redeem.btc_address.as_bytes().to_vec(),
            redeem_id.clone().as_bytes().to_vec(),
        )?;

        ext::vault_registry::redeem_tokens::<T>(&redeem.vault, redeem.amount)?;
        ext::treasury::mint::<T>(redeem.requester, redeem.amount);
        <RedeemRequests<T>>::remove(redeem_id);

        Self::deposit_event(<Event<T>>::ExecuteRedeem(redeem_id, requester, redeem.vault));
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_redeem(requester: T::AccountId, redeem_id: H256) -> Result<(), Error> {
        let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
        let height = <system::Module<T>>::block_number();
        let period = <RedeemPeriod<T>>::get();

        ensure!(redeem.opentime + period > height, Error::TimeNotExpired);
        ensure!(!redeem.completed, Error::RedeemCompleted);

        ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&redeem.vault, redeem.amount)?;
        ext::collateral::slash_collateral::<T>(
            &redeem.requester,
            &redeem.vault,
            redeem.griefing_collateral,
        )?;

        Self::deposit_event(<Event<T>>::CancelRedeem(redeem_id, requester));
        Ok(())
    }

    fn get_redeem_request_from_id(
        redeem_id: &H256,
    ) -> Result<Redeem<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        ensure!(
            <RedeemRequests<T>>::contains_key(*redeem_id),
            Error::RedeemIdNotFound
        );
        Ok(<RedeemRequests<T>>::get(*redeem_id))
    }

    fn insert_redeem_request(
        key: H256,
        value: Redeem<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <RedeemRequests<T>>::insert(key, value)
    }

    #[allow(dead_code)]
    fn set_redeem_period(value: T::BlockNumber) {
        <RedeemPeriod<T>>::set(value);
    }
}
