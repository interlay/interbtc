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

    pub fn release_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
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

    pub trait Exchangeable:
        exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config
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

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn ensure_parachain_does_not_have_errors<T: security::Config>(
        error_codes: Vec<ErrorCode>,
    ) -> DispatchResult {
        <security::Module<T>>::ensure_parachain_does_not_have_errors(error_codes)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use frame_support::dispatch::DispatchResult;
    pub use nomination::VaultStatus;
    use primitive_types::H256;
    use sp_runtime::DispatchError;

    use crate::types::DOT;

    pub fn is_nomination_enabled<T: nomination::Config>() -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_nomination_enabled()
    }

    pub fn slash_nominators<T: nomination::Config>(
        vault_id: T::AccountId,
        status: VaultStatus,
        to_slash: DOT<T>,
        backing_collateral_before_slashing: DOT<T>,
    ) -> DispatchResult {
        <nomination::Module<T>>::slash_nominators(
            vault_id,
            status,
            to_slash,
            backing_collateral_before_slashing,
        )
    }

    pub fn opt_in_to_nomination<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> DispatchResult {
        <nomination::Module<T>>::opt_in_to_nomination(operator_id)
    }

    pub fn opt_out_of_nomination<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> DispatchResult {
        <nomination::Module<T>>::opt_out_of_nomination(operator_id)
    }

    pub fn get_total_nominated_collateral<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        <nomination::Module<T>>::get_total_nominated_collateral(operator_id)
    }

    pub fn is_operator<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_operator(operator_id)
    }

    pub fn deposit_nominated_collateral<T: nomination::Config>(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
        collateral: DOT<T>,
        backing_collateral: DOT<T>,
    ) -> DispatchResult {
        <nomination::Module<T>>::deposit_nominated_collateral(
            nominator_id,
            vault_id,
            collateral,
            backing_collateral,
        )
    }

    pub fn request_operator_withdrawal<T: nomination::Config>(
        operator_id: &T::AccountId,
        collateral_to_withdraw: DOT<T>,
        backing_collateral_before_withdrawal: DOT<T>,
    ) -> DispatchResult {
        <nomination::Module<T>>::request_operator_withdrawal(
            operator_id,
            collateral_to_withdraw,
            backing_collateral_before_withdrawal,
        )
    }

    pub fn request_nominator_withdrawal<T: nomination::Config>(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        collateral_to_withdraw: DOT<T>,
    ) -> DispatchResult {
        <nomination::Module<T>>::request_nominator_withdrawal(
            operator_id,
            nominator_id,
            collateral_to_withdraw,
        )
    }

    pub fn execute_operator_withdrawal<T: nomination::Config>(
        operator_id: &T::AccountId,
        request_id: H256,
    ) -> DispatchResult {
        <nomination::Module<T>>::execute_operator_withdrawal(operator_id, request_id)
    }

    pub fn execute_nominator_withdrawal<T: nomination::Config>(
        operator_id: &T::AccountId,
        nominator_id: &T::AccountId,
        request_id: H256,
    ) -> DispatchResult {
        <nomination::Module<T>>::execute_nominator_withdrawal(operator_id, nominator_id, request_id)
    }
}
