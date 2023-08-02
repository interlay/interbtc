//! RPC interface for the Issue Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub use issue_rpc_runtime_api::IssueApi as IssueRuntimeApi;

#[rpc(client, server)]
pub trait IssueApi<BlockHash, AccountId, H256, IssueRequest> {
    #[method(name = "issue_getIssueRequests")]
    fn get_issue_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;

    #[method(name = "issue_getVaultIssueRequests")]
    fn get_vault_issue_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`IssueApi`].
pub struct Issue<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Issue<C, B> {
    /// Create new `Issue` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Issue {
            client,
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<C, Block, AccountId, H256, IssueRequest> IssueApiServer<<Block as BlockT>::Hash, AccountId, H256, IssueRequest>
    for Issue<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: IssueRuntimeApi<Block, AccountId, H256, IssueRequest>,
    AccountId: Codec,
    H256: Codec,
    IssueRequest: Codec,
{
    fn get_issue_requests(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_issue_requests(at, account_id)
            .map_err(|e| internal_err(format!("Unable to fetch issue requests: {:?}", e)))
    }

    fn get_vault_issue_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_vault_issue_requests(at, vault_id)
            .map_err(|e| internal_err(format!("Unable to fetch issue requests: {:?}", e)))
    }
}
