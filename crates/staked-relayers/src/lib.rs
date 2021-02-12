#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod types;

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

pub use security;

use crate::types::{
    PolkaBTC, ProposalStatus, StakedRelayer, StatusUpdate, StatusUpdateId, Tally, Votes, DOT,
};
use bitcoin::parser::parse_transaction;
use bitcoin::types::*;
use btc_relay::BtcAddress;
use util::transactional;

/// # Staked Relayers module implementation
/// This is the implementation of the BTC Parachain Staked Relayers module following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/staked-relayers.html
///
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    weights::Weight,
    IterableStorageMap,
};
use frame_system::{ensure_root, ensure_signed};
use primitive_types::H256;
use security::types::{ErrorCode, StatusCode};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use vault_registry::Wallet;

pub trait WeightInfo {
    fn register_staked_relayer() -> Weight;
    fn deregister_staked_relayer() -> Weight;
    fn suggest_status_update() -> Weight;
    fn vote_on_status_update() -> Weight;
    fn force_status_update() -> Weight;
    fn slash_staked_relayer() -> Weight;
    fn report_vault_theft() -> Weight;
    fn report_vault_under_liquidation_threshold() -> Weight;
    fn remove_active_status_update() -> Weight;
    fn remove_inactive_status_update() -> Weight;
    fn set_maturity_period() -> Weight;
    fn evaluate_status_update() -> Weight;
}

/// ## Configuration
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + security::Config
    + collateral::Config
    + vault_registry::Config
    + btc_relay::Config
    + redeem::Config
    + replace::Config
    + refund::Config
    + sla::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;

    /// The minimum amount of deposit required to propose an update.
    type MinimumDeposit: Get<DOT<Self>>;

    /// The minimum amount of stake required to participate.
    type MinimumStake: Get<DOT<Self>>;

    /// How often (in blocks) to check for new votes.
    type VotingPeriod: Get<Self::BlockNumber>;

    /// Maximum message size in bytes
    type MaximumMessageSize: Get<u32>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Staking {
        /// Mapping from accounts of active staked relayers to the StakedRelayer struct.
        ActiveStakedRelayers get(fn active_staked_relayer): map hasher(blake2_128_concat) T::AccountId => StakedRelayer<DOT<T>, T::BlockNumber>;

        /// Mapping from accounts of inactive staked relayers to the StakedRelayer struct.
        InactiveStakedRelayers get(fn inactive_staked_relayer): map hasher(blake2_128_concat) T::AccountId => StakedRelayer<DOT<T>, T::BlockNumber>;

        /// Map of active StatusUpdates, identified by an integer key.
        ActiveStatusUpdates get(fn active_status_update): map hasher(blake2_128_concat) u64 => StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>;

        /// Map of executed or rejected StatusUpdates, identified by an integer key.
        InactiveStatusUpdates get(fn inactive_status_update): map hasher(blake2_128_concat) u64 => StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>;

        /// Integer increment-only counter used to track status updates.
        StatusCounter get(fn status_counter): u64;

        /// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
        /// identifiers of Vaults accused of theft.
        TheftReports get(fn theft_report): map hasher(blake2_128_concat) H256Le => BTreeSet<T::AccountId>;

        /// Mapping of Bitcoin block hashes to status update ids.
        BlockReports get(fn block_report): map hasher(blake2_128_concat) H256Le => u64;

        /// AccountId of the governance mechanism, as specified in the genesis.
        GovernanceId get(fn gov_id) config(): T::AccountId;

        /// Number of blocks to wait until eligible to vote.
        MaturityPeriod get(fn maturity_period) config(): T::BlockNumber;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        const MinimumDeposit: DOT<T> = T::MinimumDeposit::get();

        const MinimumStake: DOT<T> = T::MinimumStake::get();

        const VotingPeriod: T::BlockNumber = T::VotingPeriod::get();

        const MaximumMessageSize: u32 = T::MaximumMessageSize::get();

        fn deposit_event() = default;

        /// Registers a new Staked Relayer, locking the provided collateral, which must exceed `STAKED_RELAYER_STAKE`.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be registered
        /// * `stake`: to-be-locked collateral/stake in DOT
        #[weight = <T as Config>::WeightInfo::register_staked_relayer()]
        #[transactional]
        fn register_staked_relayer(origin, stake: DOT<T>) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            ensure!(
                !<ActiveStakedRelayers<T>>::contains_key(&signer),
                Error::<T>::AlreadyRegistered,
            );

            ensure!(
                !<InactiveStakedRelayers<T>>::contains_key(&signer),
                Error::<T>::AlreadyRegistered,
            );

            ensure!(
                stake >= T::MinimumStake::get(),
                Error::<T>::InsufficientStake,
            );

            ext::collateral::lock_collateral::<T>(&signer, stake)?;
            let height = <frame_system::Module<T>>::block_number();
            let maturity = height + Self::get_maturity_period();
            Self::insert_inactive_staked_relayer(&signer, stake, maturity);
            Self::deposit_event(<Event<T>>::RegisterStakedRelayer(signer, maturity, stake));
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
            let signer = ensure_signed(origin)?;
            let staked_relayer = Self::get_active_staked_relayer(&signer)?;
            Self::ensure_staked_relayer_is_not_active(&signer)?;
            ext::collateral::release_collateral::<T>(&signer, staked_relayer.stake)?;
            // TODO: require unbonding period
            Self::remove_active_staked_relayer(&signer);
            Self::deposit_event(<Event<T>>::DeregisterStakedRelayer(signer));
            Ok(())
        }

        /// Suggest a new status update and opens it up for voting.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Staked Relayer suggesting the status change.
        /// * `status_code`: Suggested BTC Parachain status (StatusCode enum).
        /// * `add_error`: [Optional] If the suggested status is Error, this set of ErrorCode indicates which error is to be added to the Errors mapping.
        /// * `remove_error`: [Optional] ErrorCode to be removed from the Errors list.
        /// * `block_hash`: [Optional] When reporting an error related to BTC-Relay, this field indicates the affected Bitcoin block (header).
        /// * `message`: Message detailing reason for status update
        #[weight = <T as Config>::WeightInfo::suggest_status_update()]
        #[transactional]
        fn suggest_status_update(origin, deposit: DOT<T>, status_code: StatusCode, add_error: Option<ErrorCode>, remove_error: Option<ErrorCode>, block_hash: Option<H256Le>, message: Vec<u8>) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            if status_code == StatusCode::Shutdown {
                Self::only_governance(&signer)?;
            }

            ensure!(
                message.len() as u32 <= T::MaximumMessageSize::get(),
                Error::<T>::MessageTooBig,
            );

            // this call should revert if not registered
            let staked_relayer = Self::get_active_staked_relayer(&signer)?;

            ensure!(
                deposit >= T::MinimumDeposit::get(),
                Error::<T>::InsufficientDeposit,
            );
            ext::collateral::lock_collateral::<T>(&signer, deposit)?;

            if let Some(ref add_error) = add_error {
                match add_error {
                    ErrorCode::NoDataBTCRelay => {
                        match block_hash {
                            Some(block_hash) => {
                                ensure!(
                                    !<BlockReports>::contains_key(block_hash),
                                    Error::<T>::BlockAlreadyReported
                                );
                            }
                            None => {
                                return Err(Error::<T>::ExpectedBlockHash.into());
                            }
                        };
                    },
                    ErrorCode::InvalidBTCRelay => {
                        match block_hash {
                            Some(block_hash) => {
                                ensure!(
                                    !<BlockReports>::contains_key(block_hash),
                                    Error::<T>::BlockAlreadyReported
                                );
                            }
                            None => {
                                return Err(Error::<T>::ExpectedBlockHash.into());
                            }
                        };
                    }
                    _ => {
                        ensure!(
                            block_hash.is_none(),
                            Error::<T>::UnexpectedBlockHash
                        );

                    }
                }
            }

            if let Some(block_hash) = block_hash {
                ensure!(
                    ext::btc_relay::block_header_exists::<T>(block_hash),
                    Error::<T>::BlockNotFound,
                );
            }

            // pre-approve
            let mut tally = Tally::default();
            tally.aye.insert(signer.clone(), staked_relayer.stake);

            let height = <frame_system::Module<T>>::block_number();
            let status_update_id = Self::insert_active_status_update(StatusUpdate{
                new_status_code: status_code.clone(),
                old_status_code: ext::security::get_parachain_status::<T>(),
                add_error: add_error.clone(),
                remove_error: remove_error.clone(),
                start: height,
                end: height + T::VotingPeriod::get(),
                proposal_status: ProposalStatus::Pending,
                btc_block_hash: block_hash,
                proposer: signer.clone(),
                deposit: deposit,
                tally: tally,
                message: message,
            });

            Self::deposit_event(<Event<T>>::StatusUpdateSuggested(status_update_id, signer, status_code, add_error, remove_error, block_hash));
            Ok(())
        }

        /// A Staked Relayer casts a vote on a suggested `StatusUpdate`. Checks the threshold
        /// of votes and executes / cancels a `StatusUpdate` depending on the threshold reached.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Staked Relayer casting the vote.
        /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `ActiveStatusUpdates`.
        /// * `approve`: `True` or `False`, depending on whether the Staked Relayer agrees or disagrees with the suggested `StatusUpdate`.
        #[weight = <T as Config>::WeightInfo::vote_on_status_update()]
        #[transactional]
        fn vote_on_status_update(origin, status_update_id: StatusUpdateId, approve: bool) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            // this call should revert if the signer is not registered
            let staked_relayer = Self::get_active_staked_relayer(&signer)?;

            let mut update = Self::get_status_update(&status_update_id)?;
            ensure!(
                update.tally.vote(signer.clone(), staked_relayer.stake, approve),
                Error::<T>::VoteAlreadyCast,
            );
            <ActiveStatusUpdates<T>>::insert(&status_update_id, &update);

            Self::deposit_event(<Event<T>>::VoteOnStatusUpdate(status_update_id, signer, approve));

            Ok(())
        }

        /// This function can only be called by the Governance Mechanism.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Governance Mechanism.
        /// * `status_code`: Suggested BTC Parachain status (`StatusCode` enum).
        /// * `errors`: If the suggested status is `Error`, this set of `ErrorCode` entries provides details on the occurred errors.
        #[weight = <T as Config>::WeightInfo::force_status_update()]
        #[transactional]
        fn force_status_update(origin, status_code: StatusCode, add_error: Option<ErrorCode>, remove_error: Option<ErrorCode>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            Self::only_governance(&signer)?;
            ext::security::set_parachain_status::<T>(status_code.clone());

            let to_add = add_error.clone();
            let to_remove = remove_error.clone();

            if let Some(error_code) = to_add {
                ext::security::insert_error::<T>(error_code);
            }

            if let Some(error_code) = to_remove {
                ext::security::remove_error::<T>(error_code);
            }

            Self::deposit_event(<Event<T>>::ForceStatusUpdate(
                status_code,
                add_error,
                remove_error,
            ));
            Ok(())
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
            let signer = ensure_signed(origin)?;
            Self::only_governance(&signer)?;

            let staked_relayer = Self::get_active_staked_relayer(&staked_relayer_id)?;
            ext::collateral::slash_collateral::<T>(staked_relayer_id.clone(), signer, staked_relayer.stake)?;
            Self::remove_active_staked_relayer(&staked_relayer_id);

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
        fn report_vault_theft(origin, vault_id: T::AccountId, tx_id: H256Le, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            Self::ensure_relayer_is_registered(&signer)?;

            // liquidated vaults are removed, so no need for check here

            // throw if already reported
            if <TheftReports<T>>::contains_key(&tx_id) {
                ensure!(
                    !<TheftReports<T>>::get(&tx_id).contains(&vault_id),
                    Error::<T>::VaultAlreadyReported,
                );
            }

            ext::btc_relay::verify_transaction_inclusion::<T>(tx_id, merkle_proof)?;
            Self::is_transaction_invalid(&vault_id, raw_tx)?;

            ext::vault_registry::liquidate_theft_vault::<T>(&vault_id)?;

            <TheftReports<T>>::mutate(&tx_id, |reports| {
                reports.insert(vault_id.clone());
            });

            // reward relayer for this report by increasing its sla
            ext::sla::event_update_relayer_sla::<T>(signer, ext::sla::RelayerEvent::CorrectTheftReport)?;

            Self::deposit_event(<Event<T>>::VaultTheft(
                vault_id,
                tx_id
            ));

            Ok(())
        }

        /// A Staked Relayer reports that a Vault is undercollateralized (i.e. below the LiquidationCollateralThreshold as defined in Vault Registry).
        /// If the collateral falls below this rate, we flag the Vault for liquidation.
        #[weight = <T as Config>::WeightInfo::report_vault_under_liquidation_threshold()]
        #[transactional]
        fn report_vault_under_liquidation_threshold(origin, vault_id: T::AccountId)  -> DispatchResult {
            let signer = ensure_signed(origin)?;
            Self::ensure_relayer_is_registered(&signer)?;

            // ensure that the vault is eligible for liquidation
            ensure!(
                // NOTE: the liquidation threshold expresses the percentage of collateral
                // required for the vault relative to the exchange rate. If the vault is
                // under this percentage it is flagged for liquidation.
                ext::vault_registry::is_vault_below_liquidation_threshold::<T>(&vault_id)?,
                Error::<T>::CollateralOk,
            );

            ext::vault_registry::liquidate_vault::<T>(&vault_id)?;

            // reward relayer for this report by increasing its sla
            ext::sla::event_update_relayer_sla::<T>(signer, ext::sla::RelayerEvent::CorrectLiquidationReport)?;

            Self::deposit_event(<Event<T>>::VaultUnderLiquidationThreshold(
                vault_id
            ));

            Ok(())
        }

        /// A Staked Relayer reports that the Exchange Rate Oracle is offline. This function checks if the last exchange
        /// rate data in the Exchange Rate Oracle is indeed older than the indicated threshold.
        #[weight = 1000]
        #[transactional]
        fn report_oracle_offline(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            Self::ensure_relayer_is_registered(&signer)?;

            ensure!(
                !ext::security::get_errors::<T>().contains(&ErrorCode::OracleOffline),
                Error::<T>::OracleAlreadyReported,
            );

            ensure!(
                ext::oracle::is_max_delay_passed::<T>(),
                Error::<T>::OracleOnline,
            );

            ext::security::set_parachain_status::<T>(StatusCode::Error);
            ext::security::insert_error::<T>(ErrorCode::OracleOffline);

            // reward relayer for this report by increasing its sla
            ext::sla::event_update_relayer_sla::<T>(signer, ext::sla::RelayerEvent::CorrectOracleOfflineReport)?;

            Self::deposit_event(<Event<T>>::OracleOffline());

            Ok(())
        }

        /// Permanently remove an `ActiveStatusUpdate`.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `status_update_id` - id of the active status update to remove
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::remove_active_status_update()]
        #[transactional]
        fn remove_active_status_update(origin, status_update_id: StatusUpdateId) {
            ensure_root(origin)?;
            <ActiveStatusUpdates<T>>::remove(status_update_id);
        }

        /// Permanently remove an `InactiveStatusUpdate`.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `status_update_id` - id of the inactive status update to remove
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::remove_inactive_status_update()]
        #[transactional]
        fn remove_inactive_status_update(origin, status_update_id: StatusUpdateId) {
            ensure_root(origin)?;
            <InactiveStatusUpdates<T>>::remove(status_update_id);
        }

        /// Sets the maturity period.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `period` - the number of blocks to wait before a relayer is considered active.
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::set_maturity_period()]
        #[transactional]
        fn set_maturity_period(origin, period: T::BlockNumber) {
            ensure_root(origin)?;
            <MaturityPeriod<T>>::set(period);
        }

        /// Calls evaluate_status_update_at_height, for testing purposes.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        ///
        /// # Weight: `O(1)`
        #[weight = <T as Config>::WeightInfo::evaluate_status_update()]
        #[transactional]
        fn evaluate_status_update(origin, status_update_id: StatusUpdateId) {
            ensure_root(origin)?;
            let mut status_update = Self::get_status_update(&status_update_id)?;
            Self::_evaluate_status_update(status_update_id, &mut status_update)?;
            <InactiveStatusUpdates<T>>::remove(status_update_id);
        }

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }

        fn on_finalize(n: T::BlockNumber) {
            Self::end_block(n)
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn begin_block(height: T::BlockNumber) -> DispatchResult {
        for (id, acc) in <InactiveStakedRelayers<T>>::iter() {
            let _ = Self::try_bond_staked_relayer(&id, acc.stake, height, acc.height);
        }
        Ok(())
    }

    fn end_block(height: T::BlockNumber) {
        <ActiveStatusUpdates<T>>::translate(
            |id, mut status_update: StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>| {
                match Self::evaluate_status_update_at_height(id, &mut status_update, height) {
                    // remove proposal
                    Ok(true) => None,
                    // proposal is not accepted or rejected
                    Ok(false) => Some(status_update),
                    // something went wrong, keep the proposal
                    Err(err) => {
                        sp_runtime::print(err);
                        Some(status_update)
                    }
                }
            },
        );
    }

    /// Checks if the given StatusUpdate has expired. If so, it evaluates it.
    /// Returns true if the `StatusUpdate` should be garbage collected.
    ///
    /// # Arguments
    ///
    /// * `id` - id of the `StatusUpdate`
    /// * `status_update` - `StatusUpdate` to evaluate
    /// * `height` - current height of the chain.
    fn evaluate_status_update_at_height(
        id: StatusUpdateId,
        status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
        height: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        if height >= status_update.end {
            Self::_evaluate_status_update(id, status_update)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Evaluates whether the `StatusUpdate` has been accepted or rejected.
    ///
    /// # Arguments
    ///
    /// * `id` - id of the `StatusUpdate`
    /// * `status_update` - `StatusUpdate` to evaluate
    fn _evaluate_status_update(
        id: StatusUpdateId,
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> Result<(), DispatchError> {
        if status_update.tally.is_approved() {
            Self::execute_status_update(id, &mut status_update)?;
            Self::update_sla_score_for_status_update(&status_update, true)?;
        } else {
            Self::reject_status_update(id, &mut status_update)?;
            Self::update_sla_score_for_status_update(&status_update, false)?;
        }
        Self::insert_inactive_status_update(id, status_update);

        Ok(())
    }

    /// Activate the staked relayer if mature.
    /// Used for external integration tests.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    pub fn activate_staked_relayer(id: &T::AccountId) -> DispatchResult {
        let staked_relayer = Self::get_inactive_staked_relayer(id)?;
        let height = <frame_system::Module<T>>::block_number();
        Self::try_bond_staked_relayer(id, staked_relayer.stake, height, staked_relayer.height)?;
        Ok(())
    }

    /// Activate the staked relayer if mature.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    /// * `stake` - amount of stake to deposit.
    /// * `height` - current height of the chain.
    /// * `maturity` - previous height + bonding period.
    fn try_bond_staked_relayer(
        id: &T::AccountId,
        stake: DOT<T>,
        height: T::BlockNumber,
        maturity: T::BlockNumber,
    ) -> DispatchResult {
        ensure!(height >= maturity, Error::<T>::NotMatured);
        Self::bond_staked_relayer(id, stake, height);
        Ok(())
    }

    /// Activate the staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    /// * `stake` - amount of stake to deposit.
    fn bond_staked_relayer(id: &T::AccountId, stake: DOT<T>, height: T::BlockNumber) {
        Self::insert_active_staked_relayer(id, stake, height);
        Self::remove_inactive_staked_relayer(id);
    }

    fn ensure_staked_relayer_is_not_active(id: &T::AccountId) -> DispatchResult {
        for (_, update) in <ActiveStatusUpdates<T>>::iter() {
            ensure!(!update.tally.contains(id), Error::<T>::StatusUpdateFound);
        }
        Ok(())
    }

    /// Should throw if not called by the governance account.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    pub(crate) fn only_governance(id: &T::AccountId) -> DispatchResult {
        ensure!(<GovernanceId<T>>::get() == *id, Error::<T>::GovernanceOnly);
        Ok(())
    }

    /// Ensure a staked relayer is registered.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn ensure_relayer_is_registered(id: &T::AccountId) -> DispatchResult {
        ensure!(
            <ActiveStakedRelayers<T>>::contains_key(id),
            Error::<T>::NotRegistered,
        );
        Ok(())
    }

    /// Gets the active staked relayer or throws an error.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn get_active_staked_relayer(
        id: &T::AccountId,
    ) -> Result<StakedRelayer<DOT<T>, T::BlockNumber>, DispatchError> {
        Self::ensure_relayer_is_registered(id)?;
        Ok(<ActiveStakedRelayers<T>>::get(id))
    }

    /// Gets the inactive staked relayer or throws an error.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    fn get_inactive_staked_relayer(
        id: &T::AccountId,
    ) -> Result<StakedRelayer<DOT<T>, T::BlockNumber>, DispatchError> {
        ensure!(
            <InactiveStakedRelayers<T>>::contains_key(id),
            Error::<T>::NotRegistered,
        );
        Ok(<InactiveStakedRelayers<T>>::get(id))
    }

    /// Creates an active staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    /// * `stake` - token deposited
    /// * `height` - bonding height
    pub(crate) fn insert_active_staked_relayer(
        id: &T::AccountId,
        stake: DOT<T>,
        height: T::BlockNumber,
    ) {
        <ActiveStakedRelayers<T>>::insert(id, StakedRelayer { stake, height });
    }

    /// Creates an inactive staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    /// * `stake` - token deposited
    /// * `height` - bonding height
    pub(crate) fn insert_inactive_staked_relayer(
        id: &T::AccountId,
        stake: DOT<T>,
        height: T::BlockNumber,
    ) {
        <InactiveStakedRelayers<T>>::insert(id, StakedRelayer { stake, height });
    }

    /// Removes an active staked relayer, decrementing the total count.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the relayer.
    fn remove_active_staked_relayer(id: &T::AccountId) {
        <ActiveStakedRelayers<T>>::remove(id);
    }

    /// Removes an inactive staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the relayer.
    fn remove_inactive_staked_relayer(id: &T::AccountId) {
        <InactiveStakedRelayers<T>>::remove(id);
    }

    /// Insert a new active status update and return the generated ID.
    ///
    /// # Arguments
    ///
    /// * `status_update` - `StatusUpdate` with the proposed changes.
    pub(crate) fn insert_active_status_update(
        status_update: StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> StatusUpdateId {
        let status_id = Self::get_status_counter();
        if let Some(block_hash) = status_update.btc_block_hash {
            // prevent duplicate blocks from being reported
            <BlockReports>::insert(block_hash, status_id);
        }
        <ActiveStatusUpdates<T>>::insert(&status_id, status_update);
        status_id
    }

    /// Insert a new inactive status update with an ID.
    ///
    /// # Arguments
    ///
    /// * `status_update` - `StatusUpdate` with the proposed changes.
    pub(crate) fn insert_inactive_status_update(
        status_id: StatusUpdateId,
        status_update: &StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) {
        <InactiveStatusUpdates<T>>::insert(&status_id, status_update);
    }

    /// Get an existing `StatusUpdate` or throw.
    ///
    /// # Arguments
    ///
    /// * `status_update_id` - id of the `StatusUpdate` to fetch.
    pub(crate) fn get_status_update(
        status_update_id: &StatusUpdateId,
    ) -> Result<StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>, DispatchError> {
        ensure!(
            <ActiveStatusUpdates<T>>::contains_key(status_update_id),
            Error::<T>::StatusUpdateNotFound,
        );
        Ok(<ActiveStatusUpdates<T>>::get(status_update_id))
    }

    /// Slash the stake of accounts who voted for an incorrect proposal,
    /// sending the funds to the governance account.
    ///
    /// # Arguments
    ///
    /// * `error` - optional errorcode
    /// * `votes` - vote set, includes account set and total stake
    fn slash_staked_relayers(
        error: &Option<ErrorCode>,
        votes: &Votes<T::AccountId, DOT<T>>,
    ) -> DispatchResult {
        if let Some(ErrorCode::NoDataBTCRelay) = error {
            // we don't slash participants for this
            return Ok(());
        }

        for acc in votes.accounts.iter() {
            // active participants are not allowed to deregister during
            // an ongoing status update, so this call should never revert
            let staked_relayer = Self::get_active_staked_relayer(acc)?;
            ext::collateral::slash_collateral::<T>(
                acc.clone(),
                <GovernanceId<T>>::get(),
                staked_relayer.stake,
            )?;
            Self::remove_active_staked_relayer(acc);
        }

        Ok(())
    }

    /// Update relayer SLA scores after a status update suggestion has been completed.
    ///
    /// # Arguments
    ///
    /// * `status_update` - the status update suggestion that was completed
    /// * `approved` - true iff the status update was accepted
    fn update_sla_score_for_status_update(
        status_update: &StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
        approved: bool,
    ) -> DispatchResult {
        let no_data_relayer = match (&status_update.add_error, &status_update.remove_error) {
            (&Some(ErrorCode::NoDataBTCRelay), _) | (_, &Some(ErrorCode::NoDataBTCRelay)) => true,
            _ => false,
        };
        let invalid_relayer = match (&status_update.add_error, &status_update.remove_error) {
            (&Some(ErrorCode::InvalidBTCRelay), _) | (_, &Some(ErrorCode::InvalidBTCRelay)) => true,
            _ => false,
        };

        let (correct_voters, incorrect_voters) = if approved {
            (
                status_update.tally.aye.accounts.iter(),
                status_update.tally.nay.accounts.iter(),
            )
        } else {
            (
                status_update.tally.nay.accounts.iter(),
                status_update.tally.aye.accounts.iter(),
            )
        };

        // reward relayers for correct votes by increasing their sla
        for relayer in correct_voters {
            if no_data_relayer {
                ext::sla::event_update_relayer_sla::<T>(
                    relayer.clone(),
                    ext::sla::RelayerEvent::CorrectNoDataVoteOrReport,
                )?;
            }
            if invalid_relayer {
                ext::sla::event_update_relayer_sla::<T>(
                    relayer.clone(),
                    ext::sla::RelayerEvent::CorrectInvalidVoteOrReport,
                )?;
            }
        }

        // punish relayers for incorrect votes by decreasing their sla
        for relayer in incorrect_voters {
            if no_data_relayer {
                ext::sla::event_update_relayer_sla::<T>(
                    relayer.clone(),
                    ext::sla::RelayerEvent::FalseNoDataVoteOrReport,
                )?;
            }
            if invalid_relayer {
                ext::sla::event_update_relayer_sla::<T>(
                    relayer.clone(),
                    ext::sla::RelayerEvent::FalseInvalidVoteOrReport,
                )?;
            }
        }

        // punish relayers that didn't vote by decreasing their sla
        let mut voters = status_update.tally.aye.accounts.clone();
        voters.append(&mut status_update.tally.nay.accounts.clone());
        let all_relayers: BTreeSet<_> = <ActiveStakedRelayers<T>>::iter()
            .map(|(relayer, _)| relayer)
            .collect();
        for abstainer in all_relayers.difference(&voters) {
            let staked_relayer = <ActiveStakedRelayers<T>>::get(abstainer);
            if staked_relayer.height > status_update.start {
                // skip participants who joined after this vote started
                continue;
            }

            ext::sla::event_update_relayer_sla::<T>(
                abstainer.clone(),
                ext::sla::RelayerEvent::IgnoredVote,
            )?;
        }

        Ok(())
    }

    /// Executes a `StatusUpdate` that has received sufficient “Yes” votes.
    ///
    /// # Arguments
    ///
    /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `ActiveStatusUpdates`.
    /// * `status_update`: `StatusUpdate` voted upon.
    fn execute_status_update(
        status_update_id: StatusUpdateId,
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> DispatchResult {
        ensure!(
            status_update.tally.is_approved(),
            Error::<T>::InsufficientYesVotes
        );

        let status_code = status_update.new_status_code.clone();
        ext::security::set_parachain_status::<T>(status_code.clone());

        let add_error = status_update.add_error.clone();
        let remove_error = status_update.remove_error.clone();
        let btc_block_hash = status_update.btc_block_hash;
        let old_status_code = status_update.old_status_code.clone();

        if let Some(ref error_code) = add_error {
            if error_code == &ErrorCode::NoDataBTCRelay || error_code == &ErrorCode::InvalidBTCRelay
            {
                ext::btc_relay::flag_block_error::<T>(
                    btc_block_hash.ok_or(Error::<T>::ExpectedBlockHash)?,
                    error_code.clone(),
                )?;
            }
            ext::security::insert_error::<T>(error_code.clone());
        }

        if let Some(ref error_code) = remove_error {
            if error_code == &ErrorCode::NoDataBTCRelay || error_code == &ErrorCode::InvalidBTCRelay
            {
                ext::btc_relay::clear_block_error::<T>(
                    btc_block_hash.ok_or(Error::<T>::ExpectedBlockHash)?,
                    error_code.clone(),
                )?;
            }
            if old_status_code == StatusCode::Error {
                ext::security::remove_error::<T>(error_code.clone());
            }
        }

        ext::collateral::release_collateral::<T>(&status_update.proposer, status_update.deposit)?;
        status_update.proposal_status = ProposalStatus::Accepted;
        Self::slash_staked_relayers(&status_update.add_error, &status_update.tally.nay)?;
        Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
            status_update_id,
            status_code.clone(),
            status_update.add_error.clone(),
            status_update.remove_error.clone(),
            status_update.btc_block_hash.clone(),
        ));
        Ok(())
    }

    /// Rejects a suggested `StatusUpdate`.
    ///
    /// # Arguments
    ///
    /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `ActiveStatusUpdates`.
    /// * `status_update`: `StatusUpdate` voted upon.
    fn reject_status_update(
        status_update_id: StatusUpdateId,
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> DispatchResult {
        ensure!(
            !status_update.tally.is_approved(),
            Error::<T>::InsufficientNoVotes
        );

        status_update.proposal_status = ProposalStatus::Rejected;
        Self::slash_staked_relayers(&status_update.add_error, &status_update.tally.aye)?;
        Self::deposit_event(<Event<T>>::RejectStatusUpdate(
            status_update_id,
            status_update.new_status_code.clone(),
            status_update.add_error.clone(),
            status_update.remove_error.clone(),
        ));
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
        payments: &Vec<(i64, BtcAddress)>,
        op_returns: &Vec<(i64, Vec<u8>)>,
        wallet: &Wallet,
    ) -> bool {
        if op_returns.len() > 0 {
            // migration should only contain payments
            return false;
        }

        for (_value, address) in payments {
            if !wallet.has_btc_address(&address) {
                return false;
            }
        }

        return true;
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
        request_value: PolkaBTC<T>,
        request_address: BtcAddress,
        payments: &Vec<(i64, BtcAddress)>,
        wallet: &Wallet,
    ) -> bool {
        let request_value = match TryInto::<u64>::try_into(request_value)
            .map_err(|_e| Error::<T>::TryIntoIntError)
        {
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
        return true;
    }

    /// Check if a vault transaction is invalid. Returns `Ok` if invalid or `Err` otherwise.
    /// This method should be callable over RPC for a staked-relayer client to check validity.
    ///
    /// # Arguments
    ///
    /// `vault_id`: the vault.
    /// `raw_tx`: the BTC transaction by the vault.
    pub fn is_transaction_invalid(vault_id: &T::AccountId, raw_tx: Vec<u8>) -> DispatchResult {
        let vault = ext::vault_registry::get_active_vault_from_id::<T>(vault_id)?;

        // TODO: ensure this cannot fail on invalid
        let tx =
            parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;

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
        if let Ok((payments, op_returns)) = ext::btc_relay::extract_outputs::<T>(tx.clone()) {
            // check if the transaction is a "migration"
            ensure!(
                !Self::is_valid_merge_transaction(&payments, &op_returns, &vault.wallet),
                Error::<T>::ValidMergeTransaction
            );

            if op_returns.len() != 1 {
                // we only expect one op_return output
                return Ok(());
            } else if op_returns[0].0 > 0 {
                // op_return output should not burn value
                return Ok(());
            } else if op_returns[0].1.len() < 32 {
                // request id is expected to be 32 bytes (256 bits)
                return Ok(());
            }

            // op_return can be up to 83 bytes so slice first 32
            let request_id = H256::from_slice(&op_returns[0].1[..32]);

            // redeem requests
            match ext::redeem::get_open_or_completed_redeem_request_from_id::<T>(&request_id) {
                Ok(req) => {
                    ensure!(
                        !Self::is_valid_request_transaction(
                            req.amount_btc,
                            req.btc_address,
                            &payments,
                            &vault.wallet,
                        ),
                        Error::<T>::ValidRedeemTransaction
                    );
                }
                Err(_) => (),
            };

            // replace requests
            match ext::replace::get_open_or_completed_replace_request::<T>(&request_id) {
                Ok(req) => {
                    if let Some(btc_address) = req.btc_address {
                        // only check replace if we have a valid btc_address
                        ensure!(
                            !Self::is_valid_request_transaction(
                                req.amount,
                                btc_address,
                                &payments,
                                &vault.wallet,
                            ),
                            Error::<T>::ValidReplaceTransaction
                        );
                    }
                }
                Err(_) => (),
            };

            // refund requests
            match ext::refund::get_open_or_completed_refund_request_from_id::<T>(&request_id) {
                Ok(req) => {
                    ensure!(
                        !Self::is_valid_request_transaction(
                            req.amount_polka_btc,
                            req.btc_address,
                            &payments,
                            &vault.wallet,
                        ),
                        Error::<T>::ValidRefundTransaction
                    );
                }
                Err(_) => (),
            };
        }

        Ok(())
    }

    /// Increments the current `StatusCounter` and returns the new value.
    pub fn get_status_counter() -> StatusUpdateId {
        <StatusCounter>::mutate(|c| {
            *c += 1;
            *c
        })
    }

    /// Gets the maturity period
    pub fn get_maturity_period() -> T::BlockNumber {
        <MaturityPeriod<T>>::get()
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        BlockNumber = <T as frame_system::Config>::BlockNumber,
        DOT = DOT<T>,
    {
        RegisterStakedRelayer(AccountId, BlockNumber, DOT),
        DeregisterStakedRelayer(AccountId),
        StatusUpdateSuggested(
            StatusUpdateId,
            AccountId,
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
            Option<H256Le>,
        ),
        VoteOnStatusUpdate(StatusUpdateId, AccountId, bool),
        ExecuteStatusUpdate(
            StatusUpdateId,
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
            Option<H256Le>,
        ),
        RejectStatusUpdate(
            StatusUpdateId,
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
        ),
        ForceStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        SlashStakedRelayer(AccountId),
        OracleOffline(),
        VaultTheft(AccountId, H256Le),
        VaultUnderLiquidationThreshold(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Staked relayer is already registered
        AlreadyRegistered,
        /// Insufficient collateral staked
        InsufficientStake,
        /// Insufficient deposit
        InsufficientDeposit,
        /// Status update message is too big
        MessageTooBig,
        /// Participant is not registered
        NotRegistered,
        /// Staked relayer has not bonded
        NotMatured,
        /// Caller is not governance module
        GovernanceOnly,
        /// Staked relayer is active
        StatusUpdateFound,
        /// Status update does not exist
        StatusUpdateNotFound,
        /// Status update has insufficient yes votes
        InsufficientYesVotes,
        /// Status update has insufficient no votes
        InsufficientNoVotes,
        /// Staked relayer has already cast vote
        VoteAlreadyCast,
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
        /// Oracle already reported
        OracleAlreadyReported,
        /// Oracle is online
        OracleOnline,
        /// Block not included by the relay
        BlockNotFound,
        /// Block already reported
        BlockAlreadyReported,
        /// Cannot report vault theft without block hash
        ExpectedBlockHash,
        /// Status update should not contain block hash
        UnexpectedBlockHash,
        /// Vault has sufficient collateral
        CollateralOk,
        /// Failed to parse transaction
        InvalidTransaction,
        /// Unable to convert value
        TryIntoIntError,
    }
}
