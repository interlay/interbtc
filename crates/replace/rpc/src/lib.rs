//! RPC interface for the Replace Module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as ReplaceClient;
pub use module_replace_rpc_runtime_api::ReplaceApi as ReplaceRuntimeApi;

#[rpc]
pub trait ReplaceApi<BlockHash, AccountId, H256, ReplaceRequest> {
    #[rpc(name = "replace_getOldVaultReplaceRequests")]
    fn get_old_vault_replace_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;

    #[rpc(name = "replace_getNewVaultReplaceRequests")]
    fn get_new_vault_replace_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;
}

/// A struct that implements the [`ReplaceApi`].
pub struct Replace<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Replace<C, B> {
    /// Create new `Replace` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Replace {
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

impl<C, Block, AccountId, H256, ReplaceRequest> ReplaceApi<<Block as BlockT>::Hash, AccountId, H256, ReplaceRequest>
    for Replace<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ReplaceRuntimeApi<Block, AccountId, H256, ReplaceRequest>,
    AccountId: Codec,
    H256: Codec,
    ReplaceRequest: Codec,
{
    fn get_old_vault_replace_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_old_vault_replace_requests(&at, vault_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch replace requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_new_vault_replace_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_new_vault_replace_requests(&at, vault_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch replace requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}
