//! # PolkaBTC Nomination Module

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod types;

use std::convert::TryInto;

use codec::{Decode, Encode, EncodeLike};

use frame_benchmarking::Zero;
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::{Currency, Randomness};
use frame_support::weights::Weight;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, transactional};
use frame_system::ensure_signed;
use primitive_types::H256;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::CheckedSub;
use types::{DefaultOperator, Operator, RichOperator};

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> = <<T as treasury::Config>::PolkaBTC as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VaultStatus {
    /// Vault is active
    Active = 0,

    /// Vault has been liquidated
    Liquidated = 1,

    /// Vault theft has been reported
    CommittedTheft = 2,
}

impl Default for VaultStatus {
    fn default() -> Self {
        VaultStatus::Active
    }
}

pub trait WeightInfo {
    fn opt_in_to_nomination() -> Weight;
    fn opt_out_of_nomination() -> Weight;
    fn deposit_nominated_collateral() -> Weight;
    fn withdraw_nominated_collateral() -> Weight;
    fn request_operator_withdrawal() -> Weight;
    fn request_nominator_withdrawal() -> Weight;
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config + collateral::Config + treasury::Config + security::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type RandomnessSource: Randomness<H256>;

    type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Nomination {
        /// ## Storage

        /// Flag indicating whether this feature is enabled
        NominationEnabled get(fn nomination_enabled) config(): bool;

        /// Upper limit, expressed as a rate out of a Vault's collateral, that can be
        /// nominated as collateral
        NominatedCollateralUpperLimitRate get(fn nominated_collateral_upper_limit_rate) config(): UnsignedFixedPoint<T>;

        /// Base unbonding period by which collateral withdrawal requests from Vault Operators
        /// are delayed
        OperatorUnbondingPeriod get(fn operator_unbonding_period) config(): T::BlockNumber;

        /// Base unbonding period by which collateral withdrawal requests from Vault Nominators
        /// are delayed
        NominatorUnbondingPeriod get(fn nominator_unbonding_period) config(): T::BlockNumber;

        /// Map of Vault Operators
        Operators: map hasher(blake2_128_concat) T::AccountId => Operator<T::AccountId, T::BlockNumber, DOT<T>>;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        DOT = DOT<T>,
    {
        NominationOptIn(AccountId),
        NominationOptOut(AccountId),
        IncreaseNominatedCollateral(AccountId, AccountId, DOT),
        DecreaseNominatedCollateral(AccountId, AccountId, DOT),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            0
        }

        /// Nominate collateral to a Vault Operator
        #[weight = <T as Config>::WeightInfo::deposit_nominated_collateral()]
        #[transactional]
        fn deposit_nominated_collateral(origin, vault_id: T::AccountId, collateral: DOT<T>) -> DispatchResult {
            Self::_deposit_nominated_collateral(&ensure_signed(origin)?, &vault_id, collateral)
        }

        /// Withdraw collateral from a Vault Operator
        #[weight = <T as Config>::WeightInfo::withdraw_nominated_collateral()]
        #[transactional]
        fn withdraw_nominated_collateral(origin, operator_id: T::AccountId, collateral: DOT<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let mut operator = Self::get_rich_operator_from_id(&operator_id)?;
            let height = <frame_system::Module<T>>::block_number();

            let nominator_unbonding_period = Self::get_nominator_unbonding_period();
            let nominator_unbonding_period_u128 = Self::blocknumber_to_u128(nominator_unbonding_period)?;

            let scaled_nominator_unbonding_period_u128 = operator
            .get_nominated_collateral_proportion_for(sender)?
            .checked_mul(nominator_unbonding_period_u128)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

            let scaled_nominator_unbonding_period = Self::u128_to_blocknumber(scaled_nominator_unbonding_period_u128)?;
            let maturity = height + scaled_nominator_unbonding_period;
            let request_id = ext::security::get_secure_id::<T>(&operator_id);
            operator.add_pending_operator_withdrawal(request_id, (maturity, collateral));
            Ok(())
        }

        /// Request withdrawal as a Vault Operator
        #[weight = <T as Config>::WeightInfo::request_operator_withdrawal()]
        #[transactional]
        fn request_operator_withdrawal(origin, collateral: DOT<T>) -> DispatchResult {
            Self::_request_operator_withdrawal(&ensure_signed(origin)?, collateral)
        }

        /// Request withdrawal as a Vault Operator
        #[weight = <T as Config>::WeightInfo::request_nominator_withdrawal()]
        #[transactional]
        fn request_nominator_withdrawal(origin, collateral: DOT<T>) -> DispatchResult {
            Self::_request_nominator_withdrawal(&ensure_signed(origin)?, collateral)
        }
    }
}

impl<T: Config> Module<T> {
    pub fn is_nominated_collateral_below_limit_rate(
        vault_collateral: DOT<T>,
        nominated_collateral: DOT<T>,
    ) -> Result<bool, DispatchError> {
        let limit_rate = <NominatedCollateralUpperLimitRate<T>>::get();
        let vault_collateral_inner = Self::dot_to_inner_fixed_point(vault_collateral)?;
        let nominated_collateral_inner = Self::dot_to_inner_fixed_point(nominated_collateral)?;

        let limit_collateral =
            UnsignedFixedPoint::<T>::checked_from_integer(vault_collateral_inner)
                .ok_or(Error::<T>::TryIntoIntError)?
                .checked_mul(&limit_rate)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
        Ok(
            UnsignedFixedPoint::<T>::checked_from_integer(nominated_collateral_inner)
                .ok_or(Error::<T>::TryIntoIntError)?
                .le(&limit_collateral),
        )
    }

    pub fn set_nominated_collateral_upper_limit_rate(limit: UnsignedFixedPoint<T>) {
        <NominatedCollateralUpperLimitRate<T>>::set(limit);
    }

    pub fn get_nominated_collateral_upper_limit_rate() -> UnsignedFixedPoint<T> {
        <NominatedCollateralUpperLimitRate<T>>::get()
    }

    pub fn get_operator_unbonding_period() -> T::BlockNumber {
        <OperatorUnbondingPeriod<T>>::get()
    }

    pub fn get_nominator_unbonding_period() -> T::BlockNumber {
        <NominatorUnbondingPeriod<T>>::get()
    }

    pub fn _request_operator_withdrawal(
        vault_id: &T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        if Self::is_nomination_enabled()? {
            // get operator object
            // ensure!(
            //     remaining_collateral.ge(&self.data.total_nominated_collateral),
            //     Error::<T>::InsufficientCollateral
            // );
            let height = <frame_system::Module<T>>::block_number();
            let maturity = height + Self::get_operator_unbonding_period();
            let request_id = ext::security::get_secure_id::<T>(vault_id);
            Self::add_pending_operator_withdrawal(request_id, (maturity, collateral));
        }
        Ok(())
    }

    // TODO: Deposit and withdraw through the vault_registry
    pub fn _deposit_nominated_collateral(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        vault.deposit_nominated_collateral(nominator_id.clone(), collateral)?;
        Self::deposit_event(Event::<T>::IncreaseNominatedCollateral(
            nominator_id.clone(),
            vault_id.clone(),
            collateral,
        ));
        Ok(())
    }

    pub fn _withdraw_nominated_collateral(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.withdraw_nominated_collateral(nominator_id.clone(), collateral)?;
        Self::deposit_event(Event::<T>::DecreaseNominatedCollateral(
            nominator_id.clone(),
            operator_id.clone(),
            collateral,
        ));
        Ok(())
    }

    pub fn slash_nominators(
        vault_id: T::AccountId,
        status: VaultStatus,
        to_slash: DOT<T>,
        backing_collateral_before_slashing: DOT<T>,
    ) -> DispatchResult {
        let rich_operator = Self::get_rich_operator_from_id(&vault_id.clone())?;
        rich_operator.slash_nominators(status, to_slash, backing_collateral_before_slashing)
    }

    pub fn opt_in_to_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            !<Operators<T>>::contains_key(operator_id),
            Error::<T>::VaultAlreadyOptedInToNomination
        );
        let mut operator: RichOperator<T> = Operator::new(operator_id.clone()).into();
        <Operators<T>>::insert(operator_id, operator.data.clone());
        Ok(())
    }

    pub fn opt_out_of_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            <Operators<T>>::contains_key(operator_id),
            Error::<T>::VaultNotOptedInToNomination
        );
        let operator = Self::get_rich_operator_from_id(operator_id)?;

        if operator.data.nominators.len() > 0 {
            operator.refund_nominated_collateral()?;
        }
        <Operators<T>>::remove(operator_id);

        Self::deposit_event(Event::<T>::NominationOptOut(operator_id.clone()));
        Ok(())
    }

    pub fn is_nomination_enabled() -> Result<bool, DispatchError> {
        Ok(Self::nomination_enabled())
    }

    pub fn is_operator(operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(Self::operator_exists(&operator_id))
    }

    pub fn get_total_nominated_collateral(
        operator_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.data.total_nominated_collateral)
    }

    pub fn get_operator_from_id(
        operator_id: &T::AccountId,
    ) -> Result<DefaultOperator<T>, DispatchError> {
        ensure!(
            Self::operator_exists(&operator_id),
            Error::<T>::OperatorNotFound
        );
        let operator = <Operators<T>>::get(operator_id);
        Ok(operator)
    }

    fn get_rich_operator_from_id(
        operator_id: &T::AccountId,
    ) -> Result<RichOperator<T>, DispatchError> {
        Ok(Self::get_operator_from_id(operator_id)?.into())
    }

    fn operator_exists(id: &T::AccountId) -> bool {
        <Operators<T>>::contains_key(id)
    }

    fn dot_to_u128(x: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn blocknumber_to_u128(x: T::BlockNumber) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_blocknumber(x: u128) -> Result<T::BlockNumber, DispatchError> {
        TryInto::<T::BlockNumber>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn dot_to_inner_fixed_point(
        x: DOT<T>,
    ) -> Result<<<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        let x_u128 = Self::dot_to_u128(x)?;

        // calculate how many tokens should be maximally issued given the threshold.
        let vault_collateral_as_inner =
            TryInto::try_into(x_u128).map_err(|_| Error::<T>::TryIntoIntError)?;
        Ok(vault_collateral_as_inner)
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Account has insufficient balance
        InsufficientFunds,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        WithdrawalNotUnbonded,
        NominatorLiquidationFailed,
        NominatorNotFound,
        TooLittleDelegatedCollateral,
        VaultAlreadyOptedInToNomination,
        VaultNotOptedInToNomination,
        VaultNotQualifiedToOptOutOfNomination,
        TryIntoIntError,
        OperatorNotFound
    }
}
