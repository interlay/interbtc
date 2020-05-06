#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use sp_std::vec::Vec;
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
    use crate::{PolkaBTC, DOT};
    use x_core::{Result, UnitResult};

    pub fn replace_tokens<T: vault_registry::Trait>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_replace_tokens(
            &old_vault_id,
            &new_vault_id,
            tokens,
            collateral,
        )
    }

    pub fn decrease_to_be_redeemed_tokens<T: vault_registry::Trait>(
        vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_decrease_to_be_redeemed_tokens(&vault_id, tokens)
    }

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>> {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn increase_to_be_redeemed_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<()> {
        <vault_registry::Module<T>>::_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn is_over_minimum_collateral<T: vault_registry::Trait>(_collateral: DOT<T>) -> bool {
        // FIXME: call from vault registry when ready
        unimplemented!()
    }

    pub fn is_vault_below_auction_threshold<T: vault_registry::Trait>(
        _vault_id: T::AccountId,
    ) -> Result<bool> {
        // FIXME: call from vault registry when ready
        unimplemented!()
    }

    pub fn is_collateral_below_secure_threshold<T: vault_registry::Trait>(
        _collateral: DOT<T>,
        _btc_amount_btc: PolkaBTC<T>,
    ) -> Result<bool> {
        //FIXME:
        unimplemented!()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use x_core::UnitResult;

    pub fn get_collateral_from_account<T: collateral::Trait>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn release_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::release_collateral(&sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        collateral: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::slash_collateral(old_vault_id, new_vault_id, collateral)
    }

    pub fn lock_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::lock_collateral(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use primitive_types::H256;
    use x_core::UnitResult;

    pub fn get_secure_id<T: security::Trait>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::_get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }
}
