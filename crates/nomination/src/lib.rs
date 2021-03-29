//! # PolkaBTC Nomination Module

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod types;

mod default_weights;

use sp_std::convert::TryInto;

use codec::{Decode, Encode, EncodeLike};

use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::Currency;
use frame_support::weights::Weight;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, transactional};
use frame_system::ensure_root;
use primitive_types::H256;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::CheckedMul;
use types::{DefaultOperator, Operator, RichOperator};

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;
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
    fn set_nomination_enabled() -> Weight;
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config + collateral::Config + treasury::Config + security::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

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
        MaxNominationRatio get(fn max_nomination_ratio) config(): UnsignedFixedPoint<T>;

        /// Base unbonding period by which collateral withdrawal requests from Vault Operators
        /// are delayed
        OperatorUnbondingPeriod get(fn operator_unbonding_period) config(): T::BlockNumber;

        /// Base unbonding period by which collateral withdrawal requests from Vault Nominators
        /// are delayed
        NominatorUnbondingPeriod get(fn nominator_unbonding_period) config(): T::BlockNumber;

        /// Sum of collateral in pending withdraw requests
        CollateralToBeWithdrawn get(fn collateral_to_be_withdrawn) config(): DOT<T>;

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
        BlockNumber = <T as frame_system::Config>::BlockNumber,
    {
        // [operator_id]
        NominationOptIn(AccountId),
        // [operator_id]
        NominationOptOut(AccountId),
        // [nominator_id, operator_id, collateral]
        IncreaseNominatedCollateral(AccountId, AccountId, DOT),
        // [nominator_id, operator_id, collateral]
        WithdrawNominatedCollateral(AccountId, AccountId, DOT),
        // [request_id, operator_id, maturity_block, collateral]
        RequestOperatorCollateralWithdrawal(H256, AccountId, BlockNumber, DOT),
        // [request_id, operator_id, collateral]
        ExecuteOperatorCollateralWithdrawal(H256, AccountId, DOT),
        // [request_id, operator_id, collateral]
        CancelOperatorCollateralWithdrawal(H256, AccountId, DOT),
        // [request_id, nominator_id, operator_id, maturity_block, collateral]
        RequestNominatorCollateralWithdrawal(H256, AccountId, AccountId, BlockNumber, DOT),
        // [request_id, nominator_id, operator_id, collateral]
        ExecuteNominatorCollateralWithdrawal(H256, AccountId, AccountId, DOT),
        // [request_id, nominator_id, operator_id, collateral]
        CancelNominatorCollateralWithdrawal(H256, AccountId, AccountId, DOT),
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

        #[weight = <T as Config>::WeightInfo::set_nomination_enabled()]
        #[transactional]
        fn set_nomination_enabled(origin, enabled: bool) {
            ensure_root(origin)?;
            <NominationEnabled>::set(enabled);
        }
    }
}

impl<T: Config> Module<T> {
    pub fn is_collateral_below_max_nomination_ratio(
        vault_collateral: DOT<T>,
        nominated_collateral: DOT<T>,
    ) -> Result<bool, DispatchError> {
        let max_nomination_ratio = Self::get_max_nomination_ratio();
        let vault_collateral_inner = Self::dot_to_inner_fixed_point(vault_collateral)?;
        let nominated_collateral_inner = Self::dot_to_inner_fixed_point(nominated_collateral)?;

        let max_nominated_collateral =
            UnsignedFixedPoint::<T>::checked_from_integer(vault_collateral_inner)
                .ok_or(Error::<T>::TryIntoIntError)?
                .checked_mul(&max_nomination_ratio)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
        Ok(
            UnsignedFixedPoint::<T>::checked_from_integer(nominated_collateral_inner)
                .ok_or(Error::<T>::TryIntoIntError)?
                .le(&max_nominated_collateral),
        )
    }

    pub fn set_max_nomination_ratio(limit: UnsignedFixedPoint<T>) {
        <MaxNominationRatio<T>>::set(limit);
    }

    pub fn get_max_nomination_ratio() -> UnsignedFixedPoint<T> {
        <MaxNominationRatio<T>>::get()
    }

    pub fn set_operator_unbonding_period(period: T::BlockNumber) {
        <OperatorUnbondingPeriod<T>>::set(period)
    }

    pub fn get_operator_unbonding_period() -> T::BlockNumber {
        <OperatorUnbondingPeriod<T>>::get()
    }

    pub fn set_nominator_unbonding_period(period: T::BlockNumber) {
        <NominatorUnbondingPeriod<T>>::set(period)
    }

    pub fn get_nominator_unbonding_period() -> T::BlockNumber {
        <NominatorUnbondingPeriod<T>>::get()
    }

    pub fn request_operator_withdrawal(
        operator_id: &T::AccountId,
        collateral_to_withdraw: DOT<T>,
        backing_collateral_before_withdrawal: DOT<T>,
    ) -> DispatchResult {
        ensure!(
            Self::is_operator(operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        let request_id = ext::security::get_secure_id::<T>(operator_id);
        let height = <frame_system::Module<T>>::block_number();
        let maturity = height + Self::get_operator_unbonding_period();
        operator.add_pending_operator_withdrawal(
            request_id,
            collateral_to_withdraw,
            backing_collateral_before_withdrawal,
            maturity,
        )?;
        Self::deposit_event(Event::<T>::RequestOperatorCollateralWithdrawal(
            request_id,
            operator_id.clone(),
            maturity,
            collateral_to_withdraw,
        ));
        Ok(())
    }

    pub fn execute_operator_withdrawal(
        operator_id: &T::AccountId,
        request_id: H256,
    ) -> DispatchResult {
        ensure!(
            Self::is_operator(operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.execute_operator_withdrawal(request_id)
    }

    pub fn request_nominator_withdrawal(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        collateral_to_withdraw: DOT<T>,
    ) -> DispatchResult {
        ensure!(
            Self::is_operator(operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        let height = <frame_system::Module<T>>::block_number();
        let nominator_unbonding_period_u128 =
            Self::blocknumber_to_u128(Self::get_nominator_unbonding_period())?;
        let scaled_nominator_unbonding_period_u128 = operator
            .get_nominated_collateral_proportion_for(nominator_id.clone())?
            .checked_mul(nominator_unbonding_period_u128)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let maturity = height + Self::u128_to_blocknumber(scaled_nominator_unbonding_period_u128)?;
        let request_id = ext::security::get_secure_id::<T>(operator_id);
        operator.add_pending_nominator_withdrawal(
            nominator_id.clone(),
            request_id,
            collateral_to_withdraw,
            maturity,
        );
        Self::deposit_event(Event::<T>::RequestNominatorCollateralWithdrawal(
            request_id,
            nominator_id.clone(),
            operator_id.clone(),
            maturity,
            collateral_to_withdraw,
        ));
        Ok(())
    }

    pub fn execute_nominator_withdrawal(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        request_id: H256,
    ) -> DispatchResult {
        ensure!(
            Self::is_operator(operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.execute_nominator_withdrawal(nominator_id.clone(), request_id)
    }

    pub fn deposit_nominated_collateral(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
        collateral: DOT<T>,
        backing_collateral: DOT<T>,
    ) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.deposit_nominated_collateral(
            nominator_id.clone(),
            collateral,
            backing_collateral,
        )?;
        Self::deposit_event(Event::<T>::IncreaseNominatedCollateral(
            nominator_id.clone(),
            operator_id.clone(),
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
        Self::deposit_event(Event::<T>::WithdrawNominatedCollateral(
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
        let mut operator = Self::get_rich_operator_from_id(&vault_id.clone())?;
        operator.slash_nominators(status, to_slash, backing_collateral_before_slashing)?;
        <Operators<T>>::insert(operator.id(), operator.data.clone());
        Ok(())
    }

    pub fn opt_in_to_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            !<Operators<T>>::contains_key(operator_id),
            Error::<T>::VaultAlreadyOptedInToNomination
        );
        let operator: RichOperator<T> = Operator::new(operator_id.clone()).into();
        <Operators<T>>::insert(operator_id, operator.data.clone());
        Ok(())
    }

    pub fn opt_out_of_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            <Operators<T>>::contains_key(operator_id),
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;

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
        Ok(<Operators<T>>::contains_key(&operator_id))
    }

    pub fn get_total_nominated_collateral(
        operator_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.data.total_nominated_collateral)
    }

    pub fn get_operator_from_id(
        operator_id: &T::AccountId,
    ) -> Result<DefaultOperator<T>, Error<T>> {
        Ok(<Operators<T>>::get(operator_id))
    }

    fn get_rich_operator_from_id(
        operator_id: &T::AccountId,
    ) -> Result<RichOperator<T>, DispatchError> {
        Ok(Self::get_operator_from_id(operator_id)?.into())
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
        OperatorNotFound,
        WithdrawRequestNotFound,
        WithdrawRequestNotMatured,
        InsufficientCollateral,
        FailedToAddNominator
    }
}
