//! RPC interface for the Refund Module.

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

pub use module_refund_rpc_runtime_api::RefundApi as RefundRuntimeApi;

#[rpc(client, server)]
pub trait RefundApi<BlockHash, AccountId, H256, RefundRequest> {
    #[method(name = "refund_getRefundRequests")]
    fn get_refund_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;
    #[method(name = "refund_getRefundRequestsByIssueId")]
    fn get_refund_requests_by_issue_id(&self, issue_id: H256, at: Option<BlockHash>) -> RpcResult<Option<H256>>;
    #[method(name = "refund_getVaultRefundRequests")]
    fn get_vault_refund_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`RefundApi`].
pub struct Refund<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Refund<C, B> {
    /// Create new `Refund` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Refund {
            client,
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<C, Block, AccountId, H256, RefundRequest> RefundApiServer<<Block as BlockT>::Hash, AccountId, H256, RefundRequest>
    for Refund<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RefundRuntimeApi<Block, AccountId, H256, RefundRequest>,
    AccountId: Codec,
    H256: Codec,
    RefundRequest: Codec,
{
    fn get_refund_requests(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_refund_requests(&at, account_id)
            .map_err(|e| internal_err(format!("Unable to fetch refund requests: {:?}", e)))
    }

    fn get_refund_requests_by_issue_id(
        &self,
        issue_id: H256,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_refund_requests_by_issue_id(&at, issue_id)
            .map_err(|e| internal_err(format!("Unable to fetch refund requests: {:?}", e)))
    }

    fn get_vault_refund_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_vault_refund_requests(&at, vault_id)
            .map_err(|e| internal_err(format!("Unable to fetch refund requests: {:?}", e)))
    }
}
