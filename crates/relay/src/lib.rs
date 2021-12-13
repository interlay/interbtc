//! # Relay Pallet
//! Based on the [specification](https://spec.interlay.io/spec/relay.html).

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

use crate::types::BalanceOf;
use bitcoin::{parser::parse_transaction, types::*};
use btc_relay::{types::OpReturnPaymentData, BtcAddress};
use frame_support::{dispatch::DispatchResult, ensure, transactional, weights::Pays};
use frame_system::ensure_signed;
use sp_std::{
    convert::{TryFrom, TryInto},
    vec::Vec,
};
use types::DefaultVaultId;
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
        + vault_registry::Config
        + btc_relay::Config
        + redeem::Config
        + replace::Config
        + refund::Config
        + fee::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        VaultTheft {
            vault_id: DefaultVaultId<T>,
            tx_id: H256Le,
        },
        VaultDoublePayment {
            vault_id: DefaultVaultId<T>,
            tx_id_1: H256Le,
            tx_id_2: H256Le,
        },
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
        /// Expected two unique transactions
        DuplicateTransaction,
        /// Expected duplicate OP_RETURN ids
        ExpectedDuplicate,
    }

    /// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
    /// identifiers of Vaults accused of theft.
    #[pallet::storage]
    #[pallet::getter(fn theft_report)]
    pub(super) type TheftReports<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, DefaultVaultId<T>, Blake2_128Concat, H256Le, Option<()>, ValueQuery>;

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
        pub fn initialize(
            origin: OriginFor<T>,
            raw_block_header: RawBlockHeader,
            block_height: u32,
        ) -> DispatchResultWithPostInfo {
            let relayer = ensure_signed(origin)?;

            let block_header = ext::btc_relay::parse_raw_block_header::<T>(&raw_block_header)?;
            ext::btc_relay::initialize::<T>(relayer, block_header, block_height)?;

            // don't take tx fees on success
            Ok(Pays::No.into())
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
        /// - Events:
        /// 	- One event for block stored (fork or extension).
        ///
        /// Total Complexity: O(C + P)
        /// # </weight>
        #[pallet::weight(<T as Config>::WeightInfo::store_block_header())]
        #[transactional]
        pub fn store_block_header(
            origin: OriginFor<T>,
            raw_block_header: RawBlockHeader,
        ) -> DispatchResultWithPostInfo {
            let relayer = ensure_signed(origin)?;

            let block_header = ext::btc_relay::parse_raw_block_header::<T>(&raw_block_header)?;
            ext::btc_relay::store_block_header::<T>(&relayer, block_header)?;

            // don't take tx fees on success
            Ok(Pays::No.into())
        }

        /// Report misbehavior by a Vault, providing a fraud proof (malicious Bitcoin transaction
        /// and the corresponding transaction inclusion proof). This fully slashes the Vault.
        ///
        /// # Arguments
        ///
        /// * `origin`: Any signed user.
        /// * `vault_id`: The account of the vault to check.
        /// * `raw_merkle_proof`: The proof of tx inclusion.
        /// * `raw_tx`: The raw Bitcoin transaction.
        #[pallet::weight(<T as Config>::WeightInfo::report_vault_theft())]
        #[transactional]
        pub fn report_vault_theft(
            origin: OriginFor<T>,
            vault_id: DefaultVaultId<T>,
            raw_merkle_proof: Vec<u8>,
            raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let reporter_id = ensure_signed(origin)?;

            let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(&raw_merkle_proof)?;
            let transaction = parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;
            let tx_id = transaction.tx_id();

            // throw if already reported
            ensure!(
                !<TheftReports<T>>::contains_key(&vault_id, &tx_id),
                Error::<T>::VaultAlreadyReported,
            );

            ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
            Self::_is_parsed_transaction_invalid(&vault_id, transaction)?;

            ext::vault_registry::liquidate_theft_vault::<T>(&vault_id, reporter_id)?;

            <TheftReports<T>>::mutate(&vault_id, &tx_id, |inner| {
                let _ = inner.insert(());
            });

            Self::deposit_event(Event::<T>::VaultTheft { vault_id, tx_id });

            // don't take tx fees on success
            Ok(Pays::No.into())
        }

        /// Report Vault double payment, providing two fraud proofs (malicious Bitcoin transactions
        /// and the corresponding transaction inclusion proofs). This fully slashes the Vault.
        ///
        /// This can be used for any multiple of payments, i.e., a vault making two, three, four, etc. payments
        /// by proving just one double payment.
        ///
        /// # Arguments
        ///
        /// * `origin`: Any signed user.
        /// * `vault_id`: The account of the vault to check.
        /// * `raw_merkle_proofs`: The proofs of tx inclusion.
        /// * `raw_txs`: The raw Bitcoin transactions.
        #[pallet::weight(<T as Config>::WeightInfo::report_vault_theft())]
        #[transactional]
        pub fn report_vault_double_payment(
            origin: OriginFor<T>,
            vault_id: DefaultVaultId<T>,
            raw_merkle_proofs: (Vec<u8>, Vec<u8>),
            raw_txs: (Vec<u8>, Vec<u8>),
        ) -> DispatchResultWithPostInfo {
            let reporter_id = ensure_signed(origin)?;

            // transactions must be unique
            ensure!(raw_txs.0 != raw_txs.1, Error::<T>::DuplicateTransaction);

            let parse_and_verify = |raw_tx, raw_proof| -> Result<Transaction, DispatchError> {
                let merkle_proof = ext::btc_relay::parse_merkle_proof::<T>(raw_proof)?;
                let transaction = parse_transaction(raw_tx).map_err(|_| Error::<T>::InvalidTransaction)?;
                // ensure transaction is included
                ext::btc_relay::verify_transaction_inclusion::<T>(transaction.tx_id(), merkle_proof)?;
                Ok(transaction)
            };

            let left_tx = parse_and_verify(&raw_txs.0, &raw_merkle_proofs.0)?;
            let right_tx = parse_and_verify(&raw_txs.1, &raw_merkle_proofs.1)?;

            let left_tx_id = left_tx.tx_id();
            let right_tx_id = right_tx.tx_id();

            let vault = ext::vault_registry::get_active_vault_from_id::<T>(&vault_id)?;
            // ensure that the payment is made from one of the registered wallets of the Vault,
            // this prevents a transaction with the same OP_RETURN flagging this Vault for theft
            ensure!(
                Self::has_input_from_wallet(&left_tx, &vault.wallet)
                    && Self::has_input_from_wallet(&right_tx, &vault.wallet),
                Error::<T>::VaultNoInputToTransaction
            );

            match (
                OpReturnPaymentData::<T>::try_from(left_tx),
                OpReturnPaymentData::<T>::try_from(right_tx),
            ) {
                (Ok(left), Ok(right)) => {
                    // verify that the OP_RETURN matches, amounts are not relevant as Vaults
                    // might transfer any amount in the theft transaction
                    ensure!(left.op_return == right.op_return, Error::<T>::ExpectedDuplicate);

                    ext::vault_registry::liquidate_theft_vault::<T>(&vault_id, reporter_id)?;

                    <TheftReports<T>>::mutate(&vault_id, &left_tx_id, |inner| {
                        let _ = inner.insert(());
                    });
                    <TheftReports<T>>::mutate(&vault_id, &right_tx_id, |inner| {
                        let _ = inner.insert(());
                    });

                    Self::deposit_event(Event::<T>::VaultDoublePayment {
                        vault_id,
                        tx_id_1: left_tx_id,
                        tx_id_2: right_tx_id,
                    });

                    // don't take tx fees on success
                    Ok(Pays::No.into())
                }
                _ => Err(Error::<T>::InvalidTransaction.into()),
            }
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    pub(crate) fn has_input_from_wallet(transaction: &Transaction, wallet: &Wallet) -> bool {
        // collect all addresses that feature in the inputs of the transaction
        let input_addresses: Vec<Result<BtcAddress, _>> = transaction
            .clone()
            .inputs
            .into_iter()
            .map(|input| input.extract_address())
            .collect();

        // TODO: can a vault steal funds if it registers a P2WPKH-P2SH since we
        // would extract the `P2WPKHv0`?
        input_addresses.into_iter().any(|address_result| match address_result {
            Ok(address) => wallet.has_btc_address(&address),
            _ => false,
        })
    }

    /// Checks if the vault is doing a valid merge transaction to move funds between
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `transaction` - the tx
    /// * `wallet` - vault btc addresses
    pub(crate) fn is_valid_merge_transaction(transaction: &Transaction, wallet: &Wallet) -> bool {
        return transaction
            .outputs
            .iter()
            .all(|output| matches!(output.extract_address(), Ok(addr) if wallet.has_btc_address(&addr)));
    }

    /// Checks if the vault is sending a valid request transaction.
    ///
    /// # Arguments
    ///
    /// * `request_value` - amount of btc as specified in the request
    /// * `request_address` - recipient btc address
    /// * `payment_data` - all payment data extracted from tx
    /// * `wallet` - vault btc addresses
    pub(crate) fn is_valid_request_transaction(
        request_value: BalanceOf<T>,
        request_address: BtcAddress,
        payment_data: &OpReturnPaymentData<T>,
        wallet: &Wallet,
    ) -> bool {
        let request_value = match TryInto::<u64>::try_into(request_value).map_err(|_e| Error::<T>::TryIntoIntError) {
            Ok(value) => value as i64,
            Err(_) => return false,
        };

        match payment_data.ensure_valid_payment_to(request_value, request_address, None) {
            Ok(None) => true,
            Ok(Some(return_to_self)) if wallet.has_btc_address(&return_to_self) => true,
            _ => false,
        }
    }

    /// Check if a vault transaction is invalid. Returns `Ok` if invalid or `Err` otherwise.
    /// This method should be callable over RPC for a staked-relayer client to check validity.
    ///
    /// # Arguments
    ///
    /// `vault_id`: the vault.
    /// `raw_tx`: the BTC transaction by the vault.
    pub fn is_transaction_invalid(vault_id: &DefaultVaultId<T>, raw_tx: Vec<u8>) -> DispatchResult {
        let tx = parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;
        Self::_is_parsed_transaction_invalid(vault_id, tx)
    }

    /// Check if a vault transaction is invalid. Returns `Ok` if invalid or `Err` otherwise.
    pub fn _is_parsed_transaction_invalid(vault_id: &DefaultVaultId<T>, tx: Transaction) -> DispatchResult {
        let vault = ext::vault_registry::get_active_vault_from_id::<T>(vault_id)?;

        // check if vault's btc address features in an input of the transaction
        ensure!(
            Self::has_input_from_wallet(&tx, &vault.wallet),
            // since the transaction does not have any inputs that correspond
            // to any of the vault's registered BTC addresses, return Err
            Error::<T>::VaultNoInputToTransaction
        );

        // Vaults are required to move funds for redeem, replace and refund operations.
        // Each transaction MUST feature at least two or three outputs as follows:
        // * recipient: the recipient of the redeem / replace
        // * op_return: the associated ID encoded in the OP_RETURN
        // * vault: any "spare change" the vault is transferring

        ensure!(
            !Self::is_valid_merge_transaction(&tx, &vault.wallet),
            Error::<T>::ValidMergeTransaction
        );

        if let Ok(payment_data) = OpReturnPaymentData::<T>::try_from(tx) {
            // redeem requests
            if let Ok(req) = ext::redeem::get_open_or_completed_redeem_request_from_id::<T>(&payment_data.op_return) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount_btc, req.btc_address, &payment_data, &vault.wallet),
                    Error::<T>::ValidRedeemTransaction
                );
            };

            // replace requests
            if let Ok(req) = ext::replace::get_open_or_completed_replace_request::<T>(&payment_data.op_return) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount, req.btc_address, &payment_data, &vault.wallet),
                    Error::<T>::ValidReplaceTransaction
                );
            };

            // refund requests
            if let Ok(req) = ext::refund::get_open_or_completed_refund_request_from_id::<T>(&payment_data.op_return) {
                ensure!(
                    !Self::is_valid_request_transaction(req.amount_btc, req.btc_address, &payment_data, &vault.wallet),
                    Error::<T>::ValidRefundTransaction
                );
            };
        }

        Ok(())
    }
}
