#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod types;

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
    ActiveStakedRelayer, InactiveStakedRelayer, ProposalStatus, StakedRelayerStatus, StatusUpdate,
    Tally, DOT,
};
use bitcoin::parser::parse_transaction;
use bitcoin::types::*;
/// # Security module implementation
/// This is the implementation of the BTC Parachain Security module following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/security
///
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    weights::Weight,
    IterableStorageMap,
};
use primitive_types::H256;
use security::types::{ErrorCode, StatusCode};
use sp_core::{H160, U256};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use system::ensure_signed;

/// ## Configuration
/// The pallet's configuration trait.
pub trait Trait:
    system::Trait
    + security::Trait
    + collateral::Trait
    + vault_registry::Trait
    + btc_relay::Trait
    + redeem::Trait
    + replace::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

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

        /// Map of StatusUpdates, identified by an integer key.
        StatusUpdates get(fn status_update): map hasher(blake2_128_concat) U256 => StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>;

        /// Integer increment-only counter used to track status updates.
        StatusCounter get(fn status_counter): U256;

        /// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
        /// identifiers of Vaults accused of theft.
        TheftReports get(fn theft_report): map hasher(blake2_128_concat) H256Le => BTreeSet<T::AccountId>;

        /// AccountId of the governance mechanism, as specified in the genesis.
        GovernanceId get(fn gov_id) config(): T::AccountId;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        const MaturityPeriod: T::BlockNumber = T::MaturityPeriod::get();

        const MinimumDeposit: DOT<T> = T::MinimumDeposit::get();

        const MinimumStake: DOT<T> = T::MinimumStake::get();

        const MinimumParticipants: u64 = T::MinimumParticipants::get();

        const VoteThreshold: u64 = T::VoteThreshold::get();

        fn deposit_event() = default;

        /// Registers a new Staked Relayer, locking the provided collateral, which must exceed `STAKED_RELAYER_STAKE`.
        ///
        /// # Arguments
        ///
        /// * `origin`: The account of the Staked Relayer to be registered
        /// * `stake`: to-be-locked collateral/stake in DOT
        #[weight = 1000]
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
            let height = <system::Module<T>>::block_number();
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
        #[weight = 1000]
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
        #[weight = 1000]
        fn activate_staked_relayer(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let staked_relayer = Self::get_inactive_staked_relayer(&signer)?;

            match staked_relayer.status {
                StakedRelayerStatus::Bonding(period) => {
                    // on_initialize should catch all matured relayers
                    // but this is helpful for tests
                    let height = <system::Module<T>>::block_number();
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
        #[weight = 1000]
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
        /// * `add_error`: If the suggested status is Error, this set of ErrorCode indicates which error is to be added to the Errors mapping.
        /// * `remove_error`: ErrorCode to be removed from the Errors list.
        /// * `block_hash`: [Optional] When reporting an error related to BTC-Relay, this field indicates the affected Bitcoin block (header).
        #[weight = 1000]
        fn suggest_status_update(origin, deposit: DOT<T>, status_code: StatusCode, add_error: Option<ErrorCode>, remove_error: Option<ErrorCode>, block_hash: Option<H256Le>) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            if status_code == StatusCode::Shutdown {
                Self::only_governance(&signer)?;
            }

            ensure!(
                <ActiveStakedRelayers<T>>::contains_key(&signer),
                Error::<T>::StakedRelayersOnly,
            );

            ensure!(
                deposit >= T::MinimumDeposit::get(),
                Error::<T>::InsufficientDeposit,
            );
            ext::collateral::lock_collateral::<T>(&signer, deposit)?;

            // pre-approve
            let mut tally = Tally::default();
            tally.aye.insert(signer.clone());

            let height = <system::Module<T>>::block_number();
            let status_update_id = Self::insert_status_update(StatusUpdate{
                new_status_code: status_code.clone(),
                old_status_code: ext::security::get_parachain_status::<T>(),
                add_error: add_error.clone(),
                remove_error: remove_error.clone(),
                time: height,
                proposal_status: ProposalStatus::Pending,
                btc_block_hash: block_hash,
                proposer: signer.clone(),
                deposit: deposit,
                tally: tally,
            });

            Self::deposit_event(<Event<T>>::StatusUpdateSuggested(status_update_id, status_code, add_error, remove_error, signer));
            Ok(())
        }

        /// A Staked Relayer casts a vote on a suggested `StatusUpdate`. Checks the threshold
        /// of votes and executes / cancels a `StatusUpdate` depending on the threshold reached.
        ///
        /// # Arguments
        ///
        /// * `origin`: The AccountId of the Staked Relayer casting the vote.
        /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `StatusUpdates`.
        /// * `approve`: `True` or `False`, depending on whether the Staked Relayer agrees or disagrees with the suggested `StatusUpdate`.
        #[weight = 1000]
        fn vote_on_status_update(origin, status_update_id: U256, approve: bool) -> DispatchResult {
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
            <StatusUpdates<T>>::insert(&status_update_id, &update);

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
        #[weight = 1000]
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
        #[weight = 1000]
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
        /// * `origin`: The AccountId of the Governance Mechanism.
        /// * `staked_relayer_id`: The account of the Staked Relayer to be slashed.
        #[weight = 1000]
        fn report_vault_theft(origin, vault_id: T::AccountId, tx_id: H256Le, _tx_block_height: U256, _tx_index: u64, _merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                Self::check_relayer_registered(&signer),
                Error::<T>::StakedRelayersOnly,
            );

            let vault = ext::vault_registry::get_vault_from_id::<T>(&vault_id)?;

            // throw if already reported
            if <TheftReports<T>>::contains_key(&tx_id) {
                ensure!(
                    !<TheftReports<T>>::get(&tx_id).contains(&vault_id),
                    Error::<T>::VaultAlreadyReported,
                );
            }

            let tx = parse_transaction(raw_tx.as_slice())?;

            let input_addresses: Vec<_> = tx.inputs.into_iter().map(|input|
                                                                    input.extract_address()).collect();

            ensure!(input_addresses
                    .into_iter()
                    .any(|address_result| {match address_result
                                           {
                                               Ok(address) => H160::from_slice(&address) == vault.btc_address,
                                               _ => false
                                           }
                    }), Error::<T>::VaultNoInputToTransaction);

            // only check if correct format
            if tx.outputs.len() <= 2 {
                let out = &tx.outputs[0];
                // check if migration
                if let Ok(out_addr) = out.extract_address() {
                    ensure!(H160::from_slice(&out_addr) != vault.btc_address, Error::<T>::ValidMergeTransaction);
                    let out_val = out.value;
                    if tx.outputs.len() == 2 {
                        let out = &tx.outputs[1];
                        // check if redeem / replace
                        if let Ok(out_ret) = out.script.extract_op_return_data() {
                            let id = H256::from_slice(&out_ret);
                            let addr = H160::from_slice(&out_addr);
                            match ext::redeem::get_redeem_request_from_id::<T>(&id) {
                                Ok(req) => {
                                    let amount = TryInto::<u64>::try_into(req.amount_btc).map_err(|_e| Error::<T>::RuntimeError)? as i64;
                                    ensure!(
                                        out_val < amount && addr != req.btc_address,
                                        Error::<T>::ValidRedeemOrReplace
                                    )
                                },
                                Err(_) => (),
                            }
                            match ext::replace::get_replace_request::<T>(&id) {
                                Ok(req) => {
                                    let amount = TryInto::<u64>::try_into(req.amount).map_err(|_e| Error::<T>::RuntimeError)? as i64;
                                    ensure!(
                                        out_val < amount && addr != req.btc_address,
                                        Error::<T>::ValidRedeemOrReplace
                                    )
                                },
                                Err(_) => (),
                            }
                        }
                    }
                }
            }
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
            ));

            Ok(())
        }


        /// A Staked Relayer reports that a Vault is undercollateralized (i.e. below the LiquidationCollateralThreshold as defined in Vault Registry).
        /// If the collateral falls below this rate, we flag the Vault for liquidation and update the ParachainStatus to ERROR - adding LIQUIDATION to Errors.
        #[weight = 1000]
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
                ext::oracle::is_max_delay_passed::<T>()?,
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
            ));

            Ok(())
        }

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }

        fn on_finalize(_n: T::BlockNumber) {
            if let Err(e) = Self::end_block() {
                sp_runtime::print(e);
            }
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

    fn end_block() -> DispatchResult {
        for (id, update) in <StatusUpdates<T>>::iter() {
            if update
                .tally
                .is_approved(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get())
            {
                Self::execute_status_update(id)?;
            } else if update
                .tally
                .is_rejected(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get())
            {
                Self::reject_status_update(id)?;
            }
        }
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
        for (_, update) in <StatusUpdates<T>>::iter() {
            ensure!(!update.tally.contains(id), Error::<T>::StatusUpdateFound);
        }
        Ok(())
    }

    fn dot_to_u128(amount: DOT<T>) -> Result<u128, Error<T>> {
        TryInto::<u128>::try_into(amount).map_err(|_e| Error::RuntimeError)
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

    /// Insert a new status update and return the generated ID.
    ///
    /// # Arguments
    ///
    /// * `status_update` - `StatusUpdate` with the proposed changes.
    pub(crate) fn insert_status_update(
        status_update: StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>,
    ) -> U256 {
        let status_id = Self::get_status_counter();
        <StatusUpdates<T>>::insert(&status_id, status_update);
        status_id
    }

    /// Remove a resolved status update.
    ///
    /// # Arguments
    ///
    /// * `status_update_id` - id of the `StatusUpdate` to delete.
    pub(crate) fn remove_status_update(status_update_id: &U256) {
        <StatusUpdates<T>>::remove(status_update_id);
    }

    /// Get an existing `StatusUpdate` or throw.
    ///
    /// # Arguments
    ///
    /// * `status_update_id` - id of the `StatusUpdate` to fetch.
    pub(crate) fn get_status_update(
        status_update_id: &U256,
    ) -> Result<StatusUpdate<T::AccountId, T::BlockNumber, DOT<T>>, DispatchError> {
        ensure!(
            <StatusUpdates<T>>::contains_key(status_update_id),
            Error::<T>::StatusUpdateNotFound,
        );
        Ok(<StatusUpdates<T>>::get(status_update_id))
    }

    /// Update the proposal status of an existing `StatusUpdate`.
    ///
    /// # Arguments
    ///
    /// * `status_update_id` - id of the `StatusUpdate` to modify.
    /// * `proposal_status` - accepted / rejected.
    fn set_proposal_status(status_update_id: &U256, proposal_status: ProposalStatus) {
        <StatusUpdates<T>>::mutate(status_update_id, |update| {
            update.proposal_status = proposal_status
        })
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
        if let Some(err) = error {
            if err == &ErrorCode::NoDataBTCRelay {
                // we don't slash partipants for this
                return Ok(());
            }
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
    /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `StatusUpdates`.
    fn execute_status_update(status_update_id: U256) -> DispatchResult {
        let update = Self::get_status_update(&status_update_id)?;

        ensure!(
            <ActiveStakedRelayersCount>::get() > T::MinimumParticipants::get(),
            Error::<T>::InsufficientParticipants,
        );

        ensure!(
            update
                .tally
                .is_approved(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get()),
            Error::<T>::InsufficientYesVotes
        );

        let status_code = update.new_status_code.clone();
        ext::security::set_parachain_status::<T>(status_code.clone());

        let add_error = update.add_error.clone();
        let remove_error = update.remove_error.clone();
        let btc_block_hash = update.btc_block_hash;
        let old_status_code = update.old_status_code;
        ext::security::mutate_errors::<T, _>(move |errors| {
            if let Some(err) = add_error {
                if err == ErrorCode::NoDataBTCRelay || err == ErrorCode::InvalidBTCRelay {
                    ext::btc_relay::flag_block_error::<T>(
                        btc_block_hash.ok_or(Error::<T>::NoBlockHash)?,
                        err.clone(),
                    )?;
                }
                errors.insert(err);
            }

            if let Some(err) = remove_error {
                if err == ErrorCode::NoDataBTCRelay || err == ErrorCode::InvalidBTCRelay {
                    ext::btc_relay::clear_block_error::<T>(
                        btc_block_hash.ok_or(Error::<T>::NoBlockHash)?,
                        err.clone(),
                    )?;
                }
                if old_status_code == StatusCode::Error {
                    errors.remove(&err);
                }
            }
            Ok(())
        })?;

        ext::collateral::release_collateral::<T>(&update.proposer, update.deposit)?;

        Self::set_proposal_status(&status_update_id, ProposalStatus::Accepted);
        Self::remove_status_update(&status_update_id);

        Self::slash_staked_relayers(&update.add_error, &update.tally.nay)?;
        Self::deposit_event(<Event<T>>::ExecuteStatusUpdate(
            status_code,
            update.add_error,
            update.remove_error,
        ));
        Ok(())
    }

    /// Rejects a suggested `StatusUpdate`.
    ///
    /// # Arguments
    ///
    /// * `status_update_id`: Identifier of the `StatusUpdate` voted upon in `StatusUpdates`.
    fn reject_status_update(status_update_id: U256) -> DispatchResult {
        let update = Self::get_status_update(&status_update_id)?;

        ensure!(
            <ActiveStakedRelayersCount>::get() > T::MinimumParticipants::get(),
            Error::<T>::InsufficientParticipants,
        );

        ensure!(
            update
                .tally
                .is_rejected(<ActiveStakedRelayersCount>::get(), T::VoteThreshold::get()),
            Error::<T>::InsufficientNoVotes
        );

        Self::set_proposal_status(&status_update_id, ProposalStatus::Rejected);
        Self::remove_status_update(&status_update_id);

        Self::slash_staked_relayers(&update.add_error, &update.tally.aye)?;
        Self::deposit_event(<Event<T>>::RejectStatusUpdate(
            update.new_status_code,
            update.add_error,
            update.remove_error,
        ));
        Ok(())
    }

    /// Increments the current `StatusCounter` and returns the new value.
    pub fn get_status_counter() -> U256 {
        <StatusCounter>::mutate(|c| {
            *c += U256::one();
            *c
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        BlockNumber = <T as system::Trait>::BlockNumber,
        DOT = DOT<T>,
    {
        RegisterStakedRelayer(AccountId, BlockNumber, DOT),
        DeregisterStakedRelayer(AccountId),
        ActivateStakedRelayer(AccountId, DOT),
        DeactivateStakedRelayer(AccountId),
        StatusUpdateSuggested(
            U256,
            StatusCode,
            Option<ErrorCode>,
            Option<ErrorCode>,
            AccountId,
        ),
        VoteOnStatusUpdate(U256, AccountId, bool),
        ExecuteStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        RejectStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        ForceStatusUpdate(StatusCode, Option<ErrorCode>, Option<ErrorCode>),
        SlashStakedRelayer(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        AlreadyRegistered,
        InsufficientStake,
        InsufficientDeposit,
        InsufficientParticipants,
        NotRegistered,
        NotMatured,
        GovernanceOnly,
        StakedRelayersOnly,
        StatusUpdateFound,
        StatusUpdateNotFound,
        InsufficientYesVotes,
        InsufficientNoVotes,
        VoteAlreadyCast,
        VaultAlreadyReported,
        VaultAlreadyLiquidated,
        VaultNoInputToTransaction,
        ValidRedeemOrReplace,
        ValidMergeTransaction,
        OracleOnline,
        NoBlockHash,
        CollateralOk,
        RuntimeError,
    }
}
