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
    use crate::types::{PolkaBTC, DOT};
    use x_core::Result;

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

    pub fn btc_to_dots<T: Exchangeable>(_amount: PolkaBTC<T>) -> Result<DOT<T>> {
        // FIXME: use rate
        let _rate = get_exchange_rate::<T>()?;
        Ok(Default::default())
    }

    pub fn dots_to_btc<T: Exchangeable>(_amount: DOT<T>) -> Result<PolkaBTC<T>> {
        // FIXME: use rate
        let _rate = get_exchange_rate::<T>()?;
        Ok(Default::default())
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use x_core::UnitResult;

    pub fn recover_from_liquidation<T: security::Trait>() -> UnitResult {
        Ok(())
    }
}
