//! RPC interface for the Refund Module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as RefundClient;
pub use module_refund_rpc_runtime_api::RefundApi as RefundRuntimeApi;

#[rpc]
pub trait RefundApi<BlockHash, AccountId, H256, RefundRequest> {
    #[rpc(name = "refund_getRefundRequests")]
    fn get_refund_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> JsonRpcResult<Vec<H256>>;
    #[rpc(name = "refund_getRefundRequestsByIssueId")]
    fn get_refund_requests_by_issue_id(&self, issue_id: H256, at: Option<BlockHash>) -> JsonRpcResult<Option<H256>>;
    #[rpc(name = "refund_getVaultRefundRequests")]
    fn get_vault_refund_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> JsonRpcResult<Vec<H256>>;
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

impl<C, Block, AccountId, H256, RefundRequest> RefundApi<<Block as BlockT>::Hash, AccountId, H256, RefundRequest>
    for Refund<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RefundRuntimeApi<Block, AccountId, H256, RefundRequest>,
    AccountId: Codec,
    H256: Codec,
    RefundRequest: Codec,
{
    fn get_refund_requests(
        &self,
        account_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_refund_requests(&at, account_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch refund requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_refund_requests_by_issue_id(
        &self,
        issue_id: H256,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<Option<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_refund_requests_by_issue_id(&at, issue_id)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to fetch refund requests.".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }

    fn get_vault_refund_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_vault_refund_requests(&at, vault_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch refund requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}
