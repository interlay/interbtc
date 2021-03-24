//! # PolkaBTC Redeem Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/redeem.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

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

use crate::types::{PolkaBTC, RedeemRequestV1, Version, DOT};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use primitive_types::H256;
use sp_runtime::{traits::*, ModuleId};
use sp_std::{convert::TryInto, vec::Vec};
use vault_registry::CurrencySource;

/// The redeem module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"i/redeem");

pub trait WeightInfo {
    fn request_redeem() -> Weight;
    fn liquidation_redeem() -> Weight;
    fn execute_redeem() -> Weight;
    fn cancel_redeem() -> Weight;
    fn set_redeem_period() -> Weight;
    fn mint_tokens_for_reimbursed_redeem() -> Weight;
}

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + vault_registry::Config
    + collateral::Config
    + btc_relay::Config
    + treasury::Config
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

        /// Users create redeem requests to receive BTC in return for PolkaBTC.
        /// This mapping provides access from a unique hash redeemId to a Redeem struct.
        RedeemRequests: map hasher(blake2_128_concat) H256 => RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>;

        /// The minimum amount of btc that is accepted for redeem requests; any lower values would
        /// risk the bitcoin client to reject the payment
        RedeemBtcDustValue get(fn redeem_btc_dust_value) config(): PolkaBTC<T>;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V2): Version = Version::V0;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        PolkaBTC = PolkaBTC<T>,
        DOT = DOT<T>,
    {
        RequestRedeem(
            H256,       // redeem_id
            AccountId,  // redeemer
            PolkaBTC,   // redeem_amount_polka_btc
            PolkaBTC,   // fee_polka_btc
            DOT,        // premium_dot
            AccountId,  // vault_id
            BtcAddress, // user btc_address
        ),
        // [redeemer, amount_polka_btc]
        LiquidationRedeem(AccountId, PolkaBTC),
        // [redeem_id, redeemer, amount_polka_btc, fee_polka_btc, vault]
        ExecuteRedeem(H256, AccountId, PolkaBTC, PolkaBTC, AccountId),
        // [redeem_id, redeemer, vault_id, slashing_amount_in_dot, reimburse]
        CancelRedeem(H256, AccountId, AccountId, DOT, bool),
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

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            use frame_support::{migration::StorageKeyIterator, Blake2_128Concat};

            if matches!(Self::storage_version(), Version::V0 | Version::V1) {
                StorageKeyIterator::<H256, RedeemRequestV1<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Blake2_128Concat>::new(<RedeemRequests<T>>::module_prefix(), b"RedeemRequests")
                    .drain()
                    .for_each(|(id, request_v1)| {
                        let status = match (request_v1.completed,request_v1.cancelled, request_v1.reimburse) {
                            (false, false, false) => RedeemRequestStatus::Pending,
                            (true, false, false) => RedeemRequestStatus::Completed,
                            (false, true, true) => RedeemRequestStatus::Reimbursed(true),
                            (false, true, false) => RedeemRequestStatus::Retried,
                            _ => RedeemRequestStatus::Completed, // should never happen
                        };
                        let request_v2 = RedeemRequest {
                            vault: request_v1.vault,
                            opentime: request_v1.opentime,
                            fee: request_v1.fee,
                            amount_btc: request_v1.amount_btc,
                            premium_dot: request_v1.premium_dot,
                            redeemer: request_v1.redeemer,
                            btc_address: request_v1.btc_address,
                            status
                        };
                        <RedeemRequests<T>>::insert(id, request_v2);
                    });

                StorageVersion::put(Version::V2);
            }

            0
        }


        /// A user requests to start the redeem procedure. This function checks the BTC Parachain
        /// status in Security and decides how the Redeem process is to be executed. If no `vault_id`
        /// is given the user's polkaBtc is burnt in exchange for liquidated collateral at the current
        /// exchange rate.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `btc_address` - the address to receive BTC
        /// * `vault_id` - [optional] address of the vault
        #[weight = <T as Config>::WeightInfo::request_redeem()]
        #[transactional]
        fn request_redeem(origin, amount_polka_btc: PolkaBTC<T>, btc_address: BtcAddress, vault_id: T::AccountId)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_request_redeem(redeemer, amount_polka_btc, btc_address, vault_id)?;
            Ok(())
        }

        #[weight = <T as Config>::WeightInfo::liquidation_redeem()]
        #[transactional]
        fn liquidation_redeem(origin, amount_polka_btc: PolkaBTC<T>) -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_liquidation_redeem(redeemer, amount_polka_btc)?;
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
        /// * `tx_block_height` - block number of backing chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        #[weight = <T as Config>::WeightInfo::execute_redeem()]
        #[transactional]
        fn execute_redeem(origin, redeem_id: H256, tx_id: H256Le, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_execute_redeem(redeem_id, tx_id, merkle_proof, raw_tx)?;
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
        #[weight = <T as Config>::WeightInfo::cancel_redeem()]
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
        amount_polka_btc: PolkaBTC<T>,
        btc_address: BtcAddress,
        vault_id: T::AccountId,
    ) -> Result<H256, DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(
            amount_polka_btc <= redeemer_balance,
            Error::<T>::AmountExceedsUserBalance
        );

        let fee_polka_btc = ext::fee::get_redeem_fee::<T>(amount_polka_btc)?;
        let redeem_amount_polka_btc = amount_polka_btc
            .checked_sub(&fee_polka_btc)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        let vault = ext::vault_registry::get_active_vault_from_id::<T>(&vault_id)?;
        let height = <frame_system::Module<T>>::block_number();
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;
        ensure!(
            redeem_amount_polka_btc <= vault.issued_tokens,
            Error::<T>::AmountExceedsVaultBalance
        );

        // only allow requests of amount above above the minimum
        let dust_value = <RedeemBtcDustValue<T>>::get();
        ensure!(
            // this is the amount the vault will send (minus fee)
            redeem_amount_polka_btc >= dust_value,
            Error::<T>::AmountBelowDustAmount
        );

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<T>(&vault_id, redeem_amount_polka_btc)?;

        // lock full amount (inc. fee)
        ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
        let redeem_id = ext::security::get_secure_id::<T>(&redeemer);

        let below_premium_redeem = ext::vault_registry::is_vault_below_premium_threshold::<T>(&vault_id)?;
        let premium_dot = if below_premium_redeem {
            let redeem_amount_polka_btc_in_dot = ext::oracle::btc_to_dots::<T>(redeem_amount_polka_btc)?;
            ext::fee::get_premium_redeem_fee::<T>(redeem_amount_polka_btc_in_dot)?
        } else {
            Self::u128_to_dot(0u128)?
        };

        Self::insert_redeem_request(
            redeem_id,
            RedeemRequest {
                vault: vault_id.clone(),
                opentime: height,
                fee: fee_polka_btc,
                amount_btc: redeem_amount_polka_btc,
                // TODO: reimplement partial redeem for system liquidation
                premium_dot,
                redeemer: redeemer.clone(),
                btc_address: btc_address.clone(),
                status: RedeemRequestStatus::Pending,
            },
        );

        // TODO: add fee to redeem event
        Self::deposit_event(<Event<T>>::RequestRedeem(
            redeem_id,
            redeemer,
            redeem_amount_polka_btc,
            fee_polka_btc,
            premium_dot,
            vault_id,
            btc_address,
        ));

        Ok(redeem_id)
    }

    fn _liquidation_redeem(redeemer: T::AccountId, amount_polka_btc: PolkaBTC<T>) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(
            amount_polka_btc <= redeemer_balance,
            Error::<T>::AmountExceedsUserBalance
        );

        ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
        ext::treasury::burn::<T>(redeemer.clone(), amount_polka_btc)?;
        ext::vault_registry::redeem_tokens_liquidation::<T>(&redeemer, amount_polka_btc)?;

        // vault-registry emits `RedeemTokensLiquidation` with dot amount
        Self::deposit_event(<Event<T>>::LiquidationRedeem(redeemer, amount_polka_btc));

        Ok(())
    }

    fn _execute_redeem(
        redeem_id: H256,
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;

        // only executable before the request has expired
        ensure!(
            !has_request_expired::<T>(redeem.opentime, Self::redeem_period()),
            Error::<T>::CommitPeriodExpired
        );

        let amount: usize = redeem.amount_btc.try_into().map_err(|_e| Error::<T>::TryIntoIntError)?;
        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
        // NOTE: vault client must register change addresses before
        // sending the bitcoin transaction
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            Some(amount as i64),
            redeem.btc_address,
            Some(redeem_id.clone().as_bytes().to_vec()),
        )?;

        // burn amount (without fee)
        ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_btc)?;

        // send fees to pool
        ext::treasury::unlock_and_transfer::<T>(
            redeem.redeemer.clone(),
            ext::fee::fee_pool_account_id::<T>(),
            redeem.fee,
        )?;
        ext::fee::increase_polka_btc_rewards_for_epoch::<T>(redeem.fee);

        ext::vault_registry::redeem_tokens::<T>(
            &redeem.vault,
            redeem.amount_btc,
            redeem.premium_dot,
            &redeem.redeemer,
        )?;

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
        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;
        ensure!(redeemer == redeem.redeemer, Error::<T>::UnauthorizedUser);

        // only cancellable after the request has expired
        ensure!(
            has_request_expired::<T>(redeem.opentime, Self::redeem_period()),
            Error::<T>::TimeNotExpired
        );

        let vault = ext::vault_registry::get_vault_from_id::<T>(&redeem.vault)?;
        let vault_id = redeem.vault.clone();

        let amount_polka_btc_in_dot = ext::oracle::btc_to_dots::<T>(redeem.amount_btc)?;
        let punishment_fee_in_dot = ext::fee::get_punishment_fee::<T>(amount_polka_btc_in_dot)?;

        // now update the collateral; the logic is different for liquidated vaults.
        let slashed_amount = if vault.is_liquidated() {
            let confiscated_collateral = ext::vault_registry::calculate_collateral::<T>(
                ext::vault_registry::get_backing_collateral::<T>(&redeem.vault)?,
                redeem.amount_btc,
                vault.to_be_redeemed_tokens, // note: this is the value read at start of function
            )?;

            let slashing_destination = if reimburse {
                CurrencySource::FreeBalance(redeemer.clone())
            } else {
                CurrencySource::LiquidationVault
            };
            ext::vault_registry::slash_collateral::<T>(
                CurrencySource::Backing(vault_id.clone()),
                slashing_destination,
                confiscated_collateral,
            )?;

            confiscated_collateral
        } else {
            // not liquidated
            let slashed_dot = if reimburse {
                // user requested to be reimbursed in DOT
                let reimburse_in_dot = amount_polka_btc_in_dot
                    .checked_add(&punishment_fee_in_dot)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ext::vault_registry::slash_collateral_saturated::<T>(
                    CurrencySource::Backing(vault_id.clone()),
                    CurrencySource::FreeBalance(redeem.redeemer.clone()),
                    reimburse_in_dot,
                )?
            } else {
                // user chose to keep his PolkaBTC - only transfer it the punishment fee
                let slashed_punishment_fee = ext::vault_registry::slash_collateral_saturated::<T>(
                    CurrencySource::Backing(vault_id.clone()),
                    CurrencySource::FreeBalance(redeemer.clone()),
                    punishment_fee_in_dot,
                )?;

                slashed_punishment_fee
            };
            // calculate additional amount to slash, a high SLA means we slash less
            let slashing_amount_in_dot =
                ext::sla::calculate_slashed_amount::<T>(&vault_id, amount_polka_btc_in_dot, reimburse)?;

            // slash the remaining amount from the vault to the fee pool
            let remaining_dot_to_be_slashed = slashing_amount_in_dot
                .checked_sub(&slashed_dot)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            if remaining_dot_to_be_slashed > Self::u128_to_dot(0u128)? {
                let slashed_to_fee_pool = ext::vault_registry::slash_collateral_saturated::<T>(
                    CurrencySource::Backing(vault_id.clone()),
                    CurrencySource::FreeBalance(ext::fee::fee_pool_account_id::<T>()),
                    remaining_dot_to_be_slashed,
                )?;
                ext::fee::increase_dot_rewards_for_epoch::<T>(slashed_to_fee_pool);
            }
            let _ = ext::vault_registry::ban_vault::<T>(vault_id.clone());

            slashing_amount_in_dot
        };

        // first update the polkabtc tokens; this logic is the same regardless of whether or not the vault is liquidated
        if reimburse {
            // Transfer the transaction fee to the pool. Even though the redeem was not
            // successful, the user receives a premium in DOT, so it's to take the fee.
            ext::treasury::unlock_and_transfer::<T>(
                redeem.redeemer.clone(),
                ext::fee::fee_pool_account_id::<T>(),
                redeem.fee,
            )?;
            ext::fee::increase_polka_btc_rewards_for_epoch::<T>(redeem.fee);

            if ext::vault_registry::is_vault_below_secure_threshold::<T>(&redeem.vault)? {
                // vault can not afford to back the tokens that he would receive, so we burn it
                ext::treasury::burn::<T>(redeemer.clone(), redeem.amount_btc)?;
                ext::vault_registry::decrease_tokens::<T>(&redeem.vault, &redeem.redeemer, redeem.amount_btc)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(false));
            } else {
                // Transfer the rest of the user's PolkaBTC (i.e. excluding fee) to the vault
                ext::treasury::unlock_and_transfer::<T>(
                    redeem.redeemer.clone(),
                    redeem.vault.clone(),
                    redeem.amount_btc,
                )?;
                ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, redeem.amount_btc)?;
                Self::set_redeem_status(redeem_id, RedeemRequestStatus::Reimbursed(true));
            }
        } else {
            // unlock user's PolkaBTC, including fee
            let total_polka_btc = redeem
                .amount_btc
                .checked_add(&redeem.fee)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ext::treasury::unlock::<T>(redeemer.clone(), total_polka_btc)?;
            ext::vault_registry::decrease_to_be_redeemed_tokens::<T>(&vault_id, redeem.amount_btc)?;
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

        ext::vault_registry::try_increase_to_be_issued_tokens::<T>(&vault_id, redeem.amount_btc)?;
        ext::vault_registry::issue_tokens::<T>(&vault_id, redeem.amount_btc)?;
        ext::treasury::mint::<T>(vault_id, redeem.amount_btc);

        Self::set_redeem_status(redeem_id, RedeemRequestStatus::Completed);

        Ok(())
    }

    /// Insert a new redeem request into state.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    /// * `value` - the redeem request
    fn insert_redeem_request(key: H256, value: RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>) {
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
    ) -> Vec<(H256, RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>)> {
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
    ) -> Vec<(H256, RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>)> {
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
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError> {
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
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError> {
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

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }
}

fn has_request_expired<T: Config>(opentime: T::BlockNumber, period: T::BlockNumber) -> bool {
    let height = <frame_system::Module<T>>::block_number();
    height > opentime + period
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
