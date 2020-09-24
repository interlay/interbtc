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
pub trait VaultRegistryApi<BlockHash, AccountId> {
    #[rpc(name = "vaultRegistry_getFirstVaultWithSufficientCollateral")]
    fn get_first_vault_with_sufficient_collateral(&self, amount: T::PolkaBTC) -> Result<()>;

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

impl<C, Block, AccountId> VaultRegistryApi<<Block as BlockT>::Hash, AccountId>
    for VaultRegistry<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: VaultRegistryRuntimeApi<Block, AccountId>,
    AccountId: Codec,
{
    fn get_first_vault_with_sufficient_collateral(&self, amount: T::PolkaBTC) -> Result<()> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_first_vault_with_sufficient_collateral(&at, amount)
            .map_or_else(
                |e| {
                    Err(RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to get a vault".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
                |result| {
                    result.map_err(|e| RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Vault found.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
            )
    }
    fn get_issuable_tokens_from_vault(&self, vault: AccountId) -> Result<()> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_issuable_tokens_from_vault(&at, vaul).map_or_else(
            |e| {
                Err(RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to get number of issuable token".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
            |result| {
                result.map_err(|e| RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Got number of issuable tokens.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
        )
    }
}
