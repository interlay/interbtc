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

    pub(crate) fn for_account<T: collateral::Trait>(id: &T::AccountId) -> DOTBalance<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use x_core::Error;

    pub(crate) fn get_exchange_rate<T: exchange_rate_oracle::Trait>() -> Result<u128, Error> {
        <exchange_rate_oracle::Module<T>>::get_exchange_rate()
    }
}
