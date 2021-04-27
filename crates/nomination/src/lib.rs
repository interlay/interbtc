//! # PolkaBTC Nomination Module

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

use sp_std::{convert::TryInto, vec::Vec};

use codec::{Decode, Encode, EncodeLike};

use ext::vault_registry::VaultStatus;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use primitive_types::H256;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::{CheckedAdd, Zero};
use types::{DefaultOperator, RichOperator, UnsignedFixedPoint, DOT};
pub use types::{Nominator, Operator};
use vault_registry::LiquidationTarget;

pub trait WeightInfo {
    fn set_nomination_enabled() -> Weight;
    fn opt_in_to_nomination() -> Weight;
    fn opt_out_of_nomination() -> Weight;
    fn deposit_nominated_collateral() -> Weight;
    fn request_collateral_withdrawal() -> Weight;
    fn execute_collateral_withdrawal() -> Weight;
    // fn cancel_collateral_withdrawal() -> Weight;
}

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config + collateral::Config + treasury::Config + security::Config + vault_registry::Config + fee::Config
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
        NominationEnabled get(fn is_nomination_enabled) config(): bool;

        /// Upper limit, expressed as a rate out of a Vault's collateral, that can be
        /// nominated as collateral
        MaxNominationRatio get(fn get_max_nomination_ratio) config(): UnsignedFixedPoint<T>;

        /// Maximum number of nominators a single operator can have
        MaxNominatorsPerOperator get(fn get_max_nominators_per_operator) config(): u16;

        /// Base unbonding period by which collateral withdrawal requests from Vault Operators
        /// are delayed
        OperatorUnbondingPeriod get(fn get_operator_unbonding_period) config(): T::BlockNumber;

        /// Base unbonding period by which collateral withdrawal requests from Vault Nominators
        /// are delayed
        NominatorUnbondingPeriod get(fn get_nominator_unbonding_period) config(): T::BlockNumber;

        /// Map of Vault Operators
        Operators: map hasher(blake2_128_concat) T::AccountId => Operator<T::AccountId, T::BlockNumber, DOT<T>>;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        BlockNumber = <T as frame_system::Config>::BlockNumber,
        DOT = DOT<T>,
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
        // [operator_id, collateral]
        ExecuteOperatorCollateralWithdrawal(AccountId, DOT),
        // [request_id, operator_id, collateral]
        CancelOperatorCollateralWithdrawal(H256, AccountId, DOT),
        // [request_id, nominator_id, operator_id, maturity_block, collateral]
        RequestNominatorCollateralWithdrawal(H256, AccountId, AccountId, BlockNumber, DOT),
        // [nominator_id, operator_id, collateral]
        ExecuteNominatorCollateralWithdrawal(AccountId, AccountId, DOT),
        // [request_id, nominator_id, operator_id, collateral]
        CancelNominatorCollateralWithdrawal(H256, AccountId, AccountId, DOT),
        // [operator_id, collateral, status]
        SlashCollateral(AccountId, DOT, VaultStatus),
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
        fn deposit_nominated_collateral(origin, operator_id: T::AccountId, amount: DOT<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_deposit_nominated_collateral(&sender, &operator_id, amount)
        }

        #[weight = <T as Config>::WeightInfo::request_collateral_withdrawal()]
        #[transactional]
        fn request_collateral_withdrawal(origin, operator_id: T::AccountId, amount: DOT<T>) -> DispatchResult {
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

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }
    }
}

impl<T: Config> Module<T> {
    fn begin_block(_height: T::BlockNumber) -> DispatchResult {
        let (_, liquidated_operator_amounts) =
            ext::vault_registry::liquidate_undercollateralized_vaults::<T>(LiquidationTarget::OperatorsOnly);
        for (operator_id, total_slashed_amount) in liquidated_operator_amounts {
            Self::slash_nominators(operator_id.clone(), VaultStatus::Liquidated, total_slashed_amount)?;
        }
        Ok(())
    }

    pub fn set_max_nomination_ratio(limit: UnsignedFixedPoint<T>) -> DispatchResult {
        <MaxNominationRatio<T>>::set(limit);
        Ok(())
    }

    pub fn set_max_nominators_per_operator(limit: u16) -> DispatchResult {
        <MaxNominatorsPerOperator>::set(limit);
        Ok(())
    }

    pub fn set_operator_unbonding_period(period: T::BlockNumber) -> DispatchResult {
        <OperatorUnbondingPeriod<T>>::set(period);
        Ok(())
    }

    pub fn set_nominator_unbonding_period(period: T::BlockNumber) -> DispatchResult {
        <NominatorUnbondingPeriod<T>>::set(period);
        Ok(())
    }

    /// Liquidates a vault, transferring all of its token balances to the `LiquidationVault`.
    /// Delegates to `liquidate_vault_with_status`, using `Liquidated` status
    pub fn liquidate_operator(vault_id: &T::AccountId) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        Self::liquidate_operator_with_status(vault_id, VaultStatus::Liquidated)
    }

    /// Liquidates a vault, transferring all of its token balances to the
    /// `LiquidationVault`, as well as the DOT collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to liquidate
    /// * `status` - status with which to liquidate the vault
    ///
    /// # Errors
    /// * `VaultNotFound` - if the vault to liquidate does not exist
    pub fn liquidate_operator_with_status(operator_id: &T::AccountId, status: VaultStatus) -> DispatchResult {
        let slashed_amount = ext::vault_registry::liquidate_vault_with_status::<T>(operator_id, status)?;
        Self::deposit_event(Event::<T>::SlashCollateral(operator_id.clone(), slashed_amount, status));
        Self::slash_nominators(operator_id.clone(), status, slashed_amount)
    }

    /// Unbond collateral withdrawal if mature.
    ///
    /// # Arguments
    ///
    /// * `withdrawer_id` - AccountId of the withdrawer
    /// * `vault_id` - AccountId of the vault
    /// * `amount` - DOt amount to withdraw
    /// * `height` - current block height
    /// * `maturity` - height at request time + unbonding period
    fn _execute_collateral_withdrawal(withdrawer_id: &T::AccountId, operator_id: &T::AccountId) -> DispatchResult {
        if withdrawer_id.eq(operator_id) {
            Self::execute_operator_withdrawal(operator_id)
        } else {
            Self::execute_nominator_withdrawal(operator_id, withdrawer_id)
        }
    }

    fn _request_collateral_withdrawal(
        withdrawer_id: &T::AccountId,
        operator_id: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        if withdrawer_id.eq(operator_id) {
            Self::request_operator_withdrawal(operator_id, amount)?
        } else {
            Self::request_nominator_withdrawal(operator_id, withdrawer_id, amount)?
        };
        ext::vault_registry::decrease_backing_collateral::<T>(operator_id, amount)
    }

    pub fn request_operator_withdrawal(operator_id: &T::AccountId, collateral_to_withdraw: DOT<T>) -> DispatchResult {
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

    pub fn request_nominator_withdrawal(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        collateral_to_withdraw: DOT<T>,
    ) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        let request_id = ext::security::get_secure_id::<T>(operator_id);
        let height = ext::security::active_block_number::<T>();
        let maturity = height
            .checked_add(&Self::get_nominator_unbonding_period())
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        operator.add_pending_nominator_withdrawal(
            nominator_id.clone(),
            request_id,
            collateral_to_withdraw,
            maturity,
        )?;
        Self::deposit_event(Event::<T>::RequestNominatorCollateralWithdrawal(
            request_id,
            nominator_id.clone(),
            operator_id.clone(),
            maturity,
            collateral_to_withdraw,
        ));
        Ok(())
    }

    pub fn execute_nominator_withdrawal(operator_id: &T::AccountId, nominator_id: &T::AccountId) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        let matured_collateral = operator.execute_nominator_withdrawal(nominator_id.clone())?;
        ensure!(!matured_collateral.is_zero(), Error::<T>::NoMaturedCollateral);
        Self::deposit_event(Event::<T>::ExecuteNominatorCollateralWithdrawal(
            nominator_id.clone(),
            operator_id.clone(),
            matured_collateral,
        ));
        Ok(())
    }

    pub fn _deposit_nominated_collateral(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            Self::is_operator(&operator_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.deposit_nominated_collateral(nominator_id.clone(), collateral)?;
        ext::vault_registry::lock_additional_collateral_from_address::<T>(&operator_id, collateral, &nominator_id)?;
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
        total_slashed_amount: DOT<T>,
    ) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(&vault_id)?;
        operator.slash_nominators(status, total_slashed_amount)?;
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
        ext::vault_registry::set_is_nomination_operator::<T>(operator_id, true);
        Self::deposit_event(Event::<T>::NominationOptIn(operator_id.clone()));
        Ok(())
    }

    pub fn _opt_out_of_nomination(operator_id: &T::AccountId) -> DispatchResult {
        let mut operator = Self::get_rich_operator_from_id(operator_id)?;
        operator.force_refund_nominated_collateral()?;
        <Operators<T>>::remove(operator_id);
        ext::vault_registry::set_is_nomination_operator::<T>(operator_id, false);
        Self::deposit_event(Event::<T>::NominationOptOut(operator_id.clone()));
        Ok(())
    }

    pub fn is_operator(operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Operators<T>>::contains_key(&operator_id))
    }

    pub fn get_total_nominated_collateral(operator_id: &T::AccountId) -> Result<DOT<T>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.data.total_nominated_collateral)
    }

    pub fn get_collateral_to_be_withdrawn(operator_id: &T::AccountId) -> Result<DOT<T>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.data.collateral_to_be_withdrawn)
    }

    pub fn get_nominators(
        operator_id: &T::AccountId,
    ) -> Result<Vec<(T::AccountId, Nominator<T::AccountId, T::BlockNumber, DOT<T>>)>, DispatchError> {
        let operator = Self::get_rich_operator_from_id(operator_id)?;
        Ok(operator.get_nominators())
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

    fn dot_to_u128(x: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
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
        TryIntoIntError,
        NotAVault,
        WithdrawRequestNotFound,
        WithdrawRequestNotMatured,
        InsufficientCollateral,
        FailedToAddNominator,
        VaultNominationDisabled,
        DepositViolatesMaxNominationRatio,
        NoMaturedCollateral,
        OperatorHasTooManyNominators
    }
}
