use crate::{ext, Config, Error};
use codec::{Decode, Encode, HasCompact};
use sp_arithmetic::FixedPointNumber;
use vault_registry::SlashingAccessors;

#[cfg(test)]
use mocktopus::macros::mockable;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;

pub struct RichNominator<T: Config> {
    pub(crate) data: DefaultNominator<T>,
}

pub type DefaultNominator<T> = Nominator<<T as frame_system::Config>::AccountId, Collateral<T>, SignedFixedPoint<T>>;

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
pub struct Nominator<AccountId: Ord, Collateral, SignedFixedPoint> {
    pub id: AccountId,
    pub vault_id: AccountId,
    pub collateral: Collateral,
    pub slash_tally: SignedFixedPoint,
}

impl<AccountId: Ord, Collateral: HasCompact + Default, SignedFixedPoint: Default>
    Nominator<AccountId, Collateral, SignedFixedPoint>
{
    pub(crate) fn new(id: AccountId, vault_id: AccountId) -> Nominator<AccountId, Collateral, SignedFixedPoint> {
        Nominator {
            id,
            vault_id,
            collateral: Default::default(),
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

impl<T: Config> SlashingAccessors<Collateral<T>, SignedFixedPoint<T>, Error<T>> for RichNominator<T> {
    fn get_slash_per_token(&self) -> Result<SignedFixedPoint<T>, Error<T>> {
        let vault =
            ext::vault_registry::get_vault_from_id::<T>(&self.data.vault_id).map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.slash_per_token)
    }

    fn get_collateral(&self) -> Collateral<T> {
        self.data.collateral
    }

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Collateral<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.collateral)?;
        <crate::Nominators<T>>::insert((&self.data.id, &self.data.vault_id), self.data.clone());
        Ok(())
    }

    fn get_total_collateral(&self) -> Result<Collateral<T>, Error<T>> {
        let vault =
            ext::vault_registry::get_vault_from_id::<T>(&self.data.vault_id).map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.total_collateral)
    }

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Collateral<T>) -> Result<(), Error<T>>,
    {
        let mut vault =
            ext::vault_registry::get_vault_from_id::<T>(&self.data.vault_id).map_err(|_| Error::<T>::VaultNotFound)?;
        func(&mut vault.total_collateral)?;
        ext::vault_registry::insert_vault::<T>(&vault.id.clone(), vault);
        Ok(())
    }

    fn get_backing_collateral(&self) -> Result<Collateral<T>, Error<T>> {
        let vault =
            ext::vault_registry::get_vault_from_id::<T>(&self.data.vault_id).map_err(|_| Error::<T>::VaultNotFound)?;
        Ok(vault.backing_collateral)
    }

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Collateral<T>) -> Result<(), Error<T>>,
    {
        let mut vault =
            ext::vault_registry::get_vault_from_id::<T>(&self.data.vault_id).map_err(|_| Error::<T>::VaultNotFound)?;
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
        <crate::Nominators<T>>::insert((&self.data.id, &self.data.vault_id), self.data.clone());
        Ok(())
    }
}
