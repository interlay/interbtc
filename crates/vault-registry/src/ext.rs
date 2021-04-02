#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: collateral::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::transfer(source.clone(), destination.clone(), amount)
    }

    pub fn lock<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn release_collateral<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn for_account<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }

    pub fn get_free_balance<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_balance_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;

    pub fn total_issued<T: treasury::Config>() -> PolkaBTC<T> {
        <treasury::Module<T>>::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub trait Exchangeable: exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config {}
    impl<T> Exchangeable for T where T: exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config {}

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

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn ensure_parachain_does_not_have_errors<T: security::Config>(error_codes: Vec<ErrorCode>) -> DispatchResult {
        <security::Module<T>>::ensure_parachain_does_not_have_errors(error_codes)
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Module<T>>::active_block_number()
    }
}
