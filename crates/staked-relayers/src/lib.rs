//! # Staked Relayers Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/staked-relayers.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod types;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

pub use security;

use crate::types::{Collateral, Wrapped};
use bitcoin::{parser::parse_transaction, types::*};

use btc_relay::BtcAddress;
use frame_support::{dispatch::DispatchResult, ensure, transactional};
use frame_system::ensure_signed;
use sp_core::H256;

use sp_std::{collections::btree_set::BTreeSet, convert::TryInto, vec::Vec};
use vault_registry::Wallet;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + security::Config
        + currency::Config<currency::Collateral>
        + currency::Config<currency::Wrapped>
        + vault_registry::Config
        + btc_relay::Config
        + redeem::Config
        + replace::Config
        + refund::Config
        + sla::Config
        + fee::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        VaultTheft(T::AccountId, H256Le),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Vault already reported
        VaultAlreadyReported,
        /// Vault BTC address not in transaction input
        VaultNoInputToTransaction,
        /// Valid redeem transaction
        ValidRedeemTransaction,
        /// Valid replace transaction
        ValidReplaceTransaction,
        /// Valid refund transaction
        ValidRefundTransaction,
        /// Valid merge transaction
        ValidMergeTransaction,
        /// Failed to parse transaction
        InvalidTransaction,
        /// Unable to convert value
        TryIntoIntError,
    }

    /// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
    /// identifiers of Vaults accused of theft.
    #[pallet::storage]
    #[pallet::getter(fn theft_report)]
    pub(super) type TheftReports<T: Config> =
        StorageMap<_, Blake2_128Concat, H256Le, BTreeSet<T::AccountId>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// One time function to initialize the BTC-Relay with the first block
        ///
        /// # Arguments
        ///
        /// * `block_header_bytes` - 80 byte raw Bitcoin block header.
        /// * `block_height` - starting Bitcoin block height of the submitted block header.
        ///
        /// # <weight>
        /// - Storage Reads:
        /// 	- One storage read to check that parachain is not shutdown. O(1)
        /// 	- One storage read to check if relayer authorization is disabled. O(1)
        /// 	- One storage read to check if relayer is authorized. O(1)
        /// - Storage Writes:
        ///     - One storage write to store block hash. O(1)
        ///     - One storage write to store block header. O(1)
        /// 	- One storage write to initialize main chain. O(1)
        ///     - One storage write to store best block hash. O(1)
        ///     - One storage write to store best block height. O(1)
        /// - Events:
        /// 	- One event for initialization.
        ///
        /// Total Complexity: O(1)
        /// # </weight>
        #[pallet::weight(<T as Config>::WeightInfo::initialize())]
        #[transactional]
        pub(super) fn initialize(
            origin: OriginFor<T>,
            raw_block_header: RawBlockHeader,
            block_height: u32,
        ) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let relayer = ensure_signed(origin)?;
            ext::btc_relay::initialize::<T>(relayer, raw_block_header, block_height)?;
            Ok(().into())
        }

        /// Stores a single new block header
        ///
        /// # Arguments
        ///
        /// * `raw_block_header` - 80 byte raw Bitcoin block header.
        ///
        /// # <weight>
        /// Key: C (len of chains), P (len of positions)
        /// - Storage Reads:
        /// 	- One storage read to check that parachain is not shutdown. O(1)
        /// 	- One storage read to check if relayer authorization is disabled. O(1)
        /// 	- One storage read to check if relayer is authorized. O(1)
        /// 	- One storage read to check if block header is stored. O(1)
        /// 	- One storage read to retrieve parent block hash. O(1)
        /// 	- One storage read to check if difficulty check is disabled. O(1)
        /// 	- One storage read to retrieve last re-target. O(1)
        /// 	- One storage read to retrieve all Chains. O(C)
        /// - Storage Writes:
        ///     - One storage write to store block hash. O(1)
        ///     - One storage write to store block header. O(1)
        /// 	- One storage mutate to extend main chain. O(1)
        ///     - One storage write to store best block hash. O(1)
        ///     - One storage write to store best block height. O(1)
        /// - Notable Computation:
        /// 	- O(P) sort to reorg chains.
        /// - External Module Operations:
        /// 	- Updates relayer sla score.
        /// - Events:
        /// 	- One event for block stored (fork or extension).
        ///
        /// Total Complexity: O(C + P)
        /// # </weight>
        #[pallet::weight(<T as Config>::WeightInfo::store_block_header())]
        #[transactional]
        pub(super) fn store_block_header(
            origin: OriginFor<T>,
            raw_block_header: RawBlockHeader,
        ) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let relayer = ensure_signed(origin)?;
            Self::store_block_header_and_update_sla(&relayer, raw_block_header)?;
            Ok(().into())
        }

        /// A Staked Relayer reports misbehavior by a Vault, providing a fraud proof
        /// (malicious Bitcoin transaction and the corresponding transaction inclusion proof).
        ///
        /// # Arguments
        ///
        /// * `origin`: Any signed user.
        /// * `vault_id`: The account of the vault to check.
        /// * `tx_id`: The hash of the transaction
        /// * `merkle_proof`: The proof of tx inclusion.
        /// * `raw_tx`: The raw Bitcoin transaction.
        #[pallet::weight(<T as Config>::WeightInfo::report_vault_theft())]
        #[transactional]
        pub(super) fn report_vault_theft(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;

            let transaction = parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;
            let tx_id = transaction.tx_id();

            // liquidated vaults are removed, so no need for check here

            // throw if already reported
            if <TheftReports<T>>::contains_key(&tx_id) {
                ensure!(
                    !<TheftReports<T>>::get(&tx_id).contains(&vault_id),
                    Error::<T>::VaultAlreadyReported,
                );
            }

            ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
            Self::_is_parsed_transaction_invalid(&vault_id, transaction)?;

            ext::vault_registry::liquidate_theft_vault::<T>(&vault_id)?;

            <TheftReports<T>>::mutate(&tx_id, |reports| {
                reports.insert(vault_id.clone());
            });

            // reward the participant by increasing their SLA
            ext::sla::event_update_relayer_sla::<T>(&signer, ext::sla::RelayerEvent::TheftReport)?;

            Self::deposit_event(<Event<T>>::VaultTheft(vault_id, tx_id));

            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn store_block_header_and_update_sla(relayer: &T::AccountId, raw_block_header: RawBlockHeader) -> DispatchResult {
        ext::btc_relay::store_block_header::<T>(&relayer, raw_block_header)?;
        // reward the participant by increasing their SLA
        ext::sla::event_update_relayer_sla::<T>(&relayer, ext::sla::RelayerEvent::StoreBlock)?;
        Ok(())
    }

    /// Checks if the vault is doing a valid merge transaction to move funds between
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `payments` - all payment outputs extracted from tx
    /// * `op_returns` - all op_return outputs extracted from tx
    /// * `wallet` - vault btc addresses
    pub(crate) fn is_valid_merge_transaction(
        payments: &[(i64, BtcAddress)],
        op_returns: &[(i64, Vec<u8>)],
        wallet: &Wallet,
    ) -> bool {
        if !op_returns.is_empty() {
            // migration should only contain payments
            return false;
        }

        for (_value, address) in payments {
            if !wallet.has_btc_address(&address) {
                return false;
            }
        }

        true
    }

    /// Checks if the vault is sending a valid request transaction.
    ///
    /// # Arguments
    ///
    /// * `request_value` - amount of btc as specified in the request
    /// * `request_address` - recipient btc address
    /// * `payments` - all payment outputs extracted from tx
    /// * `wallet` - vault btc addresses
    pub(crate) fn is_valid_request_transaction(
        request_value: Wrapped<T>,
        request_address: BtcAddress,
        payments: &[(i64, BtcAddress)],
        wallet: &Wallet,
    ) -> bool {
        let request_value = match TryInto::<u64>::try_into(request_value).map_err(|_e| Error::<T>::TryIntoIntError) {
            Ok(value) => value as i64,
            Err(_) => return false,
        };

        // check all outputs, vault cannot pay to unknown recipients
        for (value, address) in payments {
            if *address == request_address {
                if *value < request_value {
                    // insufficient payment to recipient
                    return false;
                }
            } else if !wallet.has_btc_address(&address) {
                // payment to unknown address
                return false;
            }
        }

        // tx has sufficient payment to recipient and
        // all refunds are to wallet addresses
        true
    }

    /// Check if a vault transaction is invalid. Returns `Ok` if invalid or `Err` otherwise.
    /// This method should be callable over RPC for a staked-relayer client to check validity.
    ///
    /// # Arguments
    ///
    /// `vault_id`: the vault.
    /// `raw_tx`: the BTC transaction by the vault.
    pub fn is_transaction_invalid(vault_id: &T::AccountId, raw_tx: Vec<u8>) -> DispatchResult {
        let tx = parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;
        Self::_is_parsed_transaction_invalid(vault_id, tx)
    }

    /// Check if a vault transaction is invalid. Returns `Ok` if invalid or `Err` otherwise.
    pub fn _is_parsed_transaction_invalid(vault_id: &T::AccountId, tx: Transaction) -> DispatchResult {
        let vault = ext::vault_registry::get_active_vault_from_id::<T>(vault_id)?;

        // collect all addresses that feature in the inputs of the transaction
        let input_addresses: Vec<Result<BtcAddress, _>> = tx
            .clone()
            .inputs
            .into_iter()
            .map(|input| input.extract_address())
            .collect();

        // check if vault's btc address features in an input of the transaction
        ensure!(
            // TODO: can a vault steal funds if it registers a P2WPKH-P2SH since we
            // would extract the `P2WPKHv0`?
            input_addresses.into_iter().any(|address_result| {
                match address_result {
                    Ok(address) => vault.wallet.has_btc_address(&address),
                    _ => false,
                }
            }),
            // since the transaction does not have any inputs that correspond
            // to any of the vault's registered BTC addresses, return Err
            Error::<T>::VaultNoInputToTransaction
        );

        // Vaults are required to move funds for redeem and replace operations.
        // Each transaction MUST feature at least two or three outputs as follows:
        // * recipient: the recipient of the redeem / replace
        // * op_return: the associated ID encoded in the OP_RETURN
        // * vault: any "spare change" the vault is transferring

        // should only err if there are too many outputs
        if let Ok((payments, op_returns)) = ext::btc_relay::extract_outputs::<T>(tx) {
            // check if the transaction is a "migration"
            ensure!(
                !Self::is_valid_merge_transaction(&payments, &op_returns, &vault.wallet),
                Error::<T>::ValidMergeTransaction
            );

            // we only expect one op_return output, the op_return output should not burn value, and
            // the request_id is expected to be 32 bytes
            if op_returns.len() != 1 || op_returns[0].0 > 0 || op_returns[0].1.len() < 32 {
                return Ok(());
            }

            // op_return can be up to 83 bytes so slice first 32
            let request_id = H256::from_slice(&op_returns[0].1[..32]);

            // redeem requests
            if let Ok(req) = ext::redeem::get_open_or_completed_redeem_request_from_id::<T>(&request_id) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount_btc, req.btc_address, &payments, &vault.wallet,),
                    Error::<T>::ValidRedeemTransaction
                );
            };

            // replace requests
            if let Ok(req) = ext::replace::get_open_or_completed_replace_request::<T>(&request_id) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount, req.btc_address, &payments, &vault.wallet,),
                    Error::<T>::ValidReplaceTransaction
                );
            };

            // refund requests
            if let Ok(req) = ext::refund::get_open_or_completed_refund_request_from_id::<T>(&request_id) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount_wrapped, req.btc_address, &payments, &vault.wallet,),
                    Error::<T>::ValidRefundTransaction
                );
            };
        }

        Ok(())
    }
}
