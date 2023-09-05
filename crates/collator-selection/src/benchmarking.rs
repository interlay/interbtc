// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Benchmarking setup for collator-selection

use super::*;

#[allow(unused)]
use crate::Pallet as CollatorSelection;
use frame_benchmarking::v2::*;
use frame_support::{
    assert_ok,
    codec::Decode,
    dispatch::fmt::Debug,
    traits::{Currency, EnsureOrigin, Get},
};
use frame_system::{EventRecord, Pallet as System, RawOrigin};
use pallet_authorship::EventHandler;
use pallet_session::{self as session, SessionManager};
use sp_runtime::traits::Bounded;
use sp_std::prelude::*;

pub type BalanceOf<T> = <<T as Config>::StakingCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const SEED: u32 = 0;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = BalanceOf::<T>::max_value() / 2u32.into();
    let _ = T::StakingCurrency::make_free_balance_be(&user, balance);
    user
}

fn keys<T: Config + session::Config>(c: u32) -> <T as session::Config>::Keys {
    use rand::{RngCore, SeedableRng};

    let keys = {
        let mut keys = [0u8; 128];

        if c > 0 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(c as u64);
            rng.fill_bytes(&mut keys);
        }

        keys
    };

    Decode::decode(&mut &keys[..]).unwrap()
}

fn validator<T: Config + session::Config>(c: u32) -> (T::AccountId, <T as session::Config>::Keys) {
    (create_funded_user::<T>("candidate", c), keys::<T>(c))
}

fn register_validators<T: Config + session::Config>(count: u32) -> Vec<T::AccountId> {
    let validators = (0..count).map(|c| validator::<T>(c)).collect::<Vec<_>>();

    for (who, keys) in validators.clone() {
        System::<T>::inc_providers(&who);
        <session::Pallet<T>>::set_keys(RawOrigin::Signed(who).into(), keys, Vec::new()).unwrap();
    }

    validators.into_iter().map(|(who, _)| who).collect()
}

fn register_candidates<T: Config>(count: u32) {
    let candidates = (0..count).map(|c| account("candidate", c, SEED)).collect::<Vec<_>>();
    assert!(<CandidacyBond<T>>::get() > 0u32.into(), "Bond cannot be zero!");

    for who in candidates {
        T::StakingCurrency::make_free_balance_be(&who, <CandidacyBond<T>>::get() * 2u32.into());
        assert_ok!(<CollatorSelection<T>>::register_as_candidate(
            RawOrigin::Signed(who).into()
        ));
    }
}

#[benchmarks(where
    T: pallet_authorship::Config + session::Config,
    T::RuntimeOrigin: Debug
)]
pub mod benchmarks {
    use super::*;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[benchmark]
    fn set_invulnerables(b: Linear<1, 100>) {
        let new_invulnerables = register_validators::<T>(b);
        let origin = T::UpdateOrigin::try_successful_origin().unwrap().into().unwrap();

        #[extrinsic_call]
        _(origin, new_invulnerables.clone());

        assert_last_event::<T>(
            Event::NewInvulnerables {
                invulnerables: new_invulnerables,
            }
            .into(),
        );
    }

    #[benchmark]
    fn set_desired_candidates() {
        let max: u32 = 999;
        let origin = T::UpdateOrigin::try_successful_origin().unwrap().into().unwrap();

        #[extrinsic_call]
        _(origin, max.clone());

        assert_last_event::<T>(
            Event::NewDesiredCandidates {
                desired_candidates: max,
            }
            .into(),
        );
    }

    #[benchmark]
    fn set_candidacy_bond() {
        let bond_amount = BalanceOf::<T>::max_value() / 4u32.into();
        let origin = T::UpdateOrigin::try_successful_origin().unwrap().into().unwrap();

        #[extrinsic_call]
        _(origin, bond_amount.clone());

        assert_last_event::<T>(Event::NewCandidacyBond { bond_amount }.into());
    }

    // worse case is when we have all the max-candidate slots filled except one, and we fill that
    // one.
    #[benchmark]
    fn register_as_candidate(c: Linear<1, 19>) {
        <CandidacyBond<T>>::put(BalanceOf::<T>::max_value() / 4u32.into());
        <DesiredCandidates<T>>::put(c + 1);

        register_validators::<T>(c);
        register_candidates::<T>(c);

        let caller: T::AccountId = whitelisted_caller();
        let bond = <CandidacyBond<T>>::get() * 2u32.into();
        T::StakingCurrency::make_free_balance_be(&caller, bond.clone());

        System::<T>::inc_providers(&caller);
        assert_ok!(<session::Pallet<T>>::set_keys(
            RawOrigin::Signed(caller.clone()).into(),
            keys::<T>(c + 1),
            Vec::new()
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()));

        assert_last_event::<T>(
            Event::CandidateAdded {
                account_id: caller,
                deposit: bond / 2u32.into(),
            }
            .into(),
        );
    }

    // worse case is the last candidate leaving.
    #[benchmark]
    fn leave_intent(c: Linear<6, 20>) {
        <CandidacyBond<T>>::put(BalanceOf::<T>::max_value() / 4u32.into());
        <DesiredCandidates<T>>::put(c);

        register_validators::<T>(c);
        register_candidates::<T>(c);

        let leaving = <Candidates<T>>::get().last().unwrap().who.clone();
        whitelist_account!(leaving);

        #[extrinsic_call]
        _(RawOrigin::Signed(leaving.clone()));

        assert_last_event::<T>(Event::CandidateRemoved { account_id: leaving }.into());
    }

    // worse case is paying a non-existing candidate account.
    #[benchmark]
    fn note_author() {
        <CandidacyBond<T>>::put(BalanceOf::<T>::max_value() / 4u32.into());
        T::RewardsCurrency::make_free_balance_be(&<CollatorSelection<T>>::account_id(), 2000u32.into());
        let author = account("author", 0, SEED);
        let new_block: BlockNumberFor<T> = 10u32.into();

        frame_system::Pallet::<T>::set_block_number(new_block);
        assert!(T::RewardsCurrency::free_balance(&author) == 0u32.into());

        #[block]
        {
            <CollatorSelection<T> as EventHandler<_, _>>::note_author(author.clone());
        }

        assert!(T::RewardsCurrency::free_balance(&author) > 0u32.into());
        assert_eq!(frame_system::Pallet::<T>::block_number(), new_block);
    }

    // worst case for new session.
    #[benchmark]
    fn new_session(r: Linear<1, 20>, c: Linear<1, 20>) {
        <CandidacyBond<T>>::put(BalanceOf::<T>::max_value() / 4u32.into());
        <DesiredCandidates<T>>::put(c);
        frame_system::Pallet::<T>::set_block_number(0u32.into());

        register_validators::<T>(c);
        register_candidates::<T>(c);

        let new_block: BlockNumberFor<T> = 1800u32.into();
        let zero_block: BlockNumberFor<T> = 0u32.into();
        let candidates = <Candidates<T>>::get();

        let non_removals = c.saturating_sub(r);

        for i in 0..c {
            <LastAuthoredBlock<T>>::insert(candidates[i as usize].who.clone(), zero_block);
        }

        if non_removals > 0 {
            for i in 0..non_removals {
                <LastAuthoredBlock<T>>::insert(candidates[i as usize].who.clone(), new_block);
            }
        } else {
            for i in 0..c {
                <LastAuthoredBlock<T>>::insert(candidates[i as usize].who.clone(), new_block);
            }
        }

        let pre_length = <Candidates<T>>::get().len();

        frame_system::Pallet::<T>::set_block_number(new_block);

        assert!(<Candidates<T>>::get().len() == c as usize);

        #[block]
        {
            <CollatorSelection<T> as SessionManager<_>>::new_session(0);
        }

        if c > r && non_removals >= T::MinCandidates::get() {
            assert!(<Candidates<T>>::get().len() < pre_length);
        } else if c > r && non_removals < T::MinCandidates::get() {
            assert!(<Candidates<T>>::get().len() == T::MinCandidates::get() as usize);
        } else {
            assert!(<Candidates<T>>::get().len() == pre_length);
        }
    }

    impl_benchmark_test_suite!(CollatorSelection, crate::mock::new_test_ext(), crate::mock::Test,);
}
