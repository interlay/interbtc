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

pub use crate::types::RedeemRequest;
use crate::types::{PolkaBTC, Version, DOT};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::weights::Weight;
use security::ErrorCode;
use sp_runtime::traits::CheckedAdd;
use sp_std::convert::TryInto;
use util::transactional;

/// # PolkaBTC Redeem implementation
/// The Redeem module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/redeem.html
// Substrate
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use frame_system::{ensure_root, ensure_signed};
use primitive_types::H256;
use sp_runtime::traits::*;
use sp_runtime::ModuleId;
use sp_std::vec::Vec;

/// The redeem module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"i/redeem");

pub trait WeightInfo {
    fn request_redeem() -> Weight;
    fn liquidation_redeem() -> Weight;
    fn execute_redeem() -> Weight;
    fn cancel_redeem() -> Weight;
    fn set_redeem_period() -> Weight;
}

/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait
    + vault_registry::Trait
    + collateral::Trait
    + btc_relay::Trait
    + treasury::Trait
    + fee::Trait
    + sla::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Redeem {
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
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
    {
        // [redeemId, redeemer, amountPolkaBTC, vault, btcAddress]
        RequestRedeem(H256, AccountId, PolkaBTC, AccountId, BtcAddress),
        // [redeemer, amountPolkaBTC]
        LiquidationRedeem(AccountId, PolkaBTC),
        // [redeemId, redeemer, vault]
        ExecuteRedeem(H256, AccountId, AccountId),
        // [redeemId, redeemer]
        CancelRedeem(H256, AccountId),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
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
        #[weight = <T as Trait>::WeightInfo::request_redeem()]
        #[transactional]
        fn request_redeem(origin, amount_polka_btc: PolkaBTC<T>, btc_address: BtcAddress, vault_id: T::AccountId)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_request_redeem(redeemer, amount_polka_btc, btc_address, vault_id)?;
            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::liquidation_redeem()]
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
        /// * `origin` - the vault responsible for executing this redeem request
        /// * `redeem_id` - identifier of redeem request as output from request_redeem
        /// * `tx_id` - transaction hash
        /// * `tx_block_height` - block number of backing chain
        /// * `merkle_proof` - raw bytes
        /// * `raw_tx` - raw bytes
        #[weight = <T as Trait>::WeightInfo::execute_redeem()]
        #[transactional]
        fn execute_redeem(origin, redeem_id: H256, tx_id: H256Le, merkle_proof: Vec<u8>, raw_tx: Vec<u8>)
            -> DispatchResult
        {
            let vault_id = ensure_signed(origin)?;
            Self::_execute_redeem(vault_id, redeem_id, tx_id, merkle_proof, raw_tx)?;
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
        #[weight = <T as Trait>::WeightInfo::cancel_redeem()]
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
        #[weight = <T as Trait>::WeightInfo::set_redeem_period()]
        #[transactional]
        fn set_redeem_period(origin, period: T::BlockNumber) {
            ensure_root(origin)?;
            <RedeemPeriod<T>>::set(period);
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    fn _request_redeem(
        redeemer: T::AccountId,
        amount_polka_btc: PolkaBTC<T>,
        btc_address: BtcAddress,
        vault_id: T::AccountId,
    ) -> Result<H256, DispatchError> {
        Self::ensure_parachain_running_or_error_liquidated()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(
            amount_polka_btc <= redeemer_balance,
            Error::<T>::AmountExceedsUserBalance
        );

        let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
        let height = <frame_system::Module<T>>::block_number();
        ext::vault_registry::ensure_not_banned::<T>(&vault_id, height)?;
        ensure!(
            amount_polka_btc <= vault.issued_tokens,
            Error::<T>::AmountExceedsVaultBalance
        );

        let fee_polka_btc = ext::fee::get_redeem_fee::<T>(amount_polka_btc)?;
        let redeem_amount_polka_btc = amount_polka_btc
            .checked_sub(&fee_polka_btc)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        // only allow requests of amount above above the minimum
        let dust_value = <RedeemBtcDustValue<T>>::get();
        ensure!(
            // this is the amount the vault will send (minus fee)
            redeem_amount_polka_btc >= dust_value,
            Error::<T>::AmountBelowDustAmount
        );

        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(
            &vault_id,
            redeem_amount_polka_btc,
        )?;

        // lock full amount (inc. fee)
        ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
        let redeem_id = ext::security::get_secure_id::<T>(&redeemer);

        let below_premium_redeem =
            ext::vault_registry::is_vault_below_premium_threshold::<T>(&vault_id)?;
        let premium_dot = if below_premium_redeem {
            let redeem_amount_polka_btc_in_dot =
                ext::oracle::btc_to_dots::<T>(redeem_amount_polka_btc)?;
            ext::fee::get_premium_redeem_fee::<T>(redeem_amount_polka_btc_in_dot)?
        } else {
            0.into()
        };

        Self::insert_redeem_request(
            redeem_id,
            RedeemRequest {
                vault: vault_id.clone(),
                opentime: height,
                amount_polka_btc: redeem_amount_polka_btc,
                fee: fee_polka_btc,
                amount_btc: redeem_amount_polka_btc,
                // TODO: reimplement partial redeem for system liquidation
                amount_dot: 0.into(),
                premium_dot,
                redeemer: redeemer.clone(),
                btc_address: btc_address.clone(),
                completed: false,
                cancelled: false,
                reimburse: false,
            },
        );

        // TODO: add fee to redeem event
        Self::deposit_event(<Event<T>>::RequestRedeem(
            redeem_id,
            redeemer,
            redeem_amount_polka_btc,
            vault_id,
            btc_address,
        ));

        Ok(redeem_id)
    }

    fn _liquidation_redeem(
        redeemer: T::AccountId,
        amount_polka_btc: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        Self::ensure_parachain_running_or_error_liquidated()?;

        let redeemer_balance = ext::treasury::get_balance::<T>(redeemer.clone());
        ensure!(
            amount_polka_btc <= redeemer_balance,
            Error::<T>::AmountExceedsUserBalance
        );

        ext::treasury::lock::<T>(redeemer.clone(), amount_polka_btc)?;
        ext::treasury::burn::<T>(redeemer.clone(), amount_polka_btc)?;
        ext::vault_registry::redeem_tokens_liquidation::<T>(&redeemer, amount_polka_btc)?;

        Self::deposit_event(<Event<T>>::LiquidationRedeem(redeemer, amount_polka_btc));

        Ok(())
    }

    fn _execute_redeem(
        vault_id: T::AccountId,
        redeem_id: H256,
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        Self::ensure_parachain_running_or_error_liquidated()?;

        let redeem = Self::get_open_redeem_request_from_id(&redeem_id)?;
        ensure!(vault_id == redeem.vault, Error::<T>::UnauthorizedVault);

        // only executable before the request has expired
        ensure!(
            !has_request_expired::<T>(redeem.opentime, Self::redeem_period()),
            Error::<T>::CommitPeriodExpired
        );

        let amount: usize = redeem
            .amount_btc
            .try_into()
            .map_err(|_e| Error::<T>::TryIntoIntError)?;
        ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
        // NOTE: vault client must register change addresses before
        // sending the bitcoin transaction
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            amount as i64,
            redeem.btc_address,
            Some(redeem_id.clone().as_bytes().to_vec()),
        )?;

        let amount_polka_btc = redeem.amount_polka_btc;
        let fee_polka_btc = redeem.fee;
        // burn amount (minus fee)
        ext::treasury::burn::<T>(redeem.redeemer.clone(), amount_polka_btc)?;

        // send fees to pool
        ext::treasury::unlock_and_transfer::<T>(
            redeem.redeemer.clone(),
            ext::fee::fee_pool_account_id::<T>(),
            fee_polka_btc,
        )?;
        ext::fee::increase_polka_btc_rewards_for_epoch::<T>(fee_polka_btc);

        if redeem.premium_dot > 0.into() {
            ext::vault_registry::redeem_tokens_premium::<T>(
                &redeem.vault,
                amount_polka_btc,
                redeem.premium_dot,
                &redeem.redeemer,
            )?;
        } else {
            ext::vault_registry::redeem_tokens::<T>(&redeem.vault, amount_polka_btc)?;
        }

        Self::remove_redeem_request(redeem_id, false, false);
        Self::deposit_event(<Event<T>>::ExecuteRedeem(
            redeem_id,
            redeem.redeemer,
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

        let amount_polka_btc_in_dot = ext::oracle::btc_to_dots::<T>(redeem.amount_polka_btc)?;
        let punishment_fee_in_dot = ext::fee::get_punishment_fee::<T>(amount_polka_btc_in_dot)?;

        // calculate additional amount to slash, a high SLA means we slash less
        let slashing_amount_in_dot =
            ext::sla::calculate_slashed_amount::<T>(redeem.vault.clone(), amount_polka_btc_in_dot)?;

        // slash collateral from the vault to the user
        let slashed_dot = if reimburse {
            // user requested to be reimbursed in DOT
            ext::vault_registry::decrease_tokens::<T>(
                &redeem.vault,
                &redeem.redeemer,
                redeem.amount_polka_btc,
            )?;

            // burn user's PolkaBTC
            ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;

            // reimburse the user in dot (inc. punishment fee) from vault
            let reimburse_in_dot = amount_polka_btc_in_dot
                .checked_add(&punishment_fee_in_dot)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ext::collateral::slash_collateral::<T>(
                &redeem.vault,
                &redeem.redeemer,
                reimburse_in_dot,
            )?;
            reimburse_in_dot
        } else {
            // user does not want full reimbursement and wishes to retry the redeem
            ext::collateral::slash_collateral::<T>(
                &redeem.vault,
                &redeem.redeemer,
                punishment_fee_in_dot,
            )?;
            punishment_fee_in_dot
        };

        // slash the remaining amount from the vault to the fee pool
        let remaining_dot_to_be_slashed = slashing_amount_in_dot
            .checked_sub(&slashed_dot)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        if remaining_dot_to_be_slashed > 0.into() {
            ext::collateral::slash_collateral::<T>(
                &redeem.vault,
                &ext::fee::fee_pool_account_id::<T>(),
                remaining_dot_to_be_slashed,
            )?;
            ext::fee::increase_dot_rewards_for_epoch::<T>(remaining_dot_to_be_slashed);
        }

        ext::vault_registry::ban_vault::<T>(redeem.vault.clone())?;
        ext::sla::event_update_vault_sla::<T>(redeem.vault, ext::sla::VaultEvent::RedeemFailure)?;
        Self::remove_redeem_request(redeem_id, true, reimburse);
        Self::deposit_event(<Event<T>>::CancelRedeem(redeem_id, redeemer));

        Ok(())
    }

    /// Ensure that the parachain is running or the system is in liquidation
    fn ensure_parachain_running_or_error_liquidated() -> DispatchResult {
        ext::security::ensure_parachain_is_running_or_only_has_errors::<T>(
            [ErrorCode::Liquidation].to_vec(),
        )
    }

    /// Insert a new redeem request into state.
    ///
    /// # Arguments
    ///
    /// * `key` - 256-bit identifier of the redeem request
    /// * `value` - the redeem request
    fn insert_redeem_request(
        key: H256,
        value: RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <RedeemRequests<T>>::insert(key, value)
    }

    fn remove_redeem_request(id: H256, cancelled: bool, reimburse: bool) {
        // TODO: delete redeem request from storage
        <RedeemRequests<T>>::mutate(id, |request| {
            request.completed = !cancelled;
            request.cancelled = cancelled;
            request.reimburse = reimburse
        });
    }

    /// Fetch all redeem requests for the specified account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - user account id
    pub fn get_redeem_requests_for_account(
        account_id: T::AccountId,
    ) -> Vec<(
        H256,
        RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
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
    ) -> Vec<(
        H256,
        RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    )> {
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
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        ensure!(
            <RedeemRequests<T>>::contains_key(*redeem_id),
            Error::<T>::RedeemIdNotFound
        );
        // NOTE: temporary workaround until we delete
        ensure!(
            !<RedeemRequests<T>>::get(*redeem_id).completed,
            Error::<T>::RedeemCompleted
        );
        ensure!(
            !<RedeemRequests<T>>::get(*redeem_id).cancelled,
            Error::<T>::RedeemCancelled
        );
        Ok(<RedeemRequests<T>>::get(*redeem_id))
    }

    /// Fetch a pre-existing open or completed redeem request or throw.
    /// Cancelled requests are not returned.
    ///
    /// # Arguments
    ///
    /// * `redeem_id` - 256-bit identifier of the redeem request
    pub fn get_open_or_completed_redeem_request_from_id(
        redeem_id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        ensure!(
            <RedeemRequests<T>>::contains_key(*redeem_id),
            Error::<T>::RedeemIdNotFound
        );
        ensure!(
            !<RedeemRequests<T>>::get(*redeem_id).cancelled,
            Error::<T>::RedeemCancelled
        );
        Ok(<RedeemRequests<T>>::get(*redeem_id))
    }
}

fn has_request_expired<T: Trait>(opentime: T::BlockNumber, period: T::BlockNumber) -> bool {
    let height = <frame_system::Module<T>>::block_number();
    height > opentime + period
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        AmountExceedsUserBalance,
        AmountExceedsVaultBalance,
        UnauthorizedVault,
        CommitPeriodExpired,
        UnauthorizedUser,
        TimeNotExpired,
        RedeemCompleted,
        RedeemCancelled,
        RedeemIdNotFound,
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        AmountBelowDustAmount,
    }
}
