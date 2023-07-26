use crate::{BaseCallFilter, NativeCurrency, Runtime, RuntimeCall, RuntimeEvent, Timestamp, Weight};
use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::FullTransactionProof;
use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    parameter_types,
    traits::{
        fungible::Inspect,
        tokens::{DepositConsequence, Fortitude, Preservation, Provenance, WithdrawConsequence},
        ExistenceRequirement, Get, Nothing, Randomness, ReservableCurrency, SignedImbalance, WithdrawReasons,
    },
};
use orml_traits::BalanceStatus;
use primitives::{self, AccountId, Balance, BlockNumber, Hash};
use sp_core::crypto::UncheckedFrom;
use sp_std::prelude::*;

use sp_runtime::traits::Convert;
pub struct NoRandomness;
impl Randomness<Hash, BlockNumber> for NoRandomness {
    fn random(_subject: &[u8]) -> (Hash, BlockNumber) {
        // this is deprecated so don't bother to implement
        unimplemented!()
    }
}

pub struct DummyWeightPrice;
impl Convert<Weight, Balance> for DummyWeightPrice {
    fn convert(_a: Weight) -> Balance {
        // informational only, leaving blank for now
        Default::default()
    }
}

// contracts
parameter_types! {
    pub const DeletionQueueDepth: u32 = 10;
    pub const DeletionWeightLimit: Weight = Weight::from_ref_time(100000000 as u64);
    pub const DepositPerByte: Balance = 1;
    pub const DepositPerItem: Balance = 1;
    pub const MaxCodeLen: u32 = 123 * 1024;
    pub const MaxStorageKeyLen: u32 = 128;
    pub const UnsafeUnstableInterface: bool = false;
    pub const MaxDebugBufferLen: u32 = 2 * 1024 * 1024;
    pub const DefaultDepositLimit: Balance = 1_000_000_000_000;
}

pub struct NativeCurrencyWithEd;

impl Inspect<AccountId> for NativeCurrencyWithEd {
    type Balance = Balance;

    fn total_issuance() -> Self::Balance {
        <NativeCurrency as Inspect<AccountId>>::total_issuance()
    }
    fn active_issuance() -> Self::Balance {
        <NativeCurrency as Inspect<AccountId>>::active_issuance()
    }
    fn minimum_balance() -> Self::Balance {
        1
    }

    fn total_balance(who: &AccountId) -> Self::Balance {
        <NativeCurrency as Inspect<AccountId>>::total_balance(who)
    }

    fn balance(who: &AccountId) -> Self::Balance {
        <NativeCurrency as Inspect<AccountId>>::balance(who)
    }

    fn reducible_balance(who: &AccountId, preservation: Preservation, force: Fortitude) -> Self::Balance {
        <NativeCurrency as Inspect<AccountId>>::reducible_balance(who, preservation, force)
    }

    fn can_deposit(who: &AccountId, amount: Self::Balance, provenance: Provenance) -> DepositConsequence {
        <NativeCurrency as Inspect<AccountId>>::can_deposit(who, amount, provenance)
    }

    fn can_withdraw(who: &AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
        <NativeCurrency as Inspect<AccountId>>::can_withdraw(who, amount)
    }
}

impl ReservableCurrency<AccountId> for NativeCurrencyWithEd {
    fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
        NativeCurrency::can_reserve(who, value)
    }
    fn slash_reserved(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        NativeCurrency::slash_reserved(who, value)
    }
    fn reserved_balance(who: &AccountId) -> Self::Balance {
        NativeCurrency::reserved_balance(who)
    }
    fn reserve(who: &AccountId, value: Self::Balance) -> DispatchResult {
        NativeCurrency::reserve(who, value)
    }
    fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
        NativeCurrency::unreserve(who, value)
    }
    fn repatriate_reserved(
        slashed: &AccountId,
        beneficiary: &AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> Result<Self::Balance, DispatchError> {
        NativeCurrency::repatriate_reserved(slashed, beneficiary, value, status)
    }
}

impl frame_support::traits::Currency<AccountId> for NativeCurrencyWithEd {
    type Balance = <NativeCurrency as frame_support::traits::Currency<AccountId>>::Balance;
    type NegativeImbalance = <NativeCurrency as frame_support::traits::Currency<AccountId>>::NegativeImbalance;
    type PositiveImbalance = <NativeCurrency as frame_support::traits::Currency<AccountId>>::PositiveImbalance;

    fn total_balance(who: &AccountId) -> Self::Balance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::total_balance(who)
    }
    fn can_slash(who: &AccountId, value: Self::Balance) -> bool {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::can_slash(who, value)
    }
    fn total_issuance() -> Self::Balance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::total_issuance()
    }
    fn active_issuance() -> Self::Balance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::active_issuance()
    }
    fn deactivate(x: Self::Balance) {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::deactivate(x)
    }
    fn reactivate(x: Self::Balance) {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::reactivate(x)
    }
    fn minimum_balance() -> Self::Balance {
        1
    }
    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::burn(amount)
    }
    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::issue(amount)
    }
    fn pair(amount: Self::Balance) -> (Self::PositiveImbalance, Self::NegativeImbalance) {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::pair(amount)
    }
    fn free_balance(who: &AccountId) -> Self::Balance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::free_balance(who)
    }
    fn ensure_can_withdraw(
        who: &AccountId,
        _amount: Self::Balance,
        reasons: WithdrawReasons,
        new_balance: Self::Balance,
    ) -> DispatchResult {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::ensure_can_withdraw(
            who,
            _amount,
            reasons,
            new_balance,
        )
    }
    fn transfer(
        source: &AccountId,
        dest: &AccountId,
        value: Self::Balance,
        existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::transfer(
            source,
            dest,
            value,
            existence_requirement,
        )
    }
    fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::slash(who, value)
    }
    fn deposit_into_existing(who: &AccountId, value: Self::Balance) -> Result<Self::PositiveImbalance, DispatchError> {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::deposit_into_existing(who, value)
    }
    fn resolve_into_existing(who: &AccountId, value: Self::NegativeImbalance) -> Result<(), Self::NegativeImbalance> {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::resolve_into_existing(who, value)
    }
    fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::deposit_creating(who, value)
    }
    fn resolve_creating(who: &AccountId, value: Self::NegativeImbalance) {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::resolve_creating(who, value)
    }
    fn withdraw(
        who: &AccountId,
        value: Self::Balance,
        reasons: WithdrawReasons,
        liveness: ExistenceRequirement,
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::withdraw(who, value, reasons, liveness)
    }
    fn settle(
        who: &AccountId,
        value: Self::PositiveImbalance,
        reasons: WithdrawReasons,
        liveness: ExistenceRequirement,
    ) -> Result<(), Self::PositiveImbalance> {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::settle(who, value, reasons, liveness)
    }
    fn make_free_balance_be(
        who: &AccountId,
        balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        <NativeCurrency as frame_support::traits::Currency<AccountId>>::make_free_balance_be(who, balance)
    }
}

use pallet_contracts::chain_extension::{ChainExtension, Environment, Ext, InitState, RetVal, SysConfig};

#[derive(Default)]
pub struct BtcRelayExtension;

impl ChainExtension<Runtime> for BtcRelayExtension {
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id = env.func_id();
        match func_id {
            1101 => {
                let mut env = env.buf_in_buf_out();

                let (unchecked_proof, btc_address): (FullTransactionProof, Vec<u8>) =
                    env.read_as_unbounded(env.in_len())?;

                let btc_address = Decode::decode(&mut &btc_address[..]).unwrap();
                let sats: Option<u64> =
                    btc_relay::Pallet::<Runtime>::get_and_verify_issue_payment::<Balance>(unchecked_proof, btc_address)
                        .ok()
                        .and_then(|x| x.try_into().ok());

                env.write(&sats.encode(), false, None)
                    .map_err(|_| DispatchError::Other("ChainExtension failed"))?;
            }
            _ => return Err(DispatchError::Other("Unimplemented func_id")),
        }
        Ok(RetVal::Converging(0))
    }

    fn enabled() -> bool {
        true
    }
}
pub struct DefaultSchedule;

impl Get<pallet_contracts::Schedule<Runtime>> for DefaultSchedule {
    fn get() -> pallet_contracts::Schedule<Runtime> {
        Default::default()
    }
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = NoRandomness;
    type Currency = NativeCurrencyWithEd;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type WeightPrice = DummyWeightPrice;
    type WeightInfo = ();
    type ChainExtension = BtcRelayExtension;
    type Schedule = DefaultSchedule;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type DefaultDepositLimit = DefaultDepositLimit;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = MaxCodeLen;
    type MaxStorageKeyLen = MaxStorageKeyLen;
    type UnsafeUnstableInterface = UnsafeUnstableInterface;
    type MaxDebugBufferLen = MaxDebugBufferLen;
}
