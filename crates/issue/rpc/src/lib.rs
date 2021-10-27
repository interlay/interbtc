//! RPC interface for the Issue Module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as IssueClient;
pub use module_issue_rpc_runtime_api::IssueApi as IssueRuntimeApi;

#[rpc]
pub trait IssueApi<BlockHash, AccountId, H256, IssueRequest> {
    #[rpc(name = "issue_getIssueRequests")]
    fn get_issue_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;

    #[rpc(name = "issue_getVaultIssueRequests")]
    fn get_vault_issue_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;
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

impl<C, Block, AccountId, H256, IssueRequest> IssueApi<<Block as BlockT>::Hash, AccountId, H256, IssueRequest>
    for Issue<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: IssueRuntimeApi<Block, AccountId, H256, IssueRequest>,
    AccountId: Codec,
    H256: Codec,
    IssueRequest: Codec,
{
    fn get_issue_requests(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_issue_requests(&at, account_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch issue requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_vault_issue_requests(&self, vault_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_vault_issue_requests(&at, vault_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch issue requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}
