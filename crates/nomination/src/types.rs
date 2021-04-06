use crate::{ext, Config, Error, Module};
use codec::{Decode, Encode, HasCompact};
use frame_support::traits::BalanceStatus;
use frame_support::{dispatch::DispatchResult, ensure, traits::Currency, StorageMap};
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedSub, Saturating};
use sp_runtime::DispatchError;
use sp_std::collections::btree_map::BTreeMap;

#[cfg(test)]
use mocktopus::macros::mockable;
use vault_registry::VaultStatus;

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub(crate) type UnsignedFixedPoint<T> = <T as fee::Config>::UnsignedFixedPoint;
pub(crate) type Inner<T> = <<T as fee::Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;
// pub(crate) type PolkaBTC<T> = <<T as treasury::Config>::PolkaBTC as Currency<
//     <T as frame_system::Config>::AccountId,
// >>::Balance;

pub struct RichOperator<T: Config> {
    pub(crate) data: DefaultOperator<T>,
}

pub type DefaultOperator<T> = Operator<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    DOT<T>,
>;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct Nominator<AccountId: Ord, BlockNumber, DOT> {
    pub id: AccountId,
    pub collateral: DOT,
    /// Map of request_id => (Maturity Block, DOT to withdraw)
    pub pending_withdrawals: BTreeMap<H256, (BlockNumber, DOT)>,
    pub collateral_to_be_withdrawn: DOT,
}

impl<AccountId: Ord, BlockNumber, DOT: HasCompact + Default>
    Nominator<AccountId, BlockNumber, DOT>
{
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

    pub fn force_refund_nominated_collateral(&mut self) -> DispatchResult {
        self.force_refund_nominators_proportionally(self.data.total_nominated_collateral)?;
        // Erase nominator data
        self.update(|v| {
            v.nominators = Default::default();
            v.total_nominated_collateral = Default::default();
            Ok(())
        })
    }

    pub fn deposit_nominated_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
        backing_collateral: DOT<T>,
    ) -> DispatchResult {
        let new_nominated_collateral = self
            .data
            .total_nominated_collateral
            .checked_add(&(collateral.clone()))
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let vault_collateral = backing_collateral
            .checked_sub(&(self.data.total_nominated_collateral).clone())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        ensure!(
            !Module::<T>::is_collateral_below_max_nomination_ratio(
                vault_collateral,
                new_nominated_collateral
            )?,
            Error::<T>::InsufficientCollateral
        );
        // Increase the sum of nominated collateral for this vault
        self.update(|v| {
            v.total_nominated_collateral = collateral
                .checked_add(&v.total_nominated_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })?;

        if !self.is_nominator(&nominator_id)? {
            // self.data.nominators.insert(nominator_id.clone(), new_nominator);
            self.update(|v| {
                v.nominators.insert(
                    nominator_id.clone(),
                    Nominator::new(nominator_id.clone()).clone(),
                );
                Ok(())
            })?;
        };

        self.update_nominator_collateral(nominator_id.clone(), collateral, false)
    }

    pub fn withdraw_nominated_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        let nominator = self
            .data
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;
        ensure!(
            collateral.le(&nominator.collateral),
            Error::<T>::TooLittleDelegatedCollateral
        );
        <collateral::Module<T>>::repatriate_reserved(
            self.data.id.clone(),
            nominator_id.clone(),
            collateral,
            BalanceStatus::Free,
        )?;

        // Decrease the sum of nominated collateral for this vault
        self.update(|v| {
            v.total_nominated_collateral = v
                .total_nominated_collateral
                .checked_sub(&collateral.clone())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })?;

        self.update_nominator_collateral(nominator_id.clone(), collateral, true)
    }

    fn update_nominator_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
        decrease: bool,
    ) -> DispatchResult {
        // If the remaining nominated collateral is zero, remove nominator from
        // the `nominators` map. Otherwise, update the map.
        let cloned_vault = self.data.clone();
        let nominator = cloned_vault
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;

        if decrease {
            self.decrease_nominator_collateral(
                nominator_id.clone(),
                collateral,
                nominator.collateral,
            )
        } else {
            self.increase_nominator_collateral(nominator_id.clone(), collateral)
        }
    }

    fn decrease_nominator_collateral(
        &mut self,
        nominator_id: T::AccountId,
        decrease_by: DOT<T>,
        nominated_collateral: DOT<T>,
    ) -> DispatchResult {
        if nominated_collateral.eq(&decrease_by.clone()) {
            self.update(|v| {
                v.nominators.remove(&(nominator_id.clone()));
                Ok(())
            })
        } else {
            self.update_nominator_collateral(nominator_id.clone(), decrease_by, true)
        }
    }

    fn increase_nominator_collateral(
        &mut self,
        nominator_id: T::AccountId,
        increase_by: DOT<T>,
    ) -> DispatchResult {
        self.update_nominator_collateral(nominator_id.clone(), increase_by, false)
    }

    pub fn slash_nominators_by(&mut self, amount_to_slash: DOT<T>) -> DispatchResult {
        let nominators = self.data.nominators.clone();
        let vault_clone = self.data.clone();
        for (nominator_id, _) in &vault_clone.nominators {
            let nominated_collateral_to_slash =
                self.scale_amount_by_nominator_proportion(amount_to_slash, nominator_id.clone())?;
            self.update_nominator_collateral(
                nominator_id.clone(),
                nominated_collateral_to_slash,
                true,
            )?;
        }

        self.update(|v| {
            v.nominators = nominators.clone();
            Ok(())
        })
    }

    pub fn scale_amount_by_nominator_proportion(
        &mut self,
        amount: DOT<T>,
        nominator_id: T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        let amount_u128 = Module::<T>::dot_to_u128(amount)?;
        let nominated_collateral_proportion =
            self.get_nominator_collateral_proportion(nominator_id.clone())?;
        let nominated_collateral_to_slash_u128 = nominated_collateral_proportion
            .checked_mul(amount_u128)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        Module::<T>::u128_to_dot(nominated_collateral_to_slash_u128)
    }

    pub fn get_nominator_collateral_proportion(
        &mut self,
        nominator_id: T::AccountId,
    ) -> Result<u128, DispatchError> {
        let nominator = self
            .data
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;
        let nominated_collateral_u128 = Module::<T>::dot_to_u128(nominator.collateral)?;
        let total_nominated_collateral_u128 =
            Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
        Ok(nominated_collateral_u128
            .checked_div(total_nominated_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    // Operator functionality

    pub fn add_pending_operator_withdrawal(
        &mut self,
        request_id: H256,
        collateral_to_withdraw: DOT<T>,
        maturity: T::BlockNumber,
    ) -> DispatchResult {
        let operator_collateral = self.get_operator_collateral()?;
        let remaining_operator_collateral = operator_collateral
            .checked_sub(&collateral_to_withdraw)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let max_nominatable_collateral =
            self.get_max_nominatable_collateral(remaining_operator_collateral)?;
        let nominated_collateral_to_refund = self
            .data
            .total_nominated_collateral
            .saturating_sub(max_nominatable_collateral);
        let zero_dot: DOT<T> = 0u32.into();
        if !nominated_collateral_to_refund.eq(&zero_dot) {
            // The Nominators are not assumed to be trusted by the Operator.
            // Unless the refund is forced (not subject to unbonding), a Nominator might cancel the
            // refund request and cause the Max Nomination Ratio to be exceeded.
            // As such, we need to force refund.
            self.force_refund_nominators_proportionally(nominated_collateral_to_refund)?;
        }
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

    pub fn force_refund_nominators_proportionally(&mut self, amount: DOT<T>) -> DispatchResult {
        let data_clone = self.data.clone();
        for nominator_id in data_clone.nominators.keys() {
            let nominator_collateral_to_refund =
                self.scale_amount_by_nominator_proportion(amount, nominator_id.clone())?;
            <collateral::Module<T>>::repatriate_reserved(
                self.data.id.clone(),
                nominator_id.clone(),
                nominator_collateral_to_refund,
                BalanceStatus::Free,
            )?;
        }
        Ok(())
    }

    pub fn execute_operator_withdrawal(&mut self) -> DispatchResult {
        // find mature request ids
        // compute the sum to withdraw and increase the backing collateral again
        // let withdrawal = *self
        //     .data
        //     .pending_withdrawals
        //     .get(&request_id)
        //     .ok_or(Error::<T>::WithdrawRequestNotFound)?;
        // let height = <frame_system::Module<T>>::block_number();
        // ensure!(
        //     withdrawal.0.ge(&height),
        //     Error::<T>::WithdrawRequestNotMatured
        // );
        // self.remove_pending_operator_withdrawal(request_id);
        // self.update(|v| {
        //     v.pending_withdrawals.remove(&request_id);
        //     v.collateral_to_be_withdrawn = v
        //         .collateral_to_be_withdrawn
        //         .checked_sub(&withdrawal.1)
        //         .ok_or(Error::<T>::ArithmeticUnderflow)?;
        //     Ok(())
        // })
        Ok(())
    }

    pub fn remove_pending_operator_withdrawal(&mut self, request_id: H256) {
        let _ = self.update(|v| {
            v.pending_withdrawals.remove(&request_id);
            Ok(())
        });
    }

    // Nominator functionality

    pub fn add_pending_nominator_withdrawal(
        &mut self,
        nominator_id: T::AccountId,
        request_id: H256,
        collateral_to_withdraw: DOT<T>,
        maturity: T::BlockNumber,
    ) {
        let _ = self.update(|v| {
            let mut nominator = v
                .nominators
                .get(&nominator_id)
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
        });
    }

    pub fn execute_nominator_withdrawal(&mut self, _nominator_id: T::AccountId) -> DispatchResult {
        // find mature request ids
        // compute the sum to withdraw and increase the backing collateral again

        // let nominators = self.data.nominators.clone();
        // let nominator = nominators
        //     .get(&(nominator_id.clone()))
        //     .ok_or(Error::<T>::NominatorNotFound)?;
        // let withdrawal = nominator
        //     .pending_withdrawals
        //     .get(&request_id)
        //     .ok_or(Error::<T>::WithdrawRequestNotFound)?;

        // let height = <frame_system::Module<T>>::block_number();
        // ensure!(
        //     withdrawal.0.ge(&height),
        //     Error::<T>::WithdrawRequestNotMatured
        // );
        // self.remove_pending_nominator_withdrawal(nominator_id.clone(), request_id);
        // self.update(|v| {
        //     let mut nominator = v
        //         .nominators
        //         .get(&nominator_id)
        //         .ok_or(Error::<T>::NominatorNotFound)?
        //         .clone();
        //     nominator.collateral_to_be_withdrawn = nominator
        //         .collateral_to_be_withdrawn
        //         .checked_sub(&withdrawal.1)
        //         .ok_or(Error::<T>::ArithmeticUnderflow)?;
        //     v.nominators.insert(nominator_id.clone(), nominator);
        //     Ok(())
        // })
        Ok(())
    }

    pub fn remove_pending_nominator_withdrawal(
        &mut self,
        nominator_id: T::AccountId,
        request_id: H256,
    ) {
        let _ = self.update(|v| {
            let mut nominator = v
                .nominators
                .get(&nominator_id)
                .ok_or(Error::<T>::NominatorNotFound)?
                .clone();
            nominator.pending_withdrawals.remove(&request_id);
            v.nominators.insert(nominator_id.clone(), nominator);
            Ok(())
        });
    }

    pub fn is_nominator(&mut self, nominator_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(self.data.nominators.contains_key(&nominator_id))
    }

    pub fn slash_nominators(&mut self, status: VaultStatus, to_slash: DOT<T>) -> DispatchResult {
        let nominated_collateral_to_slash =
            self.get_nominated_collateral_to_slash(to_slash, status)?;

        let nominated_collateral_remaining = self
            .data
            .total_nominated_collateral
            .checked_sub(&nominated_collateral_to_slash.clone())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        self.update(|v| {
            v.total_nominated_collateral = nominated_collateral_remaining;
            Ok(())
        })?;
        // Slash nominators proportionally
        self.slash_nominators_by(nominated_collateral_to_slash)
    }

    pub fn get_nominated_collateral_to_slash(
        &mut self,
        total_amount_to_slash: DOT<T>,
        status: VaultStatus,
    ) -> Result<DOT<T>, DispatchError> {
        let nominated_collateral_to_slash: DOT<T> = if status.eq(&VaultStatus::CommittedTheft) {
            let vault_collateral = self.get_operator_collateral()?;
            total_amount_to_slash
                .checked_sub(&vault_collateral.clone())
                .map_or(0u32.into(), |x| x)
        } else {
            let to_slash_u128 = Module::<T>::dot_to_u128(total_amount_to_slash)?;
            let nominator_collateral_proportion = self.get_nominated_collateral_proportion()?;
            let nominated_collateral_to_slash_u128 = nominator_collateral_proportion
                .checked_mul(to_slash_u128)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Module::<T>::u128_to_dot(nominated_collateral_to_slash_u128)?
        };
        Ok(nominated_collateral_to_slash)
    }

    pub fn get_operator_collateral(&mut self) -> Result<DOT<T>, DispatchError> {
        let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
        Ok(backing_collateral
            .checked_sub(&self.data.total_nominated_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_operator_collateral_proportion(&mut self) -> Result<u128, DispatchError> {
        let operator_collateral = self.get_operator_collateral()?;
        let operator_collateral_u128 = Module::<T>::dot_to_u128(operator_collateral)?;
        let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
        let backing_collateral_u128 = Module::<T>::dot_to_u128(backing_collateral)?;
        Ok(operator_collateral_u128
            .checked_div(backing_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_nominated_collateral_proportion(&mut self) -> Result<u128, DispatchError> {
        let total_nominated_collateral_u128 =
            Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
        let backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&self.id())?;
        let backing_collateral_u128 = Module::<T>::dot_to_u128(backing_collateral)?;
        Ok(total_nominated_collateral_u128
            .checked_div(backing_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_max_nominatable_collateral(
        &mut self,
        operator_collateral: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let operator_collateral_inner = ext::fee::dot_to_inner::<T>(operator_collateral)?;
        let max_nomination_ratio = Module::<T>::max_nomination_ratio();
        let max_nominatable_collateral =
            UnsignedFixedPoint::<T>::checked_from_integer(operator_collateral_inner)
                .ok_or(Error::<T>::TryIntoIntError)?
                .checked_div(&max_nomination_ratio)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
        ext::fee::inner_to_dot::<T>(max_nominatable_collateral.into_inner())
    }

    pub fn get_nomination_ratio(&mut self) -> Result<u128, DispatchError> {
        let operator_collateral = self.get_operator_collateral()?;
        let operator_collateral_u128 = Module::<T>::dot_to_u128(operator_collateral)?;
        let total_nominated_collateral_u128 =
            Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
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
