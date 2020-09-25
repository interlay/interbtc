//! RPC interface for the Vault Registry.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as VaultRegistryClient;
pub use module_vault_registry_rpc_runtime_api::VaultRegistryApi as VaultRegistryRuntimeApi;

#[rpc]
pub trait VaultRegistryApi<BlockHash, AccountId, PolkaBTC> {
    #[rpc(name = "vaultRegistry_getFirstVaultWithSufficientCollateral")]
    fn get_first_vault_with_sufficient_collateral(&self, amount: PolkaBTC) -> Result<()>;

    #[rpc(name = "vaultRegistry_getIssueableTokensFromVault")]
    fn get_issuable_tokens_from_vault(&self, vault: AccountId) -> Result<()>;
}

/// A struct that implements the [`VaultRegistryApi`].
pub struct VaultRegistry<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> VaultRegistry<C, B> {
    /// Create new `VaultRegistry` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        VaultRegistry {
            client,
            _marker: Default::default(),
        }
    }
}

pub enum Error {
    RuntimeError,
}

impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
        }
    }
}

impl<C, Block, AccountId, PolkaBTC> VaultRegistryApi<<Block as BlockT>::Hash, AccountId, PolkaBTC>
    for VaultRegistry<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: VaultRegistryRuntimeApi<Block, AccountId>,
    AccountId: Codec,
    PolkaBTC: Codec,
{
    fn get_first_vault_with_sufficient_collateral(&self, amount: PolkaBTC) -> Result<AccountId> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let vault_id = api.get_first_vault_with_sufficient_collateral(&at, amount)?;
        Ok(vault_id)
    }
    fn get_issuable_tokens_from_vault(&self, vault: AccountId) -> Result<PolkaBTC> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let issuable = api.get_issuable_tokens_from_vault(&at, vault)?;
        Ok(issuable)
    }
}
