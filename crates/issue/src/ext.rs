#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use x_core::UnitResult;

    pub fn verify_transaction_inclusion<T: btc_relay::Trait>(
        tx_id: H256Le,
        tx_block_height: u32,
        merkle_proof: Vec<u8>,
    ) -> UnitResult {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(
            tx_id,
            tx_block_height,
            merkle_proof,
            0,
            false,
        )
    }

    pub fn validate_transaction<T: btc_relay::Trait>(
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: Vec<u8>,
        issue_id: Vec<u8>,
    ) -> UnitResult {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, issue_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::PolkaBTC;
    use sp_core::H160;
    use x_core::{Result, UnitResult};

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>> {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn increase_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<H160> {
        <vault_registry::Module<T>>::_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_decrease_to_be_issued_tokens(vault_id, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use x_core::UnitResult;

    pub fn lock_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;

    pub fn mint<T: treasury::Trait>(requester: T::AccountId, amount: PolkaBTC<T>) {
        <treasury::Module<T>>::mint(requester, amount)
    }
}
