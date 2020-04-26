#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOTBalance;
    use x_core::Error;

    pub(crate) fn lock<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOTBalance<T>,
    ) -> Result<(), Error> {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub(crate) fn release<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOTBalance<T>,
    ) -> Result<(), Error> {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub(crate) fn slash<T: collateral::Trait>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOTBalance<T>,
    ) -> Result<(), Error> {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }

    pub(crate) fn for_account<T: collateral::Trait>(id: &T::AccountId) -> DOTBalance<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{DOTBalance, PolkaBTCBalance};
    use x_core::Error;

    pub trait Exchangeable:
        exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }

    pub(crate) fn get_exchange_rate<T: exchange_rate_oracle::Trait>() -> Result<u128, Error> {
        <exchange_rate_oracle::Module<T>>::get_exchange_rate()
    }

    pub(crate) fn btc_to_dots<T: Exchangeable>(
        _amount: PolkaBTCBalance<T>,
    ) -> Result<DOTBalance<T>, Error> {
        // FIXME: use rate
        let _rate = get_exchange_rate::<T>()?;
        Ok(Default::default())
    }

    pub(crate) fn dots_to_btc<T: Exchangeable>(
        _amount: DOTBalance<T>,
    ) -> Result<PolkaBTCBalance<T>, Error> {
        // FIXME: use rate
        let _rate = get_exchange_rate::<T>()?;
        Ok(Default::default())
    }
}
