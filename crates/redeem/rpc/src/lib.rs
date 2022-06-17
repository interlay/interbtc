//! RPC interface for the Redeem Module.

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

pub use module_redeem_rpc_runtime_api::RedeemApi as RedeemRuntimeApi;

#[rpc(client, server)]
pub trait RedeemApi<BlockHash, AccountId, H256, RedeemRequest> {
    #[method(name = "redeem_getRedeemRequests")]
    fn get_redeem_requests(&self, account_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;

    #[method(name = "redeem_getVaultRedeemRequests")]
    fn get_vault_redeem_requests(&self, vault_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<H256>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
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

#[async_trait]
impl<C, Block, AccountId, H256, RedeemRequest> RedeemApiServer<<Block as BlockT>::Hash, AccountId, H256, RedeemRequest>
    for Redeem<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RedeemRuntimeApi<Block, AccountId, H256, RedeemRequest>,
    AccountId: Codec,
    H256: Codec,
    RedeemRequest: Codec,
{
    fn get_redeem_requests(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_redeem_requests(&at, account_id)
            .map_err(|e| internal_err(format!("Unable to fetch redeem requests: {:?}", e)))
    }

    fn get_vault_redeem_requests(
        &self,
        vault_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<H256>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_vault_redeem_requests(&at, vault_id)
            .map_err(|e| internal_err(format!("Unable to fetch redeem requests: {:?}", e)))
    }
}
