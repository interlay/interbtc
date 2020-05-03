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
use security::StatusCode;
use sha2::{Digest, Sha256};
use sp_core::H160;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
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
        fn execute_redeem(origin, vault_id: T::AccountId, redeem_id: H256, tx_id: H256Le, tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_execute_redeem(requester, vault_id, redeem_id, tx_id, tx_block_height, merkle_proof, raw_tx)?;
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
        redeemer: T::AccountId,
        amount_polka_btc: PolkaBTC<T>,
        btc_address: H160,
        vault_id: T::AccountId,
    ) -> Result<H256, Error> {
        // TODO: check preconditions
        // Step 1: Check if the amountPolkaBTC is less or equal to the user’s balance in the treasury
        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(
            amount_polka_btc <= redeemer_balance,
            Error::AmountExceedsUserBalance
        );
        // Step 2: Retrieve the vault from Vault Registry
        let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        // Step 3: Check that the vault is currently not banned,
        let height = <system::Module<T>>::block_number();
        vault.ensure_not_banned(height)?;
        // Step 4: Check if the amountPolkaBTC is less or equal to the issuedTokens by the selected vault in the VaultRegistry
        ensure!(
            amount_polka_btc <= vault.issued_tokens,
            Error::AmountExceedsVaultBalanace
        );
        // Step 5: Check if ParachainState in Security is ERROR with LIQUIDATION in Errors
        let (amount_btc, amount_dot): (u128, u128) =
            if <security::Module<T>>::check_parachain_status(StatusCode::Error) {
                let btcdot_rate: u128 = <exchange_rate_oracle::Module<T>>::get_exchange_rate()?;
                let raw_amount_polka_btc = Self::btc_to_u128(amount_polka_btc)?;
                let amount_dot_in_btc: u128 =
                    raw_amount_polka_btc * (Self::_partial_redeem_factor()? / 100_000u128);
                let amount_btc: u128 = raw_amount_polka_btc - amount_dot_in_btc;
                let amount_dot: u128 = amount_dot_in_btc * btcdot_rate;
                (
                    amount_btc,
                    amount_dot.try_into().map_err(|_e| Error::RuntimeError)?,
                )
            } else {
                (Self::btc_to_u128(amount_polka_btc)?, 0)
            };
        // Step 6: Call the Vault Registry increaseToBeRedeemedTokens function with the amountBTC of tokens to be redeemed and the vault identified by its address.
        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(
            &vault_id,
            amount_btc.try_into().map_err(|_e| Error::RuntimeError)?,
        )?;
        // Step 7: If amountDOT > 0, call redeemTokensLiquidation in Vault Registry. This allocates the user amountDOT using the LiquidationVault’s collateral and updates the LiquidationVault’s polkaBTC balances.
        if amount_dot > 0 {
            <vault_registry::Module<T>>::_redeem_tokens_liquidation(&vault_id, amount_polka_btc)?;
        }
        // Step 8: Call the lock function in the Treasury to lock the PolkaBTC amount of the user.
        ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
        // Step 9: Generate a redeemId using generateSecureId, passing redeemer as parameter
        let key = Self::gen_redeem_key(redeemer.clone()); //TODO(jaupe) call generate_secure_id
                                                          // Step 10: Check if the Vault’s collateral rate is below PremiumRedeemThreshold.
        let premium_redeem_threshold: u128 =
            <vault_registry::Module<T>>::_premium_redeem_threshold();
        let premium_dot = if amount_dot > premium_redeem_threshold {
            // Todo(jaupe) verify if this is correct logic
            <vault_registry::Module<T>>::_redeem_premium_fee()
        } else {
            0
        };
        // Step 11: Store a new Redeem struct in the RedeemRequests mapping
        let premium_dot: DOT<T> = premium_dot.try_into().map_err(|_e| Error::RuntimeError)?; //use std::convert::TryInto;
        Self::insert_redeem_request(
            key,
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
        // Step 12: Emit the RequestRedeem event
        Self::deposit_event(<Event<T>>::RequestRedeem(
            key,
            redeemer,
            amount_polka_btc,
            vault_id,
            btc_address,
        ));
        Ok(key)
    }

    fn _partial_redeem_factor() -> Result<u128, Error> {
        // Step 1: Get the current exchange rate (exchangeRate) using getExchangeRate.
        // Step 2: Calculate totalLiquidationValue
        let total_liquidation_value = <vault_registry::Module<T>>::total_liquidation_value()?;
        let total_supply = Self::btc_to_u128(<treasury::Module<T>>::get_total_supply())?;
        Ok(total_liquidation_value / total_supply)
    }

    fn gen_redeem_key(redeemer: T::AccountId) -> H256 {
        let mut hasher = Sha256::default();
        hasher.input(redeemer.encode());
        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        H256(result)
    }

    fn btc_to_u128(amount: PolkaBTC<T>) -> Result<u128, Error> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::RuntimeError)
    }

    /// Completes CBA issuance, removing request from storage and minting token.
    fn _execute_redeem(
        vault_id: T::AccountId,
        redeemer: T::AccountId,
        redeem_id: H256,
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        // Step 2: Check if the redeemId exists. Return ERR_REDEEM_ID_NOT_FOUND if not found.
        let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
        ensure!(redeemer == redeem.redeemer, Error::UnauthorizedUser);
        // Step 1: Check if the vault is the redeem.vault
        ensure!(vault_id == redeem.vault, Error::UnauthorizedUser);
        // Step 3: Check if redeem period has expired
        let height = <system::Module<T>>::block_number();
        let period = <RedeemPeriod<T>>::get();
        ensure!(
            period < height && redeem.opentime < height - period,
            Error::CommitPeriodExpired
        );
        // Step 4: Verify the transaction
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
        // Step 5: Burn the tokens
        ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;
        // Step 6: Check if premium tokens need to be redeemed
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
        // Step 7: Remove redeem from RedeemRequests
        <RedeemRequests<T>>::remove(redeem_id);
        // Step 8: Emit an ExecuteRedeem event
        Self::deposit_event(<Event<T>>::ExecuteRedeem(redeem_id, redeemer, redeem.vault));
        Ok(())
    }

    /// Cancels CBA issuance if time has expired and slashes collateral.
    fn _cancel_redeem(
        redeemer: T::AccountId,
        redeem_id: H256,
        reimburse: bool,
    ) -> Result<(), Error> {
        // Step 1: Check if an redeem with id redeemId exists
        let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
        // Step 2: Check if the expiry time of the redeem request is up
        let height = <system::Module<T>>::block_number();
        let period = <RedeemPeriod<T>>::get();
        ensure!(redeem.opentime + period > height, Error::TimeNotExpired);
        // Step 3: Retrieve the current BTC-DOT exchange rate
        let btcdot_spot_rate = <exchange_rate_oracle::Module<T>>::get_exchange_rate()?;
        // Step 4: Handle reimbursement in DOT
        let punishment_fee = <vault_registry::Module<T>>::_punishment_fee();
        let raw_amount_polka = Self::btc_to_u128(redeem.amount_polka_btc)?;
        if reimburse {
            ext::vault_registry::_decrease_tokens::<T>(
                &redeem.vault,
                &redeem.redeemer,
                redeem.amount_polka_btc,
            )?;
            ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;
            let reimburse_in_btc = raw_amount_polka
                .checked_mul(btcdot_spot_rate * (1 + punishment_fee / 100000))
                .ok_or(Error::RuntimeError)?;
            let reimburse_amount: DOT<T> = reimburse_in_btc
                .try_into()
                .map_err(|_| Error::RuntimeError)?;
            ext::collateral::slash_collateral::<T>(
                &redeem.redeemer,
                &redeem.vault,
                reimburse_amount,
            )?;
        } else {
            let slash_in_btc = raw_amount_polka
                .checked_mul(btcdot_spot_rate * (1 + punishment_fee / 100000))
                .ok_or(Error::RuntimeError)?; //Todo(jaupe) i think it should should be
            let slash_amount: DOT<T> = slash_in_btc.try_into().map_err(|_| Error::RuntimeError)?;
            ext::collateral::slash_collateral::<T>(&redeem.redeemer, &redeem.vault, slash_amount)?;
        }
        // Step 5: Temporarily Ban the Vault from issue, redeem and replace
        <vault_registry::Module<T>>::_ban_vault(redeem.vault, height); //TODO(jaupe) make this call return an error if its not found or updated
                                                                       // Step 6: Remove redeem from RedeemRequests
        <RedeemRequests<T>>::remove(redeem_id);
        // Step 7: Emit a CancelRedeem event with the redeemer account identifier and the redeemId
        Self::deposit_event(<Event<T>>::CancelRedeem(redeem_id, redeemer));
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
