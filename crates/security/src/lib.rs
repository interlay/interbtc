#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests;

/// # Security module implementation
/// This is the implementation of the BTC Parachain Security module following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/security
///
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure, Parameter};
use system::ensure_signed;
use frame_support::traits::Currency;
use codec::{Encode, Decode, Codec};
use node_primitives::{BlockNumber, AccountId};
use sp_core::{U256, H256};
use sp_runtime::traits::{Member, SimpleArithmetic, MaybeSerializeDeserialize};
use sp_std::fmt::Debug;
use sp_std::collections::btree_set::BTreeSet;

/// ## Configuration
/// The pallet's configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Dot currency
	// FIXME: Check if correct. DOT currently emulated, until DOT bridge becomes available
	type DotBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy +
	MaybeSerializeDeserialize + Debug + From<Self::BlockNumber>;
}

/// ## Constants
/// Voter threshold
pub const STAKED_RELAYER_VOTE_THRESHOLD: u8 = 0;

/// Minimum Staked Relayer stake
pub const STAKED_RELAYER_STAKE: u64 = 10;

/// ## Enums
/// Indicates the status of the BTC Parachain.
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum StatusCode {
	/// BTC Parachain is fully operational
	Running = 0,
	/// An error has occurred. See Errors for more details.
	Error = 1,
	/// BTC Parachain operation has been fully suspended
	Shutdown = 2
}
impl Default for StatusCode {
	fn default() -> Self { StatusCode::Running }
}

/// Enum specifying errors which lead to the Error status, tacked in Errors
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ErrorCode {
	/// No error. Used as default value
	None = 0,
	/// Missing transactional data for a block header submitted to BTC-Relay
    NoDataBTCRelay = 1,
	/// Invalid transaction was detected in a block header submitted to BTC-Relay
	InvalidBTCRelay = 2,
	/// The exchangeRateOracle experienced a liveness failure (no up-to-date exchange rate available)
	OracleOffline = 3,
	/// At least one Vault is being liquidated. Redeem requests paid out partially in collateral (DOT).
	Liquidation = 4
}
impl Default for ErrorCode {
	fn default() -> Self { ErrorCode::None }
}

// Indicates the state of a proposed StatusUpdate.
#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ProposalStatus {
	/// StatusUpdate is current under review and is being voted upon
	Pending = 0,
	/// StatusUpdate has been accepted
	Accepted = 1,
	/// StatusUpdate has been rejected
	Rejected = 2
}
impl Default for ProposalStatus {
	fn default() -> Self { ProposalStatus::Pending }
}

/// ## Structs
/// Struct storing information on a proposed parachain status update
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StatusUpdate{
	/// New status of the BTC Parachain.
	new_status_code: StatusCode,
	/// Previous status of the BTC Parachain.
	old_status_code: StatusCode,
	/// If new_status_code is Error, specifies which errors are to be added to Errors
	// FIXME: will need casting to ErrorCode enum
	add_errors: BTreeSet<u8>,
	/// Indicates which ErrorCode entries are to be removed from Errors (recovery).
	// FIXME: will need casting to ErrorCode enum
	remove_errors: BTreeSet<u8>,
	/// Parachain block number at which this status update was suggested.
	time: BlockNumber,
	/// Status of the proposed status update. See ProposalStatus.
	proposal_status: ProposalStatus,
	/// Message providing more details on the change of status (detailed error message or recovery reason).
	msg: String,
	/// Block hash of the Bitcoin block where the error was detected, if related to BTC-Relay.
	btc_block_hash: H256,
	/// Set of accounts which have voted FOR this status update. This can be either Staked Relayers or the Governance Mechanism.
	votes_yes: BTreeSet<AccountId>,
	/// Set of accounts which have voted AGAINST this status update. This can be either Staked Relayers or the Governance Mechanism.
	votes_no: BTreeSet<AccountId>
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StakedRelayer {
      stake: u64
}


// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as SecurityModule {

		/// Integer/Enum defining the current state of the BTC-Parachain
		ParachainStatus get(parachain_status): StatusCode;

		/// Set of ErrorCodes, indicating the reason for an "Error" ParachainStatus
		/// FIXME: type casting to enum necessary!
		Errors get(fn error): EnumSet<ErrorCode>;

		/// Integer increment-only counter used to track status updates.
		StatusCounter get(fn status_counter): U256;

		/// Integer increment-only counter, used to prevent collisions when generating identifiers
		/// for e.g. issue, redeem or replace requests (for OP_RETURN field in Bitcoin).
		Nonce get(fn nonce): U256;

		/// Mapping from accounts of staked relayers to the StakedRelayer struct
		StakedRelayers get(fn staked_relayer): map T::AccountId => StakedRelayer;

		/// Map of StatusUpdates, identified by an integer key
		StatusUpdates get(fn status_update): map U256 => StatusUpdate;

		/// Mapping of Bitcoin transaction identifiers (SHA256 hashes) to account
		/// identifiers of Vaults accused of theft
		TheftReports get(fn theft_report): map H256 => BTreeSet<AccountId>;
	}
}


// The pallet's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		// Initializing events
		fn deposit_event() = default;
        
        // Initialize errors
        type Error = Error<T>;

        fn register_staked_relayer(origin, stake: u64) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            
            // TODO: How does this check behave when a relayer de-registered?
            // Does Substrate delete the set and this check will pass?
            ensure!(!<StakedRelayers<T>>::exists(&sender), Error::<T>::AlreadyRegistered);
          
            // ensure!(stake >= Self::MINIMUM_STAKE, Error::<T>::InsufficientStake);

            // lock stake in the collateral module
            // track the stake in the StakedRelayers mapping
            let relayer = StakedRelayer {stake: stake};
            <StakedRelayers<T>>::insert(&sender, relayer);
            
            // Emit the event
            Self::deposit_event(RawEvent::RegisterStakedRelayer(sender, stake));
            Ok(()) 
        }
	}
}




impl<T: Trait> Module<T> {

	/// Checks if the ParachainStatus matches the provided StatusCoce
	///
	/// # Arguments
	///
	/// * `status_code` - to-be-checked StatusCode enum
	fn check_parachain_status(status_code: StatusCode) -> bool {
		return status_code == <ParachainStatus>::get();
	}

	/// Checks if the given ErrorCode is contains in Errors
	///
	/// # Arguments
	///
	/// * `error_code` - to-be-checked ErrorCode enum
	fn check_parachain_error(error_code: ErrorCode) -> bool {
		return <Errors>::get().contains(&(error_code as u8));
	}
}

decl_event!(
	pub enum Event<T> where
        AccountId = <T as system::Trait>::AccountId
    {
        RegisterStakedRelayer(AccountId, u64),
        DeRegisterStakedRelayer(AccountId),
        StatusUpdateSuggested(StatusCode, BTreeSet<u8>, BTreeSet<u8>, String, AccountId),
		VoteOnStatusUpdate(U256, AccountId, bool),
		ExecuteStatusUpdate(StatusCode, BTreeSet<u8>, BTreeSet<u8>, String),
		RejectStatusUpdate(StatusCode, BTreeSet<u8>, BTreeSet<u8>, String),
		ForceStatusUpdate(StatusCode, BTreeSet<u8>, BTreeSet<u8>, String),
		SlashStakedRelayer(AccountId),
		ReportVaultTheft(AccountId),
	}
);

decl_error! {
    pub enum Error for Module<T: Trait> {
    AlreadyRegistered,
    InsufficientStake,
    NotRegistered,
    GovernanceOnly,
    StakedRelayersOnly,
    StatusUpdateNotFound,
    InsufficientYesVotes,
    InsufficientNoVotes,
    VaultAlreadyReported,
    VaultNotFound,
    VaultAlreadyLiquidated,
    ValidRedeemOrReplace,
    ValidMergeTransaction,
    CollateralOk,
    OracleOnline
    }
}
