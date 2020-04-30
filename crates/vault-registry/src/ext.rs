#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use x_core::UnitResult;

    pub fn lock<T: collateral::Trait>(sender: &T::AccountId, amount: DOT<T>) -> UnitResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn release<T: collateral::Trait>(sender: &T::AccountId, amount: DOT<T>) -> UnitResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn slash<T: collateral::Trait>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }

    pub fn for_account<T: collateral::Trait>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use sp_std::convert::TryInto;

    use crate::types::{PolkaBTC, DOT};
    use x_core::{Error, Result};

    pub trait Exchangeable:
        exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }

    pub fn get_exchange_rate<T: exchange_rate_oracle::Trait>() -> Result<u128> {
        <exchange_rate_oracle::Module<T>>::get_exchange_rate()
    }

    pub fn btc_to_dots<T: Exchangeable>(amount: PolkaBTC<T>) -> Result<DOT<T>> {
        let rate = get_exchange_rate::<T>()?;
        // XXX: for some reason amount.try_into() returns Result<usize, ...>
        // instead of doing type inference properly
        let raw_amount = TryInto::<u128>::try_into(amount).map_err(|_e| Error::RuntimeError)?;
        let converted = rate.checked_mul(raw_amount).ok_or(Error::RuntimeError)?;
        let result = converted.try_into().map_err(|_e| Error::RuntimeError)?;
        Ok(result)
    }

    pub fn dots_to_btc<T: Exchangeable>(amount: DOT<T>) -> Result<PolkaBTC<T>> {
        let rate = get_exchange_rate::<T>()?;
        let raw_amount = TryInto::<u128>::try_into(amount).map_err(|_e| Error::RuntimeError)?;
        let converted = raw_amount.checked_div(rate).ok_or(Error::RuntimeError)?;
        let result = converted.try_into().map_err(|_e| Error::RuntimeError)?;
        Ok(result)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use x_core::UnitResult;

    pub fn recover_from_liquidation<T: security::Trait>() -> UnitResult {
        Ok(())
    }
}
