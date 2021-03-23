use crate::VaultStatus;
use crate::{
    ext, sp_api_hidden_includes_decl_storage::hidden_include::StorageValue, Config, Error, Module,
};
use codec::{Decode, Encode, HasCompact};
use frame_support::traits::BalanceStatus;
use frame_support::{dispatch::DispatchResult, ensure, traits::Currency, StorageMap};
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero};
use sp_runtime::DispatchError;
use sp_std::collections::btree_map::BTreeMap;

#[cfg(test)]
use mocktopus::macros::mockable;

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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
pub struct Operator<AccountId: Ord, BlockNumber, DOT> {
    // Account identifier of the Vault
    pub id: AccountId,
    pub nominators: BTreeMap<AccountId, DOT>,
    pub total_nominated_collateral: DOT,
    /// Vector of (Maturity Block, DOT to withdraw)
    pub pending_operator_withdrawals: BTreeMap<H256, (BlockNumber, DOT)>,
    /// Vector of (Maturity Block, Nominator Id, DOT to withdraw)
    pub pending_nominator_withdrawals: BTreeMap<H256, (BlockNumber, AccountId, DOT)>,
}

impl<AccountId: Ord, BlockNumber, DOT: HasCompact + Default> Operator<AccountId, BlockNumber, DOT> {
    pub(crate) fn new(id: AccountId) -> Operator<AccountId, BlockNumber, DOT> {
        Operator {
            id,
            nominators: Default::default(),
            total_nominated_collateral: Default::default(),
            pending_nominator_withdrawals: Default::default(),
            pending_operator_withdrawals: Default::default(),
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
    fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    pub fn refund_nominated_collateral(&mut self) -> DispatchResult {
        for (nominator_id, nominated_collateral) in &self.data.nominators {
            <collateral::Module<T>>::repatriate_reserved(
                self.data.id.clone(),
                nominator_id.clone(),
                *nominated_collateral,
                BalanceStatus::Free,
            )?;
        }
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
    ) -> DispatchResult {
        let new_nominated_collateral = self
            .data
            .total_nominated_collateral
            .checked_add(&(collateral.clone()))
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let vault_collateral = self
            .data
            .backing_collateral
            .checked_sub(&(self.data.total_nominated_collateral).clone())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        ensure!(
            !Module::<T>::is_nominated_collateral_below_limit_rate(
                vault_collateral,
                new_nominated_collateral
            )?,
            Error::<T>::InsufficientCollateral
        );
        // Lock the Nominator's collateral
        ext::collateral::lock::<T>(&nominator_id, collateral)?;
        <collateral::Module<T>>::repatriate_reserved(
            nominator_id.clone(),
            self.data.id.clone(),
            collateral,
            BalanceStatus::Reserved,
        )?;
        // Increase the sum of nominated collateral for this vault
        self.update(|v| {
            v.total_nominated_collateral = collateral
                .checked_add(&v.total_nominated_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })?;

        self.update_nominator_entry(nominator_id.clone(), collateral, false)?;
        // Increase the system backing collateral
        self.update(|v| {
            v.backing_collateral = collateral
                .checked_add(&v.backing_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })?;
        Module::<T>::increase_total_backing_collateral(collateral)
    }

    pub fn withdraw_nominated_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        let nominated_collateral = self
            .data
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;
        ensure!(
            collateral.le(&(nominated_collateral.clone())),
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

        self.update_nominator_entry(nominator_id.clone(), collateral, true)?;

        // Decrease the system backing collateral
        self.update(|v| {
            v.backing_collateral = v
                .backing_collateral
                .checked_sub(&collateral)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })?;
        Module::<T>::decrease_total_backing_collateral(collateral)
    }

    fn update_nominator_entry(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
        decrease: bool,
    ) -> DispatchResult {
        // If the remaining nominated collateral is zero, remove nominator from
        // the `nominators` map. Otherwise, update the map.
        let cloned_vault = self.data.clone();
        let nominated_collateral = cloned_vault
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;

        if decrease {
            self.decrease_nominator_collateral(
                nominator_id.clone(),
                collateral,
                *nominated_collateral,
            )
        } else {
            self.increase_nominator_collateral(
                nominator_id.clone(),
                collateral,
                *nominated_collateral,
            )
        }
    }

    fn decrease_nominator_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
        nominated_collateral: DOT<T>,
    ) -> DispatchResult {
        if nominated_collateral.eq(&collateral.clone()) {
            self.update(|v| {
                v.nominators.remove(&(nominator_id.clone()));
                Ok(())
            })
        } else {
            let remaining_nominated_collateral = nominated_collateral
                .checked_sub(&collateral.clone())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            self.update(|v| {
                v.nominators
                    .insert(nominator_id.clone(), remaining_nominated_collateral);
                Ok(())
            })
        }
    }

    fn increase_nominator_collateral(
        &mut self,
        nominator_id: T::AccountId,
        collateral: DOT<T>,
        nominated_collateral: DOT<T>,
    ) -> DispatchResult {
        let new_nominated_collateral = collateral
            .checked_add(&nominated_collateral)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        self.update(|v| {
            v.nominators
                .insert(nominator_id.clone(), new_nominated_collateral);
            Ok(())
        })
    }

    pub fn slash_nominators_by(&mut self, amount_to_slash: DOT<T>) -> DispatchResult {
        let amount_to_slash_u128 = Module::<T>::dot_to_u128(amount_to_slash)?;
        let mut nominators = self.data.nominators.clone();
        let vault_clone = self.data.clone();
        for (nominator_id, _) in &vault_clone.nominators {
            let nominated_collateral_proportion =
                self.get_nominated_collateral_proportion_for(nominator_id.clone())?;
            let nominated_collateral_to_slash = nominated_collateral_proportion
                .checked_mul(amount_to_slash_u128)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let remaining_nominator_collateral: DOT<T> = match nominators.get(&(nominator_id)) {
                Some(x) => {
                    let x_u128 = Module::<T>::dot_to_u128(*x)?;
                    let remaining_collateral_u128 = x_u128
                        .checked_sub(nominated_collateral_to_slash)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    Module::<T>::u128_to_dot(remaining_collateral_u128)?
                }
                None => 0u32.into(),
            };
            if !remaining_nominator_collateral.is_zero() {
                nominators.insert(nominator_id.clone(), remaining_nominator_collateral);
            } else {
                nominators.remove(&nominator_id.clone());
            }
        }

        self.update(|v| {
            v.nominators = nominators.clone();
            Ok(())
        })
    }

    pub fn get_nominated_collateral_proportion_for(
        &mut self,
        nominator_id: T::AccountId,
    ) -> Result<u128, DispatchError> {
        let nominated_collateral = self
            .data
            .nominators
            .get(&(nominator_id.clone()))
            .ok_or(Error::<T>::NominatorNotFound)?;
        let nominated_collateral_u128 = Module::<T>::dot_to_u128(nominated_collateral.clone())?;
        let total_nominated_collateral_u128 =
            Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
        Ok(nominated_collateral_u128
            .checked_div(total_nominated_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn remove_pending_operator_withdrawal(&mut self, request_id: H256) {
        let _ = self.update(|v| {
            v.pending_operator_withdrawals.remove(&request_id);
            Ok(())
        });
    }

    pub fn add_pending_operator_withdrawal(
        &mut self,
        request_id: H256,
        withdrawal_request: (T::BlockNumber, DOT<T>),
    ) {
        let _ = self.update(|v| {
            v.pending_operator_withdrawals
                .insert(request_id, withdrawal_request.clone());
            Ok(())
        });
    }

    pub fn remove_pending_nominator_withdrawal(&mut self, request_id: H256) {
        let _ = self.update(|v| {
            v.pending_nominator_withdrawals.remove(&request_id);
            Ok(())
        });
    }

    pub fn add_pending_nominator_withdrawal(
        &mut self,
        request_id: H256,
        withdrawal_request: (T::BlockNumber, T::AccountId, DOT<T>),
    ) {
        let _ = self.update(|v| {
            v.pending_nominator_withdrawals
                .insert(request_id, withdrawal_request.clone());
            Ok(())
        });
    }

    pub fn slash_nominators(
        &mut self,
        status: VaultStatus,
        to_slash: DOT<T>,
        backing_collateral_before_slashing: DOT<T>,
    ) -> DispatchResult {
        let nominated_collateral_to_slash = self.get_nominated_collateral_to_slash(
            to_slash,
            status,
            backing_collateral_before_slashing,
        )?;

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
        backing_collateral: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let nominated_collateral_to_slash: DOT<T> = if status.eq(&VaultStatus::CommittedTheft) {
            let vault_collateral = self.get_operator_collateral(self.id(), backing_collateral)?;
            total_amount_to_slash
                .checked_sub(&vault_collateral.clone())
                .map_or(0u32.into(), |x| x)
        } else {
            let to_slash_u128 = Module::<T>::dot_to_u128(total_amount_to_slash)?;
            let nominator_collateral_proportion =
                self.get_nominator_collateral_proportion(backing_collateral)?;
            let nominated_collateral_to_slash_u128 = nominator_collateral_proportion
                .checked_mul(to_slash_u128)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Module::<T>::u128_to_dot(nominated_collateral_to_slash_u128)?
        };
        Ok(nominated_collateral_to_slash)
    }

    pub fn get_operator_collateral(
        &mut self,
        vault_id: T::AccountId,
        backing_collateral: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        // check if nomination is on and subtract nominated collateral
        // else return backing collateral
        Ok(backing_collateral
            .checked_sub(&self.data.total_nominated_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_vault_collateral_proportion(
        &mut self,
        vault_id: T::AccountId,
        backing_collateral: DOT<T>,
    ) -> Result<u128, DispatchError> {
        let vault_collateral = self.get_operator_collateral(vault_id, backing_collateral)?;
        let vault_collateral_u128 = Module::<T>::dot_to_u128(vault_collateral)?;
        let backing_collateral_u128 = Module::<T>::dot_to_u128(backing_collateral)?;
        Ok(vault_collateral_u128
            .checked_div(backing_collateral_u128)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_nominator_collateral_proportion(
        &mut self,
        backing_collateral: DOT<T>,
    ) -> Result<u128, DispatchError> {
        let total_nominated_collateral_u128 =
            Module::<T>::dot_to_u128(self.data.total_nominated_collateral)?;
        let backing_collateral_u128 = Module::<T>::dot_to_u128(backing_collateral)?;
        Ok(total_nominated_collateral_u128
            .checked_div(backing_collateral_u128)
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
