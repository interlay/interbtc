#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests; 

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure};
use system::ensure_signed;
use frame_support::traits::Currency;
use codec::{Encode, Decode};

/// The pallet's configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Dot currency
    type Currency: Currency<Self::AccountId>;

    /// Voter threshold
    const STAKED_RELAYER_VOTE_THRESHOLD: u8 = 0;
   
    // /// Minimum Staked Relayer stake
	const STAKED_RELAYER_STAKE: Self::Currency;

}

pub type DOT<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

// Indicated the status of the BTC Parachain.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
enum StatusCode {
	Running = 0,
	Error = 1,
	Shutdown = 2
}

// Enum specifying errors which lead to the Error status
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum ErrorCodes {
    NoDataBTCRelay = 1,
	InvalidBTCRelay = 2,
	OracleOffline = 3,
	Liquidation = 4
}

// Indicates the state of a proposed StatusUpdate.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum ProposalStatus {
	Pending = 0,
	Accepted = 1, 
	Rejected = 2
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StatusUpdate<StatusCode, ErrorCode, BlockNumber, AccountId> {
	newStatusCode: StatusCode, 
	oldStatusCode: StatusCode,
	addErrors: HashSet<ErrorCode>,
	removeErrors: HashSet<ErrorCode>,
	time: BlockNumber, 
	msg: String, 
	votesYes: HashSet<AccountId>,
	votesNo: HashSet<AccountId>
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StakedRelayer<Currency> {
      stake: Currency,
}


// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as SecurityModule {

		StakedRelayers get(fn stakedrelayer): map T::AccountId => StakedRelayer<DOT<T>>; 

	}
}

// The pallet's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		fn deposit_event() = default;
        
        // Initialize errors
        type Error = Error<T>;

        fn register_staked_relayer(origin, stake: DOT<T>) -> DispatchResult {
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

decl_event!(
	pub enum Event<T> where 
        AccountId = <T as system::Trait>::AccountId,
        DOT = DOT<T>
    {
        RegisterStakedRelayer(AccountId, DOT),
	}
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// This AccountId is already registered as a Staked Relayer
        AlreadyRegistered,
        /// Insufficient stake provided
        InsufficientStake,
    }
}
