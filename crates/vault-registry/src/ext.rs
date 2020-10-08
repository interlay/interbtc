#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn lock<T: collateral::Trait>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn release<T: collateral::Trait>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn slash<T: collateral::Trait>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }

    pub fn for_account<T: collateral::Trait>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {

    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub trait Exchangeable:
        exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Trait + ::treasury::Trait + ::collateral::Trait
    {
    }

    pub fn btc_to_dots<T: Exchangeable>(amount: PolkaBTC<T>) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }

    pub fn dots_to_btc<T: Exchangeable>(amount: DOT<T>) -> Result<PolkaBTC<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::dots_to_btc(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::ErrorCode;
    use sp_std::vec::Vec;

    pub fn recover_from_liquidation<T: security::Trait>() -> DispatchResult {
        Ok(())
    }

    pub fn ensure_parachain_status_running<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::_ensure_parachain_status_not_shutdown()
    }

    pub fn ensure_parachain_status_has_not_specific_errors<T: security::Trait>(
        error_codes: Vec<ErrorCode>,
    ) -> DispatchResult {
        <security::Module<T>>::_ensure_parachain_status_has_not_specific_errors(error_codes)
    }
}
