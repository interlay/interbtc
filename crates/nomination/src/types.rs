use crate::{ext, Config, Error};
use codec::{Decode, Encode, HasCompact};
use frame_support::{dispatch::DispatchResult, traits::Currency, StorageMap};
use sp_core::H256;
use sp_runtime::{
    traits::{CheckedAdd, CheckedSub},
    DispatchError,
};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

#[cfg(test)]
use mocktopus::macros::mockable;
use vault_registry::Collateral;

pub(crate) type Backing<T> =
    <<T as currency::Config<currency::Backing>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as fee::Config>::UnsignedFixedPoint;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub struct RichOperator<T: Config> {
    pub(crate) data: DefaultOperator<T>,
}

pub type DefaultOperator<T> =
    Operator<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::BlockNumber, Backing<T>>;

pub struct RichNominator<T: Config> {
    pub(crate) data: DefaultNominator<T>,
}

pub type DefaultNominator<T> = Nominator<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    Backing<T>,
    SignedFixedPoint<T>,
>;

impl<T: Config> From<&RichNominator<T>> for DefaultNominator<T> {
    fn from(rn: &RichNominator<T>) -> DefaultNominator<T> {
        rn.data.clone()
    }
}

impl<T: Config> From<DefaultNominator<T>> for RichNominator<T> {
    fn from(vault: DefaultNominator<T>) -> RichNominator<T> {
        RichNominator { data: vault }
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct Nominator<AccountId: Ord, BlockNumber, Backing, SignedFixedPoint> {
    pub id: AccountId,
    pub operator_id: AccountId,
    pub collateral: Backing,
    /// Map of request_id => (Maturity Block, collateral to withdraw)
    pub pending_withdrawals: BTreeMap<H256, (BlockNumber, Backing)>,
    pub collateral_to_be_withdrawn: Backing,
    pub slash_tally: SignedFixedPoint,
}

impl<AccountId: Ord, BlockNumber, Backing: HasCompact + Default, SignedFixedPoint: Default>
    Nominator<AccountId, BlockNumber, Backing, SignedFixedPoint>
{
    pub(crate) fn new(
        id: AccountId,
        operator_id: AccountId,
    ) -> Nominator<AccountId, BlockNumber, Backing, SignedFixedPoint> {
        Nominator {
            id,
            operator_id,
            collateral: Default::default(),
            pending_withdrawals: Default::default(),
            collateral_to_be_withdrawn: Default::default(),
            slash_tally: Default::default(),
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichNominator<T> {
    pub fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct Operator<AccountId: Ord, BlockNumber, Backing> {
    // Account identifier of the Vault
    pub id: AccountId,
    /// Map of request_id => (Maturity Block, collateral to withdraw)
    pub pending_withdrawals: BTreeMap<H256, (BlockNumber, Backing)>,
    pub collateral_to_be_withdrawn: Backing,
}

impl<AccountId: Ord, BlockNumber, Backing: HasCompact + Default> Operator<AccountId, BlockNumber, Backing> {
    pub(crate) fn new(id: AccountId) -> Operator<AccountId, BlockNumber, Backing> {
        Operator {
            id,
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

    pub fn has_nominated_collateral(&self) -> bool {
        // TODO: implement
        true
    }

    // pub fn withdraw_nominated_collateral(
    //     &mut self,
    //     nominator_id: T::AccountId,
    //     collateral: Backing<T>,
    // ) -> DispatchResult {
    //     let nominator = self
    //         .data
    //         .nominators
    //         .get_mut(&nominator_id)
    //         .ok_or(Error::<T>::NominatorNotFound)?;
    //     ensure!(
    //         collateral <= nominator.collateral,
    //         Error::<T>::TooLittleNominatedCollateral
    //     );
    //     self.decrease_nominator_collateral(nominator_id.clone(), collateral)?;
    //     // ext::vault_registry::withdraw_collateral_to_address::<T>(&self.id(), collateral, &nominator_id)
    //     Ok(())
    // }

    // Operator functionality

    pub fn add_pending_operator_withdrawal(
        &mut self,
        request_id: H256,
        collateral_to_withdraw: Backing<T>,
        maturity: T::BlockNumber,
    ) -> DispatchResult {
        // // If `collateral_to_withdraw` is larger than (operator_collateral - collateral_to_be_withdrawn),
        // // the following throws an error.
        // let operator_collateral = self.get_operator_collateral()?;
        // let remaining_operator_collateral = operator_collateral
        //     .checked_sub(&self.data.collateral_to_be_withdrawn)
        //     .ok_or(Error::<T>::InsufficientCollateral)?
        //     .checked_sub(&collateral_to_withdraw)
        //     .ok_or(Error::<T>::InsufficientCollateral)?;

        // // Trigger forced refunds if remaining nominated collateral would
        // // exceed the Max Nomination Ratio.
        // let max_nominatable_collateral = self.get_max_nominatable_collateral(remaining_operator_collateral)?;
        // let nominated_collateral_to_force_refund = self
        //     .data
        //     .total_nominated_collateral
        //     .saturating_sub(max_nominatable_collateral);
        // if !nominated_collateral_to_force_refund.is_zero() {
        //     // The Nominators are not assumed to be trusted by the Operator.
        //     // Unless the refund is forced (not subject to unbonding), a Nominator might cancel the
        //     // refund request and cause the Max Nomination Ratio to be exceeded.
        //     // As such, we need to force refund.
        //     self.force_refund_nominators_proportionally(nominated_collateral_to_force_refund)?;
        // }

        // Add withdrawal request and increase collateral to-be-withdrawn
        self.update(|v| {
            v.pending_withdrawals
                .insert(request_id, (maturity, collateral_to_withdraw));
            v.collateral_to_be_withdrawn = v
                .collateral_to_be_withdrawn
                .checked_add(&collateral_to_withdraw)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })?;
        // Lower the Operator's backing_collateral, to prevent them from issuing with the
        // to-be-withdrawn collateral.
        // ext::vault_registry::decrease_backing_collateral::<T>(&self.id(), collateral_to_withdraw)
        Ok(())
    }

    pub fn execute_operator_withdrawal(&mut self) -> Result<Backing<T>, DispatchError> {
        let matured_operator_withdrawal_requests =
            Self::get_matured_withdrawal_requests(&self.data.pending_withdrawals);
        let mut matured_collateral_to_withdraw: Backing<T> = 0u32.into();
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
        // ext::vault_registry::increase_backing_collateral::<T>(&self.id(), matured_collateral_to_withdraw)?;
        // ext::vault_registry::withdraw_collateral_to_address::<T>(
        //     &self.id(),
        //     matured_collateral_to_withdraw,
        //     &self.id(),
        // )?;
        Ok(matured_collateral_to_withdraw)
    }

    pub fn get_matured_withdrawal_requests(
        requests: &BTreeMap<H256, (T::BlockNumber, Backing<T>)>,
    ) -> Vec<(H256, (T::BlockNumber, Backing<T>))> {
        let current_height = ext::security::active_block_number::<T>();
        requests
            .clone()
            .iter()
            .filter(|(_, (maturity, _))| current_height >= *maturity)
            .map(|(id, withdrawal_request)| (id.clone(), withdrawal_request.clone()))
            .collect::<Vec<(H256, (T::BlockNumber, Backing<T>))>>()
    }

    pub fn remove_pending_operator_withdrawal(&mut self, request_id: H256) -> DispatchResult {
        let (_, withdrawal_amount) = *self
            .data
            .pending_withdrawals
            .get_mut(&request_id)
            .ok_or(Error::<T>::WithdrawalRequestNotFound)?;
        self.update(|v| {
            v.collateral_to_be_withdrawn = v
                .collateral_to_be_withdrawn
                .checked_sub(&withdrawal_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            v.pending_withdrawals.remove(&request_id);

            // The Operator's backing collateral was decreased when the withdrawal request
            // was made, to prevent issuing with the to-be-withdrawn collateral.
            // Irrespective of this function being called by `execute_withdrawal` or
            // `cancel_withdrawal`, the backing collateral needs to be increased back
            // in the vault_registry. In the former case, this is because collateral
            // is actually transferred to the withdrawer's account via the standard
            // function from the vault_registry (which subtracts from the backing collateral).
            // ext::vault_registry::increase_backing_collateral::<T>(&v.id, withdrawal_amount)
            Ok(())
        })
    }

    // Nominator functionality

    // pub fn add_pending_nominator_withdrawal(
    //     &mut self,
    //     nominator_id: T::AccountId,
    //     request_id: H256,
    //     collateral_to_withdraw: Backing<T>,
    //     maturity: T::BlockNumber,
    // ) -> DispatchResult {
    // Ensure that the Nominator has enough collateral for the withdrawal
    // let nominator = self
    //     .data
    //     .nominators
    //     .get_mut(&nominator_id)
    //     .ok_or(Error::<T>::NominatorNotFound)?;
    // let withdrawable_collateral = nominator
    //     .collateral
    //     .checked_sub(&nominator.collateral_to_be_withdrawn)
    //     .ok_or(Error::<T>::ArithmeticUnderflow)?;
    // ensure!(
    //     collateral_to_withdraw <= withdrawable_collateral,
    //     Error::<T>::TooLittleNominatedCollateral
    // );

    // // Add withdrawal request and increase collateral to-be-withdrawn
    // self.update(|v| {
    //     let mut nominator = v
    //         .nominators
    //         .get_mut(&nominator_id)
    //         .ok_or(Error::<T>::NominatorNotFound)?
    //         .clone();
    //     nominator
    //         .pending_withdrawals
    //         .insert(request_id, (maturity, collateral_to_withdraw));
    //     nominator.collateral_to_be_withdrawn = nominator
    //         .collateral_to_be_withdrawn
    //         .checked_add(&collateral_to_withdraw)
    //         .ok_or(Error::<T>::ArithmeticOverflow)?;
    //     v.nominators.insert(nominator_id.clone(), nominator);
    //     Ok(())
    // })?;
    // Lower the Operator's backing_collateral, to prevent them from issuing with the
    // to-be-withdrawn collateral.
    // ext::vault_registry::decrease_backing_collateral::<T>(&self.id(), collateral_to_withdraw)
    //     Ok(())
    // }

    // pub fn execute_nominator_withdrawal(&mut self, nominator_id: T::AccountId) -> Result<Backing<T>, DispatchError> {
    // let nominators = self.data.nominators.clone();
    // let nominator = nominators
    //     .get(&nominator_id)
    //     .ok_or(Error::<T>::NominatorNotFound)?
    //     .clone();
    // let matured_nominator_withdrawal_requests =
    //     Self::get_matured_withdrawal_requests(&nominator.pending_withdrawals);
    // let mut matured_collateral_to_withdraw: Backing<T> = 0u32.into();
    // for withdrawal in matured_nominator_withdrawal_requests.iter() {
    //     self.remove_pending_nominator_withdrawal(&nominator_id, withdrawal.0)?;
    //     matured_collateral_to_withdraw = matured_collateral_to_withdraw
    //         .checked_add(&withdrawal.1 .1)
    //         .ok_or(Error::<T>::ArithmeticOverflow)?;
    // }
    // The backing collateral was decreased when the withdrawal requests were made,
    // to prevent issuing with the to-be-withdrawn collateral.
    // Now, increase the backing collateral back, so it can be withdrawn using the
    // standard function from the vault_registry.
    // ext::vault_registry::increase_backing_collateral::<T>(&self.id(), matured_collateral_to_withdraw)?;
    // ext::vault_registry::withdraw_collateral_to_address::<T>(
    //     &self.id(),
    //     matured_collateral_to_withdraw,
    //     &nominator_id,
    // )?;
    // Ok(matured_collateral_to_withdraw)
    // }

    // pub fn remove_pending_nominator_withdrawal(
    //     &mut self,
    //     nominator_id: &T::AccountId,
    //     request_id: H256,
    // ) -> DispatchResult {
    // self.update(|v| {
    //     let mut nominator = v
    //         .nominators
    //         .get_mut(&nominator_id)
    //         .ok_or(Error::<T>::NominatorNotFound)?
    //         .clone();
    //     let (_, withdrawal_amount) = *nominator
    //         .pending_withdrawals
    //         .get_mut(&request_id)
    //         .ok_or(Error::<T>::WithdrawalRequestNotFound)?;
    //     nominator.collateral_to_be_withdrawn = nominator
    //         .collateral_to_be_withdrawn
    //         .checked_sub(&withdrawal_amount)
    //         .ok_or(Error::<T>::ArithmeticUnderflow)?;
    //     nominator.pending_withdrawals.remove(&request_id);
    //     v.nominators.insert(nominator_id.clone(), nominator);

    // The Operator's backing collateral was decreased when the withdrawal request
    // was made, to prevent issuing with the to-be-withdrawn collateral.
    // Irrespective of this function being called by `execute_withdrawal` or
    // `cancel_withdrawal`, the backing collateral needs to be increased back
    // in the vault_registry. In the former case, this is because collateral
    // is actually transferred to the withdrawer's account via the standard
    // function from the vault_registry (which subtracts from the backing collateral).
    // ext::vault_registry::increase_backing_collateral::<T>(&v.id, withdrawal_amount)
    //     Ok(())
    // })
    // }

    fn update<F>(&mut self, func: F) -> DispatchResult
    where
        F: Fn(&mut DefaultOperator<T>) -> DispatchResult,
    {
        func(&mut self.data)?;
        <crate::Operators<T>>::mutate(&self.data.id, func)?;
        Ok(())
    }
}

impl<T: Config> Collateral<Backing<T>, SignedFixedPoint<T>, Error<T>> for RichNominator<T> {
    fn get_slash_per_token(&self) -> Result<SignedFixedPoint<T>, Error<T>> {
        let vault = ext::vault_registry::get_vault_from_id::<T>(&self.data.operator_id)
            .map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.slash_per_token)
    }

    fn get_collateral(&self) -> Backing<T> {
        self.data.collateral
    }

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.collateral)?;
        <crate::Nominators<T>>::insert((&self.data.id, &self.data.operator_id), self.data.clone());
        Ok(())
    }

    fn get_total_collateral(&self) -> Result<Backing<T>, Error<T>> {
        let vault = ext::vault_registry::get_vault_from_id::<T>(&self.data.operator_id)
            .map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.total_collateral)
    }

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        let mut vault = ext::vault_registry::get_vault_from_id::<T>(&self.data.operator_id)
            .map_err(|_| Error::<T>::VaultNotFound)?;
        func(&mut vault.total_collateral)?;
        ext::vault_registry::insert_vault::<T>(&vault.id.clone(), vault);
        Ok(())
    }

    fn get_backing_collateral(&self) -> Result<Backing<T>, Error<T>> {
        let vault = ext::vault_registry::get_vault_from_id::<T>(&self.data.operator_id)
            .map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.backing_collateral)
    }

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        let mut vault = ext::vault_registry::get_vault_from_id::<T>(&self.data.operator_id)
            .map_err(|_| Error::<T>::VaultNotFound)?;
        func(&mut vault.backing_collateral)?;
        ext::vault_registry::insert_vault::<T>(&vault.id.clone(), vault);
        Ok(())
    }

    fn get_slash_tally(&self) -> SignedFixedPoint<T> {
        self.data.slash_tally
    }

    fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut SignedFixedPoint<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.slash_tally)?;
        <crate::Nominators<T>>::insert((&self.data.id, &self.data.operator_id), self.data.clone());
        Ok(())
    }
}
