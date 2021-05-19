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

use crate::types::{Backing, Issuing};
use bitcoin::{parser::parse_transaction, types::*};

use btc_relay::{BtcAddress, Error as BtcRelayError};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    transactional,
};
use frame_system::{ensure_root, ensure_signed};
use sp_core::H256;

use sp_std::{collections::btree_set::BTreeSet, convert::TryInto, vec::Vec};
use vault_registry::Wallet;

/// ## Configuration
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + security::Config
    + currency::Config<currency::Backing>
    + currency::Config<currency::Issuing>
    + vault_registry::Config
    + btc_relay::Config
    + redeem::Config
    + replace::Config
    + refund::Config
    + sla::Config
    + fee::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;

    /// The minimum amount of deposit required to propose an update.
    type MinimumDeposit: Get<Backing<Self>>;

    /// The minimum amount of stake required to participate.
    type MinimumStake: Get<Backing<Self>>;

    /// How often (in blocks) to check for new votes.
    type VotingPeriod: Get<Self::BlockNumber>;

    /// Maximum message size in bytes
    type MaximumMessageSize: Get<u32>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Staking {
        Stakes get(fn stakes): map hasher(blake2_128_concat) T::AccountId => Backing<T>;

        /// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
        /// identifiers of Vaults accused of theft.
        TheftReports get(fn theft_report): map hasher(blake2_128_concat) H256Le => BTreeSet<T::AccountId>;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

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
        #[weight = <T as Config>::WeightInfo::initialize()]
        #[transactional]
        fn initialize(
            origin,
            raw_block_header: RawBlockHeader,
            block_height: u32)
            -> DispatchResult
        {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let relayer = ensure_signed(origin)?;
            Self::ensure_relayer_is_registered(&relayer)?;
            ext::btc_relay::initialize::<T>(relayer, raw_block_header, block_height)
        }

        /// Registers a new Staked Relayer, locking the provided collateral, which must exceed `STAKED_RELAYER_STAKE`.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be registered
        /// * `stake`: to-be-locked collateral/stake in Backing
        #[weight = <T as Config>::WeightInfo::register_staked_relayer()]
        #[transactional]
        fn register_staked_relayer(origin, #[compact] stake: Backing<T>) -> DispatchResult {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;

            ensure!(
                !<Stakes<T>>::contains_key(&signer),
                Error::<T>::AlreadyRegistered,
            );

            ensure!(
                stake >= T::MinimumStake::get(),
                Error::<T>::InsufficientStake,
            );
            ext::collateral::lock_collateral::<T>(&signer, stake)?;

            <Stakes<T>>::insert(&signer, stake);

            Self::deposit_event(<Event<T>>::RegisterStakedRelayer(signer, stake));
            Ok(())
        }

        /// Deregisters a Staked Relayer, releasing the associated stake.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be deregistered
        #[weight = <T as Config>::WeightInfo::deregister_staked_relayer()]
        #[transactional]
        fn deregister_staked_relayer(origin) -> DispatchResult {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;

            ensure!(<Stakes<T>>::contains_key(&signer), Error::<T>::NotRegistered);

            // `take` also removes it from storage
            let stake = <Stakes<T>>::take(&signer);

            ext::collateral::release_collateral::<T>(&signer, stake)?;

            Self::deposit_event(<Event<T>>::DeregisterStakedRelayer(signer));
            Ok(())
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
        #[weight = <T as Config>::WeightInfo::store_block_header()]
        #[transactional]
        fn store_block_header(
            origin, raw_block_header: RawBlockHeader
        ) -> DispatchResult {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let relayer = ensure_signed(origin)?;

            Self::ensure_relayer_is_registered(&relayer)?;
            Self::store_block_header_and_update_sla(&relayer, raw_block_header)
        }

        /// Slashes the stake/collateral of a Staked Relayer and removes them from the list.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Governance Mechanism.
        /// * `staked_relayer_id`: The account of the Staked Relayer to be slashed.
        #[weight = <T as Config>::WeightInfo::slash_staked_relayer()]
        #[transactional]
        fn slash_staked_relayer(origin, staked_relayer_id: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(<Stakes<T>>::contains_key(&staked_relayer_id), Error::<T>::NotRegistered);

            // `take` also removes it from storage
            let stake = <Stakes<T>>::take(&staked_relayer_id);

            ext::collateral::slash_collateral::<T>(staked_relayer_id.clone(), ext::fee::fee_pool_account_id::<T>(), stake)?;

            Self::deposit_event(<Event<T>>::SlashStakedRelayer(
                staked_relayer_id,
            ));

            Ok(())
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
        #[weight = <T as Config>::WeightInfo::report_vault_theft()]
        #[transactional]
        fn report_vault_theft(origin, vault_id: T::AccountId, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
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

            // if the report is made by a relayer, reward it by increasing its sla
            if Self::relayer_is_registered(&signer) {
                ext::sla::event_update_relayer_sla::<T>(&signer, ext::sla::RelayerEvent::CorrectTheftReport)?;
            }

            Self::deposit_event(<Event<T>>::VaultTheft(
                vault_id,
                tx_id
            ));

            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn store_block_header_and_update_sla(relayer: &T::AccountId, raw_block_header: RawBlockHeader) -> DispatchResult {
        match ext::btc_relay::store_block_header::<T>(relayer, raw_block_header) {
            Ok(_) => {
                ext::sla::event_update_relayer_sla::<T>(relayer, ext::sla::RelayerEvent::BlockSubmission)?;
                Ok(())
            }
            Err(err) if err == DispatchError::from(BtcRelayError::<T>::DuplicateBlock) => {
                ext::sla::event_update_relayer_sla::<T>(relayer, ext::sla::RelayerEvent::DuplicateBlockSubmission)?;
                Ok(())
            }
            x => x,
        }
    }

    fn relayer_is_registered(id: &T::AccountId) -> bool {
        <Stakes<T>>::contains_key(id)
    }

    /// Ensure a staked relayer is registered.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn ensure_relayer_is_registered(id: &T::AccountId) -> DispatchResult {
        if Self::relayer_is_registered(id) {
            Ok(())
        } else {
            Err(Error::<T>::NotRegistered.into())
        }
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
        request_value: Issuing<T>,
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
                    !Self::is_valid_request_transaction(req.amount_issuing, req.btc_address, &payments, &vault.wallet,),
                    Error::<T>::ValidRefundTransaction
                );
            };
        }

        Ok(())
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Backing = Backing<T>,
    {
        RegisterStakedRelayer(AccountId, Backing),
        DeregisterStakedRelayer(AccountId),
        SlashStakedRelayer(AccountId),
        VaultTheft(AccountId, H256Le),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Staked relayer is already registered
        AlreadyRegistered,
        /// Insufficient collateral staked
        InsufficientStake,
        /// Participant is not registered
        NotRegistered,
        /// Vault already reported
        VaultAlreadyReported,
        /// Vault already liquidated
        VaultAlreadyLiquidated,
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
        /// Block not included by the relay
        BlockNotFound,
        /// Block already reported
        BlockAlreadyReported,
        /// Cannot report vault theft without block hash
        ExpectedBlockHash,
        /// Status update should not contain block hash
        UnexpectedBlockHash,
        /// Failed to parse transaction
        InvalidTransaction,
        /// Unable to convert value
        TryIntoIntError,
    }
}
