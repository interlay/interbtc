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
    ActiveStakedRelayer, InactiveStakedRelayer, PolkaBTC, ProposalStatus, StakedRelayerStatus,
    StatusUpdate, Tally, DOT,
};
use bitcoin::parser::parse_transaction;
use bitcoin::types::*;
use bitcoin::Payload as BtcPayload;
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
    fn activate_staked_relayer() -> Weight;
    fn deactivate_staked_relayer() -> Weight;
    fn suggest_status_update() -> Weight;
    fn vote_on_status_update() -> Weight;
    fn force_status_update() -> Weight;
    fn slash_staked_relayer() -> Weight;
    fn report_vault_theft() -> Weight;
    fn report_vault_under_liquidation_threshold() -> Weight;
    fn remove_active_status_update() -> Weight;
    fn remove_inactive_status_update() -> Weight;
}

/// ## Configuration
/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait
    + security::Trait
    + collateral::Trait
    + vault_registry::Trait
    + btc_relay::Trait
    + redeem::Trait
    + replace::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;

    /// Number of blocks to wait until eligible to vote.
    type MaturityPeriod: Get<Self::BlockNumber>;

    /// The minimum amount of deposit required to propose an update.
    type MinimumDeposit: Get<DOT<Self>>;

    /// The minimum amount of stake required to participate.
    type MinimumStake: Get<DOT<Self>>;

    /// The minimum number of active participants.
    type MinimumParticipants: Get<u64>;

    /// Denotes the percentage of votes necessary to enact an update.
    type VoteThreshold: Get<u64>;

    /// How often (in blocks) to check for new votes.
    type VotingPeriod: Get<Self::BlockNumber>;

    /// Maximum message size in bytes
    type MaximumMessageSize: Get<u32>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Staking {
        /// Mapping from accounts of active staked relayers to the StakedRelayer struct.
        ActiveStakedRelayers get(fn active_staked_relayer): map hasher(blake2_128_concat) T::AccountId => ActiveStakedRelayer<DOT<T>>;

        /// Integer total count of active staked relayers.
        ActiveStakedRelayersCount get(fn active_staked_relayer_count): u64;

        /// Mapping from accounts of inactive staked relayers to the StakedRelayer struct.
        InactiveStakedRelayers: map hasher(blake2_128_concat) T::AccountId => InactiveStakedRelayer<T::BlockNumber, DOT<T>>;

        /// Map of active StatusUpdates, identified by an integer key.
        ActiveStatusUpdates get(fn active_status_update): map hasher(blake2_128_concat) u64 => StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>;

        /// Map of expired, executed or rejected StatusUpdates, identified by an integer key.
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
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        const MaturityPeriod: T::BlockNumber = T::MaturityPeriod::get();

        const MinimumDeposit: DOT<T> = T::MinimumDeposit::get();

        const MinimumStake: DOT<T> = T::MinimumStake::get();

        const MinimumParticipants: u64 = T::MinimumParticipants::get();

        const VoteThreshold: u64 = T::VoteThreshold::get();

        const VotingPeriod: T::BlockNumber = T::VotingPeriod::get();

        const MaximumMessageSize: u32 = T::MaximumMessageSize::get();

        fn deposit_event() = default;

        /// Registers a new Staked Relayer, locking the provided collateral, which must exceed `STAKED_RELAYER_STAKE`.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be registered
        /// * `stake`: to-be-locked collateral/stake in DOT
        #[weight = <T as Trait>::WeightInfo::register_staked_relayer()]
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
            let period = height + T::MaturityPeriod::get();
            Self::add_inactive_staked_relayer(&signer, stake, StakedRelayerStatus::Bonding(period));
            Self::deposit_event(<Event<T>>::RegisterStakedRelayer(signer, period, stake));
            Ok(())
        }

        /// Deregisters a Staked Relayer, releasing the associated stake.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be deregistered
        #[weight = <T as Trait>::WeightInfo::deregister_staked_relayer()]
        fn deregister_staked_relayer(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let staked_relayer = Self::get_active_staked_relayer(&signer)?;
            Self::ensure_staked_relayer_is_not_active(&signer)?;
            ext::collateral::release_collateral::<T>(&signer, staked_relayer.stake)?;
            Self::remove_active_staked_relayer(&signer);
            Self::deposit_event(<Event<T>>::DeregisterStakedRelayer(signer));
            Ok(())
        }

        /// Activates a Staked Relayer if previously idle.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be activated
        #[weight = <T as Trait>::WeightInfo::activate_staked_relayer()]
        fn activate_staked_relayer(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let staked_relayer = Self::get_inactive_staked_relayer(&signer)?;

            match staked_relayer.status {
                StakedRelayerStatus::Bonding(period) => {
                    // on_initialize should catch all matured relayers
                    // but this is helpful for tests
                    let height = <frame_system::Module<T>>::block_number();
                    Self::try_bond_staked_relayer(&signer, staked_relayer.stake, height, period)?;
                },
                _ => Self::bond_staked_relayer(&signer, staked_relayer.stake),
            }

            Self::deposit_event(<Event<T>>::ActivateStakedRelayer(signer, staked_relayer.stake));
            Ok(())
        }

        /// Deactivates a Staked Relayer.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be deactivated
        #[weight = <T as Trait>::WeightInfo::deactivate_staked_relayer()]
        fn deactivate_staked_relayer(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let staked_relayer = Self::get_active_staked_relayer(&signer)?;
            Self::unbond_staked_relayer(&signer, staked_relayer.stake);
            Self::deposit_event(<Event<T>>::DeactivateStakedRelayer(signer));
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
        #[weight = <T as Trait>::WeightInfo::suggest_status_update()]
        fn suggest_status_update(origin, deposit: DOT<T>, status_code: StatusCode, add_error: Option<ErrorCode>, remove_error: Option<ErrorCode>, block_hash: Option<H256Le>, message: Vec<u8>) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            if status_code == StatusCode::Shutdown {
                Self::only_governance(&signer)?;
            }

            ensure!(
                message.len() as u32 <= T::MaximumMessageSize::get(),
                Error::<T>::MessageTooBig,
            );

            ensure!(
                <ActiveStakedRelayers<T>>::contains_key(&signer),
                Error::<T>::StakedRelayersOnly,
            );

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
            tally.aye.insert(signer.clone());

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
        #[weight = <T as Trait>::WeightInfo::vote_on_status_update()]
        fn vote_on_status_update(origin, status_update_id: u64, approve: bool) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            ensure!(
                <ActiveStakedRelayers<T>>::contains_key(&signer),
                Error::<T>::StakedRelayersOnly,
            );

            let mut update = Self::get_status_update(&status_update_id)?;
            ensure!(
                update.tally.vote(signer.clone(), approve),
                Error::<T>::VoteAlreadyCast,
            );
            <ActiveStatusUpdates<T>>::insert(&status_update_id, &update);

            Self::deposit_event(<Event<T>>::VoteOnStatusUpdate(status_update_id.clone(), signer, approve));

            Ok(())
        }

        /// This function can only be called by the Governance Mechanism.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Governance Mechanism.
        /// * `status_code`: Suggested BTC Parachain status (`StatusCode` enum).
        /// * `errors`: If the suggested status is `Error`, this set of `ErrorCode` entries provides details on the occurred errors.
        #[weight = <T as Trait>::WeightInfo::force_status_update()]
        fn force_status_update(origin, status_code: StatusCode, add_error: Option<ErrorCode>, remove_error: Option<ErrorCode>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            Self::only_governance(&signer)?;
            ext::security::set_parachain_status::<T>(status_code.clone());

            let to_add = add_error.clone();
            let to_remove = remove_error.clone();
            ext::security::mutate_errors::<T, _>(move |errors| {
                if let Some(err) = to_add {
                    errors.insert(err);
                }
                if let Some(err) = to_remove {
                    errors.remove(&err);
                }
                Ok(())
            })?;

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
        #[weight = <T as Trait>::WeightInfo::slash_staked_relayer()]
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
        /// * `tx_block_height`: Height rogue tx was included.
        /// * `merkle_proof`: The proof of tx inclusion.
        /// * `raw_tx`: The raw Bitcoin transaction.
        #[weight = <T as Trait>::WeightInfo::report_vault_theft()]
        fn report_vault_theft(origin, vault_id: T::AccountId, tx_id: H256Le, _tx_block_height: u32, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                Self::check_relayer_registered(&signer),
                Error::<T>::StakedRelayersOnly,
            );

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
            ext::security::set_parachain_status::<T>(StatusCode::Error);
            ext::security::mutate_errors::<T, _>(|errors| {
                errors.insert(ErrorCode::Liquidation);
                Ok(())
            })?;

            <TheftReports<T>>::mutate(&tx_id, |reports| {
                reports.insert(vault_id);
            });

            Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
                StatusCode::Error,
                Some(ErrorCode::Liquidation),
                None,
                Some(tx_id),
            ));

            Ok(())
        }

        /// A Staked Relayer reports that a Vault is undercollateralized (i.e. below the LiquidationCollateralThreshold as defined in Vault Registry).
        /// If the collateral falls below this rate, we flag the Vault for liquidation and update the ParachainStatus to ERROR - adding LIQUIDATION to Errors.
        #[weight = <T as Trait>::WeightInfo::report_vault_under_liquidation_threshold()]
        fn report_vault_under_liquidation_threshold(origin, vault_id: T::AccountId)  -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                Self::check_relayer_registered(&signer),
                Error::<T>::StakedRelayersOnly,
            );

            // FIXME: move the check for collateral into the vault registry
            // get the vault from the registry
            let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;
            // get the currently locked collateral for the vault
            let collateral_in_dot = ext::collateral::get_collateral_from_account::<T>(&vault_id);
            // get the current threshold for the collateral
            // NOTE: The liquidation threshold expresses the percentage of minimum collateral
            // level required for the vault. If the vault is under this percentage,
            // the vault is flagged for liquidation.
            let liquidation_collateral_threshold = ext::vault_registry::get_liquidation_collateral_threshold::<T>();

            // calculate how much PolkaBTC the vault should maximally have considering
            // the liquidation threshold.
            // NOTE: if the division fails, return 0 as maximum amount
            let raw_collateral_in_dot = Self::dot_to_u128(collateral_in_dot)?;
            let max_polka_btc_in_dot = match raw_collateral_in_dot
                .checked_div(liquidation_collateral_threshold) {
                    Some(v) => v,
                    None => 0,
            };

            // get the currently issued tokens of the vault
            let amount_btc_in_dot = ext::oracle::btc_to_dots::<T>(vault.issued_tokens)?;
            let raw_amount_btc_in_dot = Self::dot_to_u128(amount_btc_in_dot)?;

            // Ensure that the current amount of PolkaBTC (in DOT) is greater than
            // the allowed maximum of issued tokens to flag the vault for liquidation
            ensure!(
                max_polka_btc_in_dot < raw_amount_btc_in_dot,
                Error::<T>::CollateralOk,
            );

            ext::vault_registry::liquidate_vault::<T>(&vault_id)?;
            ext::security::set_parachain_status::<T>(StatusCode::Error);
            ext::security::mutate_errors::<T, _>(|errors| {
                errors.insert(ErrorCode::Liquidation);
                Ok(())
            })?;

            Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
                StatusCode::Error,
                Some(ErrorCode::Liquidation),
                None,
                None,
            ));

            Ok(())
        }

        /// A Staked Relayer reports that the Exchange Rate Oracle is offline. This function checks if the last exchange
        /// rate data in the Exchange Rate Oracle is indeed older than the indicated threshold.
        #[weight = 1000]
        fn report_oracle_offline(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                Self::check_relayer_registered(&signer),
                Error::<T>::StakedRelayersOnly,
            );

            ensure!(
                !ext::security::get_errors::<T>().contains(&ErrorCode::OracleOffline),
                Error::<T>::OracleAlreadyReported,
            );

            ensure!(
                ext::oracle::is_max_delay_passed::<T>(),
                Error::<T>::OracleOnline,
            );

            ext::security::set_parachain_status::<T>(StatusCode::Error);
            ext::security::mutate_errors::<T, _>(|errors| {
                errors.insert(ErrorCode::OracleOffline);
                Ok(())
            })?;

            Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
                StatusCode::Error,
                Some(ErrorCode::OracleOffline),
                None,
                None,
            ));

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
        #[weight = <T as Trait>::WeightInfo::remove_active_status_update()]
        fn remove_active_status_update(origin, status_update_id: u64) {
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
        #[weight = <T as Trait>::WeightInfo::remove_inactive_status_update()]
        fn remove_inactive_status_update(origin, status_update_id: u64) {
            ensure_root(origin)?;
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
impl<T: Trait> Module<T> {
    fn begin_block(height: T::BlockNumber) -> DispatchResult {
        for (id, acc) in <InactiveStakedRelayers<T>>::iter() {
            if let StakedRelayerStatus::Bonding(period) = acc.status {
                let _ = Self::try_bond_staked_relayer(&id, acc.stake, height, period);
            }
        }
        Ok(())
    }

    fn end_block(height: T::BlockNumber) {
        <ActiveStatusUpdates<T>>::translate(
            |id, mut status_update: StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>| {
                match Self::evaluate_status_update_at_height(id, &mut status_update, height) {
                    // remove proposal
                    Ok(true) => None,
                    // proposal is not accepted, rejected or expired
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

    /// Evaluates whether the `StatusUpdate` has been accepted, rejected or expired.
    /// Returns true if the `StatusUpdate` should be garbage collected.
    ///
    /// # Arguments
    ///
    /// * `id` - id of the `StatusUpdate`
    /// * `status_update` - `StatusUpdate` to evaluate
    /// * `height` - current height of the chain.
    fn evaluate_status_update_at_height(
        id: u64,
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
        height: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        if status_update
            .tally
            .is_approved(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get())
        {
            Self::execute_status_update(&mut status_update)?;
            Self::insert_inactive_status_update(id, status_update);
            Ok(true)
        } else if status_update
            .tally
            .is_rejected(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get())
        {
            Self::reject_status_update(&mut status_update)?;
            Self::insert_inactive_status_update(id, status_update);
            Ok(true)
        } else if height >= status_update.end {
            // return the proposer's collateral
            ext::collateral::release_collateral::<T>(
                &status_update.proposer,
                status_update.deposit,
            )?;
            status_update.proposal_status = ProposalStatus::Expired;
            Self::insert_inactive_status_update(id, status_update);
            Self::deposit_event(<Event<T>>::ExpireStatusUpdate(id));
            Ok(true)
        } else {
            Ok(false)
        }
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
        Self::bond_staked_relayer(id, stake);
        Ok(())
    }

    /// Activate the staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    /// * `stake` - amount of stake to deposit.
    fn bond_staked_relayer(id: &T::AccountId, stake: DOT<T>) {
        Self::add_active_staked_relayer(id, stake);
        Self::remove_inactive_staked_relayer(id);
    }

    /// Deactivate the relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the caller.
    /// * `stake` - amount of stake to deposit.
    fn unbond_staked_relayer(id: &T::AccountId, stake: DOT<T>) {
        Self::remove_active_staked_relayer(id);
        Self::add_inactive_staked_relayer(id, stake, StakedRelayerStatus::Idle);
    }

    fn ensure_staked_relayer_is_not_active(id: &T::AccountId) -> DispatchResult {
        for (_, update) in <ActiveStatusUpdates<T>>::iter() {
            ensure!(!update.tally.contains(id), Error::<T>::StatusUpdateFound);
        }
        Ok(())
    }

    fn dot_to_u128(amount: DOT<T>) -> Result<u128, Error<T>> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::ConversionError)
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

    /// Checks if a staked relayer is registered.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn check_relayer_registered(id: &T::AccountId) -> bool {
        <ActiveStakedRelayers<T>>::contains_key(id)
    }

    /// Gets the active staked relayer or throws an error.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn get_active_staked_relayer(
        id: &T::AccountId,
    ) -> Result<ActiveStakedRelayer<DOT<T>>, DispatchError> {
        ensure!(
            Self::check_relayer_registered(id),
            Error::<T>::NotRegistered,
        );
        Ok(<ActiveStakedRelayers<T>>::get(id))
    }

    /// Gets the inactive staked relayer or throws an error.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    pub(crate) fn get_inactive_staked_relayer(
        id: &T::AccountId,
    ) -> Result<InactiveStakedRelayer<T::BlockNumber, DOT<T>>, DispatchError> {
        ensure!(
            <InactiveStakedRelayers<T>>::contains_key(id),
            Error::<T>::NotRegistered,
        );
        Ok(<InactiveStakedRelayers<T>>::get(id))
    }

    /// Creates an active staked relayer, incrementing the total count.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    /// * `stake` - token deposited
    pub(crate) fn add_active_staked_relayer(id: &T::AccountId, stake: DOT<T>) {
        <ActiveStakedRelayers<T>>::insert(id, ActiveStakedRelayer { stake: stake });
        <ActiveStakedRelayersCount>::mutate(|c| {
            *c += 1;
            *c
        });
    }

    /// Creates an inactive staked relayer.
    ///
    /// # Arguments
    ///
    /// * `id` - account id of the relayer
    /// * `stake` - token deposited
    /// * `status` - reason for becoming inactive
    pub(crate) fn add_inactive_staked_relayer(
        id: &T::AccountId,
        stake: DOT<T>,
        status: StakedRelayerStatus<T::BlockNumber>,
    ) {
        <InactiveStakedRelayers<T>>::insert(
            id,
            InactiveStakedRelayer {
                stake: stake,
                status: status,
            },
        );
    }

    /// Removes an active staked relayer, decrementing the total count.
    ///
    /// # Arguments
    ///
    /// * `id` - AccountId of the relayer.
    fn remove_active_staked_relayer(id: &T::AccountId) {
        <ActiveStakedRelayers<T>>::remove(id);
        <ActiveStakedRelayersCount>::mutate(|c| {
            *c -= 1;
            *c
        });
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
    ) -> u64 {
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
        status_id: u64,
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
        status_update_id: &u64,
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
    /// * `votes` - account set
    fn slash_staked_relayers(
        error: &Option<ErrorCode>,
        votes: &BTreeSet<T::AccountId>,
    ) -> DispatchResult {
        if let Some(ErrorCode::NoDataBTCRelay) = error {
            // we don't slash participants for this
            return Ok(());
        }

        for acc in votes.iter() {
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

    /// Executes a `StatusUpdate` that has received sufficient “Yes” votes.
    ///
    /// # Arguments
    ///
    /// * `status_update`: `StatusUpdate` voted upon.
    fn execute_status_update(
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> DispatchResult {
        ensure!(
            <ActiveStakedRelayersCount>::get() > T::MinimumParticipants::get(),
            Error::<T>::InsufficientParticipants,
        );

        ensure!(
            status_update
                .tally
                .is_approved(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get()),
            Error::<T>::InsufficientYesVotes
        );

        let status_code = status_update.new_status_code.clone();
        ext::security::set_parachain_status::<T>(status_code.clone());

        let add_error = status_update.add_error.clone();
        let remove_error = status_update.remove_error.clone();
        let btc_block_hash = status_update.btc_block_hash;
        let old_status_code = status_update.old_status_code.clone();
        ext::security::mutate_errors::<T, _>(move |errors| {
            if let Some(err) = add_error {
                if err == ErrorCode::NoDataBTCRelay || err == ErrorCode::InvalidBTCRelay {
                    ext::btc_relay::flag_block_error::<T>(
                        btc_block_hash.ok_or(Error::<T>::ExpectedBlockHash)?,
                        err.clone(),
                    )?;
                }
                errors.insert(err);
            }

            if let Some(err) = remove_error {
                if err == ErrorCode::NoDataBTCRelay || err == ErrorCode::InvalidBTCRelay {
                    ext::btc_relay::clear_block_error::<T>(
                        btc_block_hash.ok_or(Error::<T>::ExpectedBlockHash)?,
                        err.clone(),
                    )?;
                }
                if old_status_code == StatusCode::Error {
                    errors.remove(&err);
                }
            }
            Ok(())
        })?;

        ext::collateral::release_collateral::<T>(&status_update.proposer, status_update.deposit)?;
        status_update.proposal_status = ProposalStatus::Accepted;
        Self::slash_staked_relayers(&status_update.add_error, &status_update.tally.nay)?;
        Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
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
    fn reject_status_update(
        mut status_update: &mut StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> DispatchResult {
        ensure!(
            <ActiveStakedRelayersCount>::get() > T::MinimumParticipants::get(),
            Error::<T>::InsufficientParticipants,
        );

        ensure!(
            status_update
                .tally
                .is_rejected(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get()),
            Error::<T>::InsufficientNoVotes
        );

        status_update.proposal_status = ProposalStatus::Rejected;
        Self::slash_staked_relayers(&status_update.add_error, &status_update.tally.aye)?;
        Self::deposit_event(<Event<T>>::RejectStatusUpdate(
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
        payments: &Vec<(i64, BtcPayload)>,
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
        request_address: BtcPayload,
        payments: &Vec<(i64, BtcPayload)>,
        wallet: &Wallet,
    ) -> bool {
        let request_value = match TryInto::<u64>::try_into(request_value)
            .map_err(|_e| Error::<T>::ConversionError)
        {
            Ok(value) => value as i64,
            Err(_) => return false,
        };

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
    ///
    /// # Arguments
    ///
    /// `vault_id`: the vault.
    /// `raw_tx`: the BTC transaction by the vault.
    pub fn is_transaction_invalid(vault_id: &T::AccountId, raw_tx: Vec<u8>) -> DispatchResult {
        let vault = ext::vault_registry::get_vault_from_id::<T>(vault_id)?;

        // TODO: ensure this cannot fail on invalid
        let tx =
            parse_transaction(raw_tx.as_slice()).map_err(|_| Error::<T>::InvalidTransaction)?;

        // collect all addresses that feature in the inputs of the transaction
        let input_addresses: Vec<Result<BtcPayload, _>> = tx
            .clone()
            .inputs
            .into_iter()
            .map(|input| input.extract_address())
            .collect();

        // check if vault's btc address features in an input of the transaction
        ensure!(
            input_addresses.into_iter().any(|address_result| {
                match address_result {
                    Ok(address) => vault.wallet.has_btc_address(&address),
                    _ => false,
                }
            }),
            Error::<T>::VaultNoInputToTransaction
        );

        // Vaults are required to move funds for redeem and replace operations.
        // Each transaction MUST feature at least two or three outputs as follows:
        // * recipient: the recipient of the redeem/replace
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
            }

            let request_id = H256::from_slice(&op_returns[0].1);

            // redeem requests
            match ext::redeem::get_redeem_request_from_id::<T>(&request_id) {
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
            match ext::replace::get_replace_request::<T>(&request_id) {
                Ok(req) => {
                    ensure!(
                        !Self::is_valid_request_transaction(
                            req.amount,
                            req.btc_address,
                            &payments,
                            &vault.wallet,
                        ),
                        Error::<T>::ValidReplaceTransaction
                    );
                }
                Err(_) => (),
            };
        }

        Ok(())
    }

    /// Increments the current `StatusCounter` and returns the new value.
    pub fn get_status_counter() -> u64 {
        <StatusCounter>::mutate(|c| {
            *c += 1;
            *c
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
        DOT = DOT<T>,
    {
        RegisterStakedRelayer(AccountId, BlockNumber, DOT),
        DeregisterStakedRelayer(AccountId),
        ActivateStakedRelayer(AccountId, DOT),
        DeactivateStakedRelayer(AccountId),
        StatusUpdateSuggested(
            u64,
            AccountId,
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
            Option<H256Le>,
        ),
        VoteOnStatusUpdate(u64, AccountId, bool),
        ExecuteStatusUpdate(
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
            Option<H256Le>,
        ),
        RejectStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        ExpireStatusUpdate(u64),
        ForceStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        SlashStakedRelayer(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Staked relayer is already registered
        AlreadyRegistered,
        /// Insufficient collateral staked
        InsufficientStake,
        /// Insufficient deposit
        InsufficientDeposit,
        /// Insufficient participants
        InsufficientParticipants,
        /// Status update message is too big
        MessageTooBig,
        /// Staked relayer is not registered
        NotRegistered,
        /// Staked relayer has not bonded
        NotMatured,
        /// Caller is not governance module
        GovernanceOnly,
        /// Caller is not staked relayer
        StakedRelayersOnly,
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
        /// Error converting value
        ConversionError,
    }
}
