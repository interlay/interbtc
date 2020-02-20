#![cfg_attr(not(feature = "std"), no_std)]

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
   
    // /// Minimum stake
    const MINIMUM_STAKE: Self::Currency;
}

pub type DOT<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

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

/// tests for this pallet
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the pallet, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
	}
	impl Trait for Test {
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		new_test_ext().execute_with(|| {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
