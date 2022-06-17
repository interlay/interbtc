//! RPC interface for the Replace Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use module_replace_rpc_runtime_api::ReplaceApi as ReplaceRuntimeApi;

#[rpc(client, server)]
pub trait ReplaceApi<BlockHash, AccountId, H256, ReplaceRequest> {
    #[method(name = "replace_getOldVaultReplaceRequests")]
    fn get_old_vault_replace_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;

    #[method(name = "replace_getNewVaultReplaceRequests")]
    fn get_new_vault_replace_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
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

#[async_trait]
impl<C, Block, AccountId, H256, ReplaceRequest>
    ReplaceApiServer<<Block as BlockT>::Hash, AccountId, H256, ReplaceRequest> for Replace<C, Block>
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
    ) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_old_vault_replace_requests(&at, vault_id)
            .map_err(|e| internal_err(format!("Unable to fetch replace requests: {:?}", e)))
    }

    fn get_new_vault_replace_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_new_vault_replace_requests(&at, vault_id)
            .map_err(|e| internal_err(format!("Unable to fetch replace requests: {:?}", e)))
    }
}
