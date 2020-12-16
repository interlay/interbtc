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
use crate::types::{PolkaBTC, RedeemRequestV0, Version, DOT};
use bitcoin::types::H256Le;
use btc_relay::BtcAddress;
use frame_support::weights::Weight;
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
use security::ErrorCode;
use sp_runtime::traits::CheckedAdd;
use sp_runtime::traits::CheckedSub;
use sp_runtime::ModuleId;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// The redeem module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"i/redeem");

pub trait WeightInfo {
    fn request_redeem() -> Weight;
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
        RequestRedeem(H256, AccountId, PolkaBTC, AccountId, BtcAddress),
        ExecuteRedeem(H256, AccountId, AccountId),
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
            use frame_support::{migration::StorageKeyIterator, Blake2_128Concat};

            if Self::storage_version() == Version::V0 {
                StorageKeyIterator::<H256, RedeemRequestV0<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Blake2_128Concat>::new(<RedeemRequests<T>>::module_prefix(), b"RedeemRequests")
                    .drain()
                    .for_each(|(id, request_v0)| {
                        let request_v1 = RedeemRequest {
                            vault: request_v0.vault,
                            opentime: request_v0.opentime,
                            amount_polka_btc: request_v0.amount_polka_btc,
                            fee: 0.into(),
                            amount_btc: request_v0.amount_btc,
                            amount_dot: request_v0.amount_dot,
                            premium_dot: request_v0.premium_dot,
                            redeemer: request_v0.redeemer,
                            btc_address: BtcAddress::P2WPKHv0(request_v0.btc_address),
                            completed: request_v0.completed,
                            cancelled: false,
                            reimburse: false,
                        };
                        <RedeemRequests<T>>::insert(id, request_v1);
                    });

                StorageVersion::put(Version::V1);
            }

            0
        }

        /// A user requests to start the redeem procedure. This function checks the BTC Parachain
        /// status in Security and decides how the Redeem process is to be executed.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `btc_address` - the address to receive BTC
        /// * `vault` - address of the vault
        #[weight = <T as Trait>::WeightInfo::request_redeem()]
        fn request_redeem(origin, amount_polka_btc: PolkaBTC<T>, btc_address: BtcAddress, vault_id: T::AccountId)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            Self::_request_redeem(redeemer, amount_polka_btc, btc_address, vault_id)?;
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
        fn cancel_redeem(origin, redeem_id: H256, reimburse: bool)
            -> DispatchResult
        {
            let redeemer = ensure_signed(origin)?;
            let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
            ensure!(redeemer == redeem.redeemer, Error::<T>::UnauthorizedUser);

            // only cancellable after the request has expired
            ensure!(
                has_request_expired::<T>(redeem.opentime, Self::redeem_period()),
                Error::<T>::TimeNotExpired
            );

            let amount_polka_btc_in_dot = ext::oracle::btc_to_dots::<T>(redeem.amount_polka_btc)?;
            let punishment_fee_in_dot = ext::fee::get_punishment_fee::<T>(amount_polka_btc_in_dot)?;

            // calculate additional amount to slash, a high SLA means we slash less
            let slashing_amount_in_dot = ext::sla::calculate_slashed_amount::<T>(
                redeem.vault.clone(),
                amount_polka_btc_in_dot
            )?;

            let slashing_amount_in_dot = if reimburse {
                // user requested to be reimbursed in DOT
                ext::vault_registry::decrease_tokens::<T>(
                    &redeem.vault,
                    &redeem.redeemer,
                    redeem.amount_polka_btc,
                )?;

                // burn user's PolkaBTC
                ext::treasury::burn::<T>(redeem.redeemer.clone(), redeem.amount_polka_btc)?;

                // reimburse the user in dot (inc. punishment fee) from vault
                let reimburse_in_dot = amount_polka_btc_in_dot + punishment_fee_in_dot;
                ext::collateral::slash_collateral::<T>(
                    &redeem.vault,
                    &redeem.redeemer,
                    reimburse_in_dot,
                )?;
                slashing_amount_in_dot.checked_sub(&amount_polka_btc_in_dot).ok_or(Error::<T>::ArithmeticUnderflow)?
            } else {
                // user does not want full reimbursement and wishes to retry the redeem
                ext::collateral::slash_collateral::<T>(&redeem.redeemer, &redeem.vault, punishment_fee_in_dot)?;
                slashing_amount_in_dot
            };

            // slash the vault's collateral to mint fees
            let sla_dot_delta = slashing_amount_in_dot.checked_sub(&punishment_fee_in_dot).ok_or(Error::<T>::ArithmeticUnderflow)?;
            if sla_dot_delta > 0.into() {
                ext::collateral::slash_collateral::<T>(
                    &redeem.vault,
                    &ext::fee::fee_pool_account_id::<T>(),
                    sla_dot_delta,
                )?;
                ext::fee::increase_dot_rewards_for_epoch::<T>(sla_dot_delta);
            }

            ext::vault_registry::ban_vault::<T>(redeem.vault.clone())?;
            ext::sla::event_update_vault_sla::<T>(redeem.vault, ext::sla::VaultEvent::RedeemFailure)?;
            Self::remove_redeem_request(redeem_id, true, reimburse);
            Self::deposit_event(<Event<T>>::CancelRedeem(redeem_id, redeemer));

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

        // TODO: introduce max fee_polka_btc param
        let fee_polka_btc = ext::fee::get_redeem_fee::<T>(amount_polka_btc)?;
        let total_amount_polka_btc = amount_polka_btc
            .checked_add(&fee_polka_btc)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        // only allow requests of amount above above the minimum
        let dust_value = <RedeemBtcDustValue<T>>::get();
        ensure!(
            // this is the amount the vault will send (minus fee)
            amount_polka_btc >= dust_value,
            Error::<T>::AmountBelowDustAmount
        );

        let (amount_btc, amount_dot): (u128, u128) =
            if ext::security::is_parachain_error_liquidation::<T>() {
                let raw_amount_polka_btc = Self::btc_to_u128(amount_polka_btc)?;
                let amount_dot_in_btc = Self::partial_redeem(raw_amount_polka_btc)?;
                let amount_btc: u128 = raw_amount_polka_btc - amount_dot_in_btc;
                let amount_dot: u128 = Self::rawbtc_to_rawdot(amount_dot_in_btc)?;
                (amount_btc, amount_dot)
            } else {
                (Self::btc_to_u128(amount_polka_btc)?, 0)
            };

        let amount_btc = Self::u128_to_btc(amount_btc)?;
        let amount_dot_as_btc = Self::u128_to_btc(amount_dot)?;
        let amount_dot = Self::u128_to_dot(amount_dot)?;

        ext::vault_registry::increase_to_be_redeemed_tokens::<T>(&vault_id, amount_btc)?;

        if amount_dot_as_btc > 0.into() {
            // TODO: should redeem_dot_in_btc be `DOT<T>`?
            ext::vault_registry::redeem_tokens_liquidation::<T>(&vault_id, amount_dot_as_btc)?;
        }

        // lock full amount (inc. fee)
        ext::treasury::lock::<T>(redeemer.clone(), total_amount_polka_btc)?;
        let redeem_id = ext::security::get_secure_id::<T>(&redeemer);

        let below_premium_redeem =
            ext::vault_registry::is_vault_below_premium_threshold::<T>(&vault_id)?;
        let premium_dot = if below_premium_redeem {
            ext::fee::get_premium_redeem_fee::<T>(amount_dot)?
        } else {
            0.into()
        };

        Self::insert_redeem_request(
            redeem_id,
            RedeemRequest {
                vault: vault_id.clone(),
                opentime: height,
                amount_polka_btc,
                fee: fee_polka_btc,
                amount_btc,
                amount_dot,
                premium_dot,
                redeemer: redeemer.clone(),
                btc_address: btc_address.clone(),
                completed: false,
                cancelled: false,
                reimburse: false,
            },
        );
        Self::deposit_event(<Event<T>>::RequestRedeem(
            redeem_id,
            redeemer,
            amount_polka_btc,
            vault_id,
            btc_address,
        ));

        Ok(redeem_id)
    }

    fn _execute_redeem(
        vault_id: T::AccountId,
        redeem_id: H256,
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        let redeem = Self::get_redeem_request_from_id(&redeem_id)?;
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
        // TODO: register change addresses (vault wallet)
        ext::btc_relay::validate_transaction::<T>(
            raw_tx,
            amount as i64,
            redeem.btc_address,
            redeem_id.clone().as_bytes().to_vec(),
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

    /// Fetch a pre-existing redeem request or throw.
    ///
    /// # Arguments
    ///
    /// * `redeem_id` - 256-bit identifier of the redeem request
    pub fn get_redeem_request_from_id(
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

    /// Ensure that the parachain is running or a vault is being liquidated.
    fn ensure_parachain_running_or_error_liquidated() -> DispatchResult {
        ext::security::ensure_parachain_only_has_errors::<T>([ErrorCode::Liquidation].to_vec())?;
        ext::security::ensure_parachain_status_running::<T>()
    }

    /// Calculates the fraction of BTC to be redeemed in DOT when the
    /// BTC Parachain state is in ERROR state due to a LIQUIDATION error.
    fn get_partial_redeem_factor() -> Result<u128, DispatchError> {
        let total_liquidation_value = ext::vault_registry::total_liquidation_value::<T>()?;
        let total_supply = Self::btc_to_u128(ext::treasury::get_total_supply::<T>())?;
        Ok(total_liquidation_value / total_supply)
    }

    fn btc_to_u128(amount: PolkaBTC<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::<T>::TryIntoIntError.into())
    }

    fn dot_to_u128(amount: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_btc(x: u128) -> Result<PolkaBTC<T>, DispatchError> {
        TryInto::<PolkaBTC<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn rawbtc_to_rawdot(btc: u128) -> Result<u128, DispatchError> {
        let dots: DOT<T> = ext::oracle::btc_to_dots::<T>(Self::u128_to_btc(btc)?)?;
        Self::dot_to_u128(dots)
    }

    fn partial_redeem(raw_btc: u128) -> Result<u128, DispatchError> {
        raw_btc
            .checked_mul(Self::get_partial_redeem_factor()?)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_div(100_000)
            .ok_or(Error::<T>::ArithmeticUnderflow.into())
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
