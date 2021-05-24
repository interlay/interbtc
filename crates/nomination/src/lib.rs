//! # Nomination Module

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod ext;
mod types;

mod default_weights;

use codec::{Decode, Encode, EncodeLike};

use ext::vault_registry::{DefaultVault, SlashingError, TryDepositCollateral, TryWithdrawCollateral, VaultStatus};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedSub, One, Zero};
use types::{
    Collateral, DefaultNominator, DefaultOperator, RichNominator, RichOperator, SignedFixedPoint, UnsignedFixedPoint,
};
pub use types::{Nominator, Operator};

pub trait WeightInfo {
    fn set_nomination_enabled() -> Weight;
    fn opt_in_to_nomination() -> Weight;
    fn opt_out_of_nomination() -> Weight;
    fn deposit_nominated_collateral() -> Weight;
    fn request_collateral_withdrawal() -> Weight;
    fn execute_collateral_withdrawal() -> Weight;
    fn cancel_collateral_withdrawal() -> Weight;
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + currency::Config<currency::Collateral>
    + currency::Config<currency::Wrapped>
    + security::Config
    + vault_registry::Config<
        UnsignedFixedPoint = <Self as fee::Config>::UnsignedFixedPoint,
        SignedFixedPoint = <Self as Config>::SignedFixedPoint,
    > + fee::Config
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    type SignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Nomination {
        /// ## Storage

        /// Flag indicating whether this feature is enabled
        NominationEnabled get(fn is_nomination_enabled) config(): bool;

        /// Base unbonding period by which collateral withdrawal requests from Vault Operators
        /// are delayed
        OperatorUnbondingPeriod get(fn get_operator_unbonding_period) config(): T::BlockNumber;

        /// Base unbonding period by which collateral withdrawal requests from Vault Nominators
        /// are delayed
        NominatorUnbondingPeriod get(fn get_nominator_unbonding_period) config(): T::BlockNumber;

        /// Map of Vault Operators
        Operators: map hasher(blake2_128_concat) T::AccountId => Operator<T::AccountId, T::BlockNumber, Collateral<T>>;

        /// Map of Nominators
        Nominators: map hasher(blake2_128_concat) (T::AccountId, T::AccountId) => Nominator<T::AccountId, T::BlockNumber, Collateral<T>, SignedFixedPoint<T>>;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        BlockNumber = <T as frame_system::Config>::BlockNumber,
        Collateral = Collateral<T>,
    {
        // [operator_id]
        NominationOptIn(AccountId),
        // [operator_id]
        NominationOptOut(AccountId),
        // [nominator_id, operator_id, collateral]
        IncreaseNominatedCollateral(AccountId, AccountId, Collateral),
        // [nominator_id, operator_id, collateral]
        WithdrawNominatedCollateral(AccountId, AccountId, Collateral),
        // [request_id, operator_id, maturity_block, collateral]
        RequestOperatorCollateralWithdrawal(H256, AccountId, BlockNumber, Collateral),
        // [operator_id, collateral]
        ExecuteOperatorCollateralWithdrawal(AccountId, Collateral),
        // [request_id, operator_id]
        CancelOperatorCollateralWithdrawal(H256, AccountId),
        // [request_id, nominator_id, operator_id, maturity_block, collateral]
        RequestNominatorCollateralWithdrawal(H256, AccountId, AccountId, BlockNumber, Collateral),
        // [nominator_id, operator_id, collateral]
        ExecuteNominatorCollateralWithdrawal(AccountId, AccountId, Collateral),
        // [request_id, nominator_id, operator_id]
        CancelNominatorCollateralWithdrawal(H256, AccountId, AccountId),
        // [operator_id, collateral, status]
        SlashCollateral(AccountId, Collateral, VaultStatus),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        fn deposit_event() = default;

        #[weight = <T as Config>::WeightInfo::set_nomination_enabled()]
        #[transactional]
        fn set_nomination_enabled(origin, enabled: bool) {
            ensure_root(origin)?;
            <NominationEnabled>::set(enabled);
        }

        /// Become an Operator in the Vault Nomination protocol
        #[weight = <T as Config>::WeightInfo::opt_in_to_nomination()]
        #[transactional]
        fn opt_in_to_nomination(origin) -> DispatchResult {
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_opt_in_to_nomination(&ensure_signed(origin)?)
        }

        /// Deregister from being Operator in the Vault Nomination protocol
        #[weight = <T as Config>::WeightInfo::opt_out_of_nomination()]
        #[transactional]
        fn opt_out_of_nomination(origin) -> DispatchResult {
            Self::_opt_out_of_nomination(&ensure_signed(origin)?)
        }

        #[weight = <T as Config>::WeightInfo::deposit_nominated_collateral()]
        #[transactional]
        fn deposit_nominated_collateral(origin, operator_id: T::AccountId, amount: Collateral<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_deposit_nominated_collateral(&sender, &operator_id, amount)
        }

        #[weight = <T as Config>::WeightInfo::request_collateral_withdrawal()]
        #[transactional]
        fn request_collateral_withdrawal(origin, operator_id: T::AccountId, amount: Collateral<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_request_collateral_withdrawal(&sender, &operator_id, amount)
        }

        #[weight = <T as Config>::WeightInfo::execute_collateral_withdrawal()]
        #[transactional]
        fn execute_collateral_withdrawal(origin, operator_id: T::AccountId) -> DispatchResult {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_execute_collateral_withdrawal(&account_id, &operator_id)
        }

        #[weight = <T as Config>::WeightInfo::execute_collateral_withdrawal()]
        #[transactional]
        fn cancel_collateral_withdrawal(origin, operator_id: T::AccountId, request_id: H256) -> DispatchResult {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_cancel_collateral_withdrawal(&account_id, &operator_id, &request_id)
        }
    }
}

impl<T: Config> Module<T> {
    pub fn set_operator_unbonding_period(period: T::BlockNumber) -> DispatchResult {
        <OperatorUnbondingPeriod<T>>::set(period);
        Ok(())
    }

    pub fn set_nominator_unbonding_period(period: T::BlockNumber) -> DispatchResult {
        <NominatorUnbondingPeriod<T>>::set(period);
        Ok(())
    }

    pub fn _request_collateral_withdrawal(
        withdrawer_id: &T::AccountId,
        operator_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(
            ext::vault_registry::is_allowed_to_withdraw_collateral::<T>(operator_id, amount)?,
            Error::<T>::InsufficientCollateral
        );
        if withdrawer_id.eq(operator_id) {
            Self::request_operator_withdrawal(operator_id, amount)?
        } else {
            Self::request_nominator_withdrawal(operator_id, withdrawer_id, amount)?
        };
        Ok(())
    }

    /// Unbond collateral withdrawal if mature.
    ///
    /// # Arguments
    ///
    /// * `withdrawer_id` - AccountId of the withdrawer
    /// * `vault_id` - AccountId of the vault
    pub fn _execute_collateral_withdrawal(withdrawer_id: &T::AccountId, operator_id: &T::AccountId) -> DispatchResult {
        if withdrawer_id.eq(operator_id) {
            Self::execute_operator_withdrawal(operator_id)
        } else {
            // Self::execute_nominator_withdrawal(operator_id, withdrawer_id)
            Ok(())
        }
    }

    pub fn _cancel_collateral_withdrawal(
        withdrawer_id: &T::AccountId,
        operator_id: &T::AccountId,
        request_id: &H256,
    ) -> DispatchResult {
        if withdrawer_id.eq(operator_id) {
            Self::cancel_operator_withdrawal(operator_id, request_id)
        } else {
            // Self::cancel_nominator_withdrawal(operator_id, withdrawer_id, request_id)
            Ok(())
        }
    }

    pub fn request_operator_withdrawal(
        operator_id: &T::AccountId,
        collateral_to_withdraw: Collateral<T>,
    ) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        let request_id = ext::security::get_secure_id::<T>(operator_id);
        let height = ext::security::active_block_number::<T>();
        let maturity = height + Self::get_operator_unbonding_period();
        operator.add_pending_operator_withdrawal(request_id, collateral_to_withdraw, maturity)?;
        Self::deposit_event(Event::<T>::RequestOperatorCollateralWithdrawal(
            request_id,
            operator_id.clone(),
            maturity,
            collateral_to_withdraw,
        ));
        Ok(())
    }

    pub fn execute_operator_withdrawal(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        // For every matured request, an event is emitted inside the object method
        let matured_collateral = operator.execute_operator_withdrawal()?;
        ensure!(!matured_collateral.is_zero(), Error::<T>::NoMaturedCollateral);
        Self::deposit_event(Event::<T>::ExecuteOperatorCollateralWithdrawal(
            operator_id.clone(),
            matured_collateral,
        ));
        Ok(())
    }

    pub fn cancel_operator_withdrawal(operator_id: &T::AccountId, request_id: &H256) -> DispatchResult {
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.remove_pending_operator_withdrawal(*request_id)?;
        Self::deposit_event(Event::<T>::CancelOperatorCollateralWithdrawal(
            *request_id,
            operator_id.clone(),
        ));
        Ok(())
    }

    pub fn request_nominator_withdrawal(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut nominator: RichNominator<T> = Self::get_nominator(&nominator_id, &operator_id)?.into();
        nominator.try_withdraw_collateral(amount)?;
        ext::collateral::unlock_and_transfer::<T>(nominator_id, operator_id, amount)?;
        Ok(())
    }

    pub fn _deposit_nominated_collateral(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let vault_backing_collateral = ext::vault_registry::get_backing_collateral::<T>(operator_id)?;
        let total_nominated_collateral: Collateral<T> = Self::get_total_nominated_collateral(operator_id)?.into();
        let new_nominated_collateral = total_nominated_collateral
            .checked_add(&amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ensure!(
            new_nominated_collateral <= Self::get_max_nominatable_collateral(vault_backing_collateral)?,
            Error::<T>::DepositViolatesMaxNominationRatio
        );
        let mut nominator: RichNominator<T> = Self::register_or_get_nominator(nominator_id, operator_id)?.into();
        nominator
            .try_deposit_collateral(amount)
            .map_err(|e| Error::<T>::from(e))?;
        ext::collateral::transfer_and_lock::<T>(nominator_id, operator_id, amount)?;

        Self::deposit_event(Event::<T>::IncreaseNominatedCollateral(
            nominator_id.clone(),
            operator_id.clone(),
            amount,
        ));
        Ok(())
    }

    /// Mark Vault as an Operator in the Vault Nomination protocol
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to mark as Nomination Operator
    pub fn _opt_in_to_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            ext::vault_registry::vault_exists::<T>(&operator_id),
            Error::<T>::NotAVault
        );
        ensure!(
            !<Operators<T>>::contains_key(operator_id),
            Error::<T>::VaultAlreadyOptedInToNomination
        );
        let operator = Operator::new(operator_id.clone());
        <Operators<T>>::insert(operator_id, operator.clone());
        Self::deposit_event(Event::<T>::NominationOptIn(operator_id.clone()));
        Ok(())
    }

    pub fn _opt_out_of_nomination(operator_id: &T::AccountId) -> DispatchResult {
        ensure!(
            Self::get_total_nominated_collateral(operator_id)?.is_zero(),
            Error::<T>::HasNominatedCollateral
        );
        <Operators<T>>::remove(operator_id);
        Self::deposit_event(Event::<T>::NominationOptOut(operator_id.clone()));
        Ok(())
    }

    pub fn is_operator(operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Operators<T>>::contains_key(&operator_id))
    }

    pub fn is_nominator(nominator_id: &T::AccountId, operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Nominators<T>>::contains_key((&nominator_id, &operator_id)))
    }

    pub fn get_total_nominated_collateral(operator_id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        let operator: DefaultVault<T> = ext::vault_registry::get_vault_from_id::<T>(operator_id)?;
        let operator_actual_collateral = ext::vault_registry::compute_collateral::<T>(operator_id)?;
        Ok(operator
            .backing_collateral
            .checked_sub(&operator_actual_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_collateral_to_be_withdrawn(operator_id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.data.collateral_to_be_withdrawn)
    }

    pub fn get_max_nomination_ratio() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let secure_collateral_threshold = ext::vault_registry::get_secure_collateral_threshold::<T>();
        let premium_redeem_threshold = ext::vault_registry::get_premium_redeem_threshold::<T>();
        Ok(secure_collateral_threshold
            .checked_div(&premium_redeem_threshold)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_sub(&<T as fee::Config>::UnsignedFixedPoint::one())
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_operator_from_id(operator_id: &T::AccountId) -> Result<DefaultOperator<T>, DispatchError> {
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        Ok(<Operators<T>>::get(operator_id))
    }

    fn get_rich_operator_from_id(operator_id: &T::AccountId) -> Result<RichOperator<T>, DispatchError> {
        Ok(Self::get_operator_from_id(operator_id)?.into())
    }

    pub fn get_nominator(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
    ) -> Result<DefaultNominator<T>, DispatchError> {
        ensure!(
            Self::is_nominator(&nominator_id, &operator_id)?,
            Error::<T>::NominatorNotFound
        );
        Ok(<Nominators<T>>::get((nominator_id, operator_id)))
    }

    pub fn get_rich_nominator(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
    ) -> Result<RichNominator<T>, DispatchError> {
        Ok(Self::get_nominator(&nominator_id, &operator_id)?.into())
    }

    pub fn get_nominator_collateral(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
    ) -> Result<Collateral<T>, DispatchError> {
        let nominator = Self::get_rich_nominator(nominator_id, operator_id)?;
        Ok(nominator.compute_collateral()?)
    }

    pub fn register_or_get_nominator(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
    ) -> Result<DefaultNominator<T>, DispatchError> {
        if !Self::is_nominator(&nominator_id, &operator_id)? {
            let nominator = Nominator::new(nominator_id.clone(), operator_id.clone());
            <Nominators<T>>::insert((nominator_id, operator_id), nominator.clone());
            Ok(nominator)
        } else {
            Ok(<Nominators<T>>::get((&nominator_id, &operator_id)))
        }
    }

    pub fn get_max_nominatable_collateral(operator_collateral: Collateral<T>) -> Result<Collateral<T>, DispatchError> {
        ext::fee::collateral_for::<T>(operator_collateral, Module::<T>::get_max_nomination_ratio()?)
    }
}

impl<T: Config> From<SlashingError> for Error<T> {
    fn from(err: SlashingError) -> Self {
        match err {
            SlashingError::ArithmeticOverflow => Error::<T>::ArithmeticOverflow,
            SlashingError::ArithmeticUnderflow => Error::<T>::ArithmeticUnderflow,
            SlashingError::TryIntoIntError => Error::<T>::TryIntoIntError,
            SlashingError::InsufficientFunds => Error::<T>::InsufficientCollateral,
        }
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
        TooLittleNominatedCollateral,
        VaultAlreadyOptedInToNomination,
        VaultNotOptedInToNomination,
        VaultNotQualifiedToOptOutOfNomination,
        VaultNotFound,
        TryIntoIntError,
        NotAVault,
        WithdrawalRequestNotFound,
        WithdrawalRequestNotMatured,
        InsufficientCollateral,
        FailedToAddNominator,
        VaultNominationDisabled,
        DepositViolatesMaxNominationRatio,
        NoMaturedCollateral,
        HasNominatedCollateral,
    }
}
