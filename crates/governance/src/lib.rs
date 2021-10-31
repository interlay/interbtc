//! # Governance Module
//! Used to vote on and dispatch calls.

// #![deny(warnings)]
#![recursion_limit = "256"]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, Get, OnUnbalanced, ReservableCurrency},
    transactional,
    weights::Pays,
};
use frame_system::{ensure_signed, RawOrigin};
use scale_info::TypeInfo;
use sp_runtime::traits::{Dispatchable, Hash};
use sp_std::fmt::Debug;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct Challenge<AccountId> {
    challenger: AccountId,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct Proposal<AccountId, Balance, BlockNumber> {
    data: Vec<u8>,
    proposer: AccountId,
    deposit: Balance,
    start: BlockNumber,
    end: BlockNumber,
    challenge: Option<Challenge<AccountId>>,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Currency type for this pallet.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The base proposal type.
        type Proposal: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;

        /// How often (in blocks) to check for new votes.
        #[pallet::constant]
        type DisputePeriod: Get<Self::BlockNumber>;

        /// Origin from which approvals must come.
        type ApproveOrigin: EnsureOrigin<Self::Origin>;

        /// Origin from which rejections must come.
        type RejectOrigin: EnsureOrigin<Self::Origin>;

        /// Handler for the unbalanced reduction when slashing a deposit.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Proposed {
            proposal_hash: T::Hash,
            proposer: T::AccountId,
            deposit: BalanceOf<T>,
        },
        Challenge {
            proposal_hash: T::Hash,
            challenger: T::AccountId,
        },
        Executed {
            proposal_hash: T::Hash,
            result: DispatchResult,
        },
        Approved {
            proposal_hash: T::Hash,
        },
        Rejected {
            proposal_hash: T::Hash,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        DuplicateProposal,
        ProposalMissing,
        InvalidProposal,
        NotReady,
        Challenged,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    pub type Proposals<T: Config> =
        StorageMap<_, Identity, T::Hash, Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber>>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn create_proposal(origin: OriginFor<T>, encoded_proposal: Vec<u8>) -> DispatchResultWithPostInfo {
            Self::_create_proposal(ensure_signed(origin)?, encoded_proposal)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn challenge_proposal(origin: OriginFor<T>, proposal_hash: T::Hash) -> DispatchResultWithPostInfo {
            Self::_challenge_proposal(ensure_signed(origin)?, proposal_hash)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn finalize_proposal(origin: OriginFor<T>, proposal_hash: T::Hash) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;
            Self::_finalize_proposal(proposal_hash)?;
            Ok(Pays::No.into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn approve_proposal(origin: OriginFor<T>, proposal_hash: T::Hash) -> DispatchResultWithPostInfo {
            T::ApproveOrigin::ensure_origin(origin)?;
            Self::execute_proposal(proposal_hash)?;
            Self::deposit_event(Event::<T>::Approved { proposal_hash });
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn reject_proposal(origin: OriginFor<T>, proposal_hash: T::Hash) -> DispatchResultWithPostInfo {
            T::RejectOrigin::ensure_origin(origin)?;
            let Proposal { proposer, deposit, .. } =
                <Proposals<T>>::take(proposal_hash).ok_or(Error::<T>::ProposalMissing)?;
            T::Slash::on_unbalanced(T::Currency::slash_reserved(&proposer, deposit).0);
            Self::deposit_event(Event::<T>::Rejected { proposal_hash });
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    fn _create_proposal(who: T::AccountId, encoded_proposal: Vec<u8>) -> DispatchResult {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        ensure!(
            !<Proposals<T>>::contains_key(&proposal_hash),
            Error::<T>::DuplicateProposal
        );

        // TODO: per byte deposit?
        let deposit = <BalanceOf<T>>::from(encoded_proposal.len() as u32);
        T::Currency::reserve(&who, deposit)?;

        let start = frame_system::Pallet::<T>::block_number();
        let end = start + T::DisputePeriod::get();
        let proposal = Proposal {
            data: encoded_proposal,
            proposer: who.clone(),
            deposit,
            start,
            end,
            challenge: None,
        };

        <Proposals<T>>::insert(proposal_hash, proposal);
        Self::deposit_event(Event::<T>::Proposed {
            proposal_hash,
            proposer: who,
            deposit,
        });

        Ok(())
    }

    // TODO: do we require a deposit?
    fn _challenge_proposal(who: T::AccountId, proposal_hash: T::Hash) -> DispatchResult {
        let now = frame_system::Pallet::<T>::block_number();
        <Proposals<T>>::mutate(proposal_hash, |maybe_proposal| {
            if let Some(proposal) = maybe_proposal {
                proposal.end = now;
                proposal.challenge = Some(Challenge {
                    challenger: who.clone(),
                });
                Ok(())
            } else {
                Err(Error::<T>::ProposalMissing)
            }
        })?;

        Self::deposit_event(Event::<T>::Challenge {
            proposal_hash,
            challenger: who,
        });
        Ok(())
    }

    fn _finalize_proposal(proposal_hash: T::Hash) -> DispatchResult {
        let now = frame_system::Pallet::<T>::block_number();
        let Proposal { challenge, end, .. } = <Proposals<T>>::get(proposal_hash).ok_or(Error::<T>::ProposalMissing)?;

        if let Some(_) = challenge {
            Err(Error::<T>::Challenged.into())
        } else if end >= now {
            Self::execute_proposal(proposal_hash)
        } else {
            Err(Error::<T>::NotReady.into())
        }
    }

    fn execute_proposal(proposal_hash: T::Hash) -> DispatchResult {
        let Proposal {
            data,
            proposer,
            deposit,
            ..
        } = <Proposals<T>>::take(proposal_hash).ok_or(Error::<T>::ProposalMissing)?;

        let proposal = T::Proposal::decode(&mut &data[..]).or(Err(Error::<T>::InvalidProposal))?;
        // TODO: schedule?
        let result = proposal
            .dispatch(RawOrigin::Root.into())
            .map(|_| ())
            .map_err(|e| e.error);

        T::Currency::unreserve(&proposer, deposit);

        Self::deposit_event(Event::<T>::Executed { proposal_hash, result });
        Ok(())
    }
}
