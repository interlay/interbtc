//! RPC interface for the Redeem Module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as RedeemClient;
pub use module_redeem_rpc_runtime_api::RedeemApi as RedeemRuntimeApi;

#[rpc]
pub trait RedeemApi<BlockHash, AccountId, H256, RedeemRequest> {
    #[rpc(name = "redeem_getRedeemRequests")]
    fn get_redeem_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;

    #[rpc(name = "redeem_getVaultRedeemRequests")]
    fn get_vault_redeem_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> Result<Vec<H256>>;
}

/// A struct that implements the [`RedeemApi`].
pub struct Redeem<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Redeem<C, B> {
    /// Create new `Redeem` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Redeem {
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

impl<C, Block, AccountId, H256, RedeemRequest> RedeemApi<<Block as BlockT>::Hash, AccountId, H256, RedeemRequest>
    for Redeem<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RedeemRuntimeApi<Block, AccountId, H256, RedeemRequest>,
    AccountId: Codec,
    H256: Codec,
    RedeemRequest: Codec,
{
    fn get_redeem_requests(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_redeem_requests(&at, account_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch redeem requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_vault_redeem_requests(&self, vault_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_vault_redeem_requests(&at, vault_id).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to fetch redeem requests.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}
