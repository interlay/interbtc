use crate::{ext, Config, Error, Module};
use codec::{Decode, Encode, HasCompact};
use frame_support::{dispatch::DispatchResult, ensure, traits::Currency, StorageMap};
use sp_core::H256;
use sp_runtime::{
    traits::{CheckedAdd, CheckedSub, Saturating, Zero},
    DispatchError,
};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

#[cfg(test)]
use mocktopus::macros::mockable;
use vault_registry::VaultStatus;

pub(crate) type DOT<T> = <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub(crate) type UnsignedFixedPoint<T> = <T as fee::Config>::UnsignedFixedPoint;

pub struct RichOperator<T: Config> {
    pub(crate) data: DefaultOperator<T>,
}

pub type DefaultOperator<T> =
    Operator<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::BlockNumber, DOT<T>>;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct Nominator<AccountId: Ord, BlockNumber, DOT> {
    pub id: AccountId,
    pub collateral: DOT,
    /// Map of request_id => (Maturity Block, DOT to withdraw)
    pub pending_withdrawals: BTreeMap<H256, (BlockNumber, DOT)>,
    pub collateral_to_be_withdrawn: DOT,
}

impl<AccountId: Ord, BlockNumber, DOT: HasCompact + Default> Nominator<AccountId, BlockNumber, DOT> {
    pub(crate) fn new(id: AccountId) -> Nominator<AccountId, BlockNumber, DOT> {
        Nominator {
            id,
            collateral: Default::default(),
            pending_withdrawals: Default::default(),
            collateral_to_be_withdrawn: Default::default(),
        }
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct Operator<AccountId: Ord, BlockNumber, DOT> {
    // Account identifier of the Vault
    pub id: AccountId,
    pub nominators: BTreeMap<AccountId, Nominator<AccountId, BlockNumber, DOT>>,
    pub total_nominated_collateral: DOT,
    /// Map of request_id => (Maturity Block, DOT to withdraw)
    pub pending_withdrawals: BTreeMap<H256, (BlockNumber, DOT)>,
    pub collateral_to_be_withdrawn: DOT,
}

impl<AccountId: Ord, BlockNumber, DOT: HasCompact + Default> Operator<AccountId, BlockNumber, DOT> {
    pub(crate) fn new(id: AccountId) -> Operator<AccountId, BlockNumber, DOT> {
        Operator {
            id,
            nominators: Default::default(),
            total_nominated_collateral: Default::default(),
            pending_withdrawals: Default::default(),
            collateral_to_be_withdrawn: Default::default(),
        }
    }
}

impl<T: Config> From<&RichOperator<T>> for DefaultOperator<T> {
    fn from(rv: &RichOperator<T>) -> DefaultOperator<T> {
        rv.data.clone()
    }
}

impl<T: Config> From<DefaultOperator<T>> for RichOperator<T> {
    fn from(vault: DefaultOperator<T>) -> RichOperator<T> {
        RichOperator { data: vault }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichOperator<T> {
    pub fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    pub fn get_nominators(&self) -> Vec<(T::AccountId, Nominator<T::AccountId, T::BlockNumber, DOT<T>>)> {
        self.data
            .nominators
            .iter()
            .map(|(id, nominator)| (id.clone(), nominator.clone()))
            .collect::<Vec<(_, _)>>()
    }

    pub fn force_refund_nominated_collateral(&mut self) -> DispatchResult {
        self.force_refund_nominators_proportionally(self.data.total_nominated_collateral)?;
        Ok(())
    }

    pub fn force_refund_nominators_proportionally(&mut self, amount: DOT<T>) -> DispatchResult {
        let data_clone = self.data.clone();
        for nominator_id in data_clone.nominators.keys() {
            let nominator_collateral_to_refund = self.scale_amount_by_nominator_proportion(amount, nominator_id)?;
            self.withdraw_nominated_collateral(nominator_id.clone(), nominator_collateral_to_refund)?;
        }
        Ok(())
    }

    pub fn deposit_nominated_collateral(&mut self, nominator_id: T::AccountId, collateral: DOT<T>) -> DispatchResult {
        let new_nominated_collateral = self
            .data
            .total_nominated_collateral
            .checked_add(&(collateral.clone()))
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ensure!(
            new_nominated_collateral <= self.get_max_nominatable_collateral(self.get_operator_collateral()?)?,
            Error::<T>::DepositViolatesMaxNominationRatio
        );
        // If the depositor is not in the `nominators` map, add them.
        if !self.is_nominator(&nominator_id)? {
            ensure!(
                self.data.nominators.len() < Module::<T>::get_max_nominators_per_operator().into(),
                Error::<T>::OperatorHasTooManyNominators
            );
            self.update(|v| {
                v.nominators
                    .insert(nominator_id.clone(), Nominator::new(nominator_id.clone()).clone());
                Ok(())
            })?;
        };
        self.increase_nominator_collateral(nominator_id, collateral)
    }

    pub fn withdraw_nominated_collateral(&mut self, nominator_id: T::AccountId, collateral: DOT<T>) -> DispatchResult {
        let nominator = self
            .data
            .nominators
            .get_mut(&nominator_id)
            .ok_or(Error::<T>::NominatorNotFound)?;
        ensure!(
            collateral <= nominator.collateral,
            Error::<T>::TooLittleNominatedCollateral
        );
        self.decrease_nominator_collateral(nominator_id.clone(), collateral)?;
        ext::vault_registry::withdraw_collateral_to_address::<T>(&self.id(), collateral, &nominator_id)
    }

    fn increase_nominator_collateral(&mut self, nominator_id: T::AccountId, increase_by: DOT<T>) -> DispatchResult {
        self.update(|v| {
            // Increase the sum of nominated collateral for this operator
            v.total_nominated_collateral = v
                .total_nominated_collateral
                .checked_add(&increase_by.clone())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let mut nominator = v
                .nominators
                .get_mut(&nominator_id)
                .ok_or(Error::<T>::NominatorNotFound)?
                .clone();
            nominator.collateral = nominator
                .collateral
                .checked_add(&increase_by)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            v.nominators.insert(nominator_id.clone(), nominator);
            Ok(())
        })
    }

    fn decrease_nominator_collateral(&mut self, nominator_id: T::AccountId, decrease_by: DOT<T>) -> DispatchResult {
        self.update(|v| {
            // Decrease the sum of nominated collateral for this operator
            v.total_nominated_collateral = v
                .total_nominated_collateral
                .checked_sub(&decrease_by.clone())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let mut nominator = v
                .nominators
                .get_mut(&nominator_id)
                .ok_or(Error::<T>::NominatorNotFound)?
                .clone();
            if nominator.collateral.eq(&decrease_by) {
                v.nominators.remove(&nominator_id);
            } else {
                nominator.collateral = nominator
                    .collateral
                    .checked_sub(&decrease_by)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                v.nominators.insert(nominator_id.clone(), nominator);
            }
            Ok(())
        })
    }

    pub fn scale_amount_by_nominator_proportion(
        &self,
        amount: DOT<T>,
        nominator_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        let amount_u128 = Module::<T>::dot_to_u128(amount)?;
        let nominator = self
            .data
            .nominators
            .get(&nominator_id)
            .ok_or(Error::<T>::NominatorNotFound)?;
        let nominated_collateral_u128 = Module::<T>::dot_to_u128(nominator.collateral)?;
        let total_nominated_collateral_u128 = Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;

        // Multiply the amount by the Nominator's collateral proportion
        let nominated_collateral_to_slash_u128 = amount_u128
            .checked_mul(nominated_collateral_u128)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_div(total_nominated_collateral_u128)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        Module::<T>::u128_to_dot(nominated_collateral_to_slash_u128)
    }

    // Operator functionality

    pub fn add_pending_operator_withdrawal(
        &mut self,
        request_id: H256,
        collateral_to_withdraw: DOT<T>,
        maturity: T::BlockNumber,
    ) -> DispatchResult {
        // If `collateral_to_withdraw` is larger than (operator_collateral - collateral_to_be_withdrawn),
        // the following throws an error.
        let operator_collateral = self.get_operator_collateral()?;
        let remaining_operator_collateral = operator_collateral
            .checked_sub(&self.data.collateral_to_be_withdrawn)
            .ok_or(Error::<T>::InsufficientCollateral)?
            .checked_sub(&collateral_to_withdraw)
            .ok_or(Error::<T>::InsufficientCollateral)?;

        // Trigger forced refunds if remaining nominated collateral would
        // exceed the Max Nomination Ratio.
        let max_nominatable_collateral = self.get_max_nominatable_collateral(remaining_operator_collateral)?;
        let nominated_collateral_to_force_refund = self
            .data
            .total_nominated_collateral
            .saturating_sub(max_nominatable_collateral);
        if !nominated_collateral_to_force_refund.is_zero() {
            // The Nominators are not assumed to be trusted by the Operator.
            // Unless the refund is forced (not subject to unbonding), a Nominator might cancel the
            // refund request and cause the Max Nomination Ratio to be exceeded.
            // As such, we need to force refund.
            self.force_refund_nominators_proportionally(nominated_collateral_to_force_refund)?;
        }

        // Add withdrawal request and increase collateral to-be-withdrawn
        self.update(|v| {
            v.pending_withdrawals
                .insert(request_id, (maturity, collateral_to_withdraw));
            v.collateral_to_be_withdrawn = v
                .collateral_to_be_withdrawn
                .checked_add(&collateral_to_withdraw)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    pub fn execute_operator_withdrawal(&mut self) -> Result<DOT<T>, DispatchError> {
        let matured_operator_withdrawal_requests =
            Self::get_matured_withdrawal_requests(&self.data.pending_withdrawals);
        let mut matured_collateral_to_withdraw: DOT<T> = 0u32.into();
        for (request_id, (_, amount)) in matured_operator_withdrawal_requests.iter() {
            self.remove_pending_operator_withdrawal(*request_id)?;
            matured_collateral_to_withdraw = matured_collateral_to_withdraw
                .checked_add(amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
        }
        // The backing collateral was decreased when the withdrawal requests were made,
        // to prevent issuing with the to-be-withdrawn collateral.
        // Now, increase the backing collateral back, so it can be withdrawn using the
        // standard function from the vault_registry.
        ext::vault_registry::increase_backing_collateral::<T>(&self.id(), matured_collateral_to_withdraw)?;
        ext::vault_registry::withdraw_collateral_to_address::<T>(
            &self.id(),
            matured_collateral_to_withdraw,
            &self.id(),
        )?;
        Ok(matured_collateral_to_withdraw)
    }

    pub fn get_matured_withdrawal_requests(
        requests: &BTreeMap<H256, (T::BlockNumber, DOT<T>)>,
    ) -> Vec<(H256, (T::BlockNumber, DOT<T>))> {
        let current_height = ext::security::active_block_number::<T>();
        requests
            .clone()
            .iter()
            .filter(|(_, (maturity, _))| current_height >= *maturity)
            .map(|(id, withdrawal_request)| (id.clone(), withdrawal_request.clone()))
            .collect::<Vec<(H256, (T::BlockNumber, DOT<T>))>>()
    }

    pub fn remove_pending_operator_withdrawal(&mut self, request_id: H256) -> DispatchResult {
        let withdrawal = *self
            .data
            .pending_withdrawals
            .get_mut(&request_id)
            .ok_or(Error::<T>::WithdrawRequestNotFound)?;
        self.update(|v| {
            v.collateral_to_be_withdrawn = v
                .collateral_to_be_withdrawn
                .checked_sub(&withdrawal.1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            v.pending_withdrawals.remove(&request_id);
            Ok(())
        })
    }

    // Nominator functionality

    pub fn add_pending_nominator_withdrawal(
        &mut self,
        nominator_id: T::AccountId,
        request_id: H256,
        collateral_to_withdraw: DOT<T>,
        maturity: T::BlockNumber,
    ) -> DispatchResult {
        // Ensure that the Nominator has enough collateral for the withdrawal
        let nominator = self
            .data
            .nominators
            .get_mut(&nominator_id)
            .ok_or(Error::<T>::NominatorNotFound)?;
        let withdrawable_collateral = nominator
            .collateral
            .checked_sub(&nominator.collateral_to_be_withdrawn)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        ensure!(
            collateral_to_withdraw <= withdrawable_collateral,
            Error::<T>::TooLittleNominatedCollateral
        );

        // Add withdrawal request and increase collateral to-be-withdrawn
        self.update(|v| {
            let mut nominator = v
                .nominators
                .get_mut(&nominator_id)
                .ok_or(Error::<T>::NominatorNotFound)?
                .clone();
            nominator
                .pending_withdrawals
                .insert(request_id, (maturity, collateral_to_withdraw));
            nominator.collateral_to_be_withdrawn = nominator
                .collateral_to_be_withdrawn
                .checked_add(&collateral_to_withdraw)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            v.nominators.insert(nominator_id.clone(), nominator);
            Ok(())
        })
    }

    pub fn execute_nominator_withdrawal(&mut self, nominator_id: T::AccountId) -> Result<DOT<T>, DispatchError> {
        let nominators = self.data.nominators.clone();
        let nominator = nominators
            .get(&nominator_id)
            .ok_or(Error::<T>::NominatorNotFound)?
            .clone();
        let matured_nominator_withdrawal_requests =
            Self::get_matured_withdrawal_requests(&nominator.pending_withdrawals);
        let mut matured_collateral_to_withdraw: DOT<T> = 0u32.into();
        for withdrawal in matured_nominator_withdrawal_requests.iter() {
            self.remove_pending_nominator_withdrawal(nominator_id.clone(), withdrawal.0)?;
            matured_collateral_to_withdraw = matured_collateral_to_withdraw
                .checked_add(&withdrawal.1 .1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
        }
        // The backing collateral was decreased when the withdrawal requests were made,
        // to prevent issuing with the to-be-withdrawn collateral.
        // Now, increase the backing collateral back, so it can be withdrawn using the
        // standard function from the vault_registry.
        ext::vault_registry::increase_backing_collateral::<T>(&self.id(), matured_collateral_to_withdraw)?;
        ext::vault_registry::withdraw_collateral_to_address::<T>(
            &self.id(),
            matured_collateral_to_withdraw,
            &nominator_id,
        )?;
        Ok(matured_collateral_to_withdraw)
    }

    pub fn remove_pending_nominator_withdrawal(
        &mut self,
        nominator_id: T::AccountId,
        request_id: H256,
    ) -> DispatchResult {
        self.update(|v| {
            let mut nominator = v
                .nominators
                .get_mut(&nominator_id)
                .ok_or(Error::<T>::NominatorNotFound)?
                .clone();
            let withdrawal = nominator
                .pending_withdrawals
                .get_mut(&request_id)
                .ok_or(Error::<T>::WithdrawRequestNotFound)?;
            nominator.collateral_to_be_withdrawn = nominator
                .collateral_to_be_withdrawn
                .checked_sub(&withdrawal.1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            nominator.pending_withdrawals.remove(&request_id);
            v.nominators.insert(nominator_id.clone(), nominator);
            Ok(())
        })
    }

    pub fn is_nominator(&self, nominator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(self.data.nominators.contains_key(&nominator_id))
    }

    pub fn slash_nominators(&mut self, status: VaultStatus, total_slashed_amount: DOT<T>) -> DispatchResult {
        let nominated_collateral_to_slash = self.get_nominated_collateral_to_slash(total_slashed_amount, status)?;
        // Slash nominators proportionally
        let vault_clone = self.data.clone();
        for (nominator_id, _) in &vault_clone.nominators {
            let nominated_collateral_to_slash =
                self.scale_amount_by_nominator_proportion(nominated_collateral_to_slash, nominator_id)?;
            self.decrease_nominator_collateral(nominator_id.clone(), nominated_collateral_to_slash)?;
        }

        if status.eq(&VaultStatus::CommittedTheft) {
            // Refund any leftover nominated collateral.
            // Otherwise, there is a risk of exceeding the Max Nomination Ratio
            // after the theft liquidation.
            self.force_refund_nominators_proportionally(self.data.total_nominated_collateral)?;
        }
        Ok(())
    }

    pub fn get_nominated_collateral_to_slash(
        &self,
        total_slashed_amount: DOT<T>,
        status: VaultStatus,
    ) -> Result<DOT<T>, DispatchError> {
        let nominated_collateral_to_slash: DOT<T> = if status.eq(&VaultStatus::CommittedTheft) {
            let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
            // If, after the liquidation, the vault backing collateral
            // is smaller than the total_nominated_collateral (since it wasn't yet updated),
            // it means nominators need to be liquidated too.
            self.data.total_nominated_collateral.saturating_sub(backing_collateral)
        } else {
            let total_slahed_amount_u128 = Module::<T>::dot_to_u128(total_slashed_amount)?;
            let total_nominated_collateral_u128 = Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
            let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
            let backing_collateral_before_slashing = backing_collateral
                .checked_add(&total_slashed_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let backing_collateral_before_slashing_u128 = Module::<T>::dot_to_u128(backing_collateral_before_slashing)?;
            let nominated_collateral_to_slash_u128 = total_slahed_amount_u128
                .checked_mul(total_nominated_collateral_u128)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(backing_collateral_before_slashing_u128)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Module::<T>::u128_to_dot(nominated_collateral_to_slash_u128)?
        };
        Ok(nominated_collateral_to_slash)
    }

    pub fn get_operator_collateral(&self) -> Result<DOT<T>, DispatchError> {
        let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
        Ok(backing_collateral
            .checked_sub(&self.data.total_nominated_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_max_nominatable_collateral(&self, operator_collateral: DOT<T>) -> Result<DOT<T>, DispatchError> {
        ext::fee::dot_for::<T>(operator_collateral, Module::<T>::get_max_nomination_ratio())
    }

    pub fn get_nomination_ratio(&self) -> Result<u128, DispatchError> {
        let operator_collateral = self.get_operator_collateral()?;
        let operator_collateral_u128 = Module::<T>::dot_to_u128(operator_collateral)?;
        let total_nominated_collateral_u128 = Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
        Ok(total_nominated_collateral_u128
            .checked_div(operator_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    fn update<F>(&mut self, func: F) -> DispatchResult
    where
        F: Fn(&mut DefaultOperator<T>) -> DispatchResult,
    {
        func(&mut self.data)?;
        <crate::Operators<T>>::mutate(&self.data.id, func)?;
        Ok(())
    }
}
