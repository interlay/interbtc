//! RPC interface for the Relay Pallet.

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

pub use module_relay_rpc_runtime_api::RelayApi as RelayRuntimeApi;

#[rpc(client, server)]
pub trait RelayApi<BlockHash, VaultId> {
    #[method(name = "relay_isTransactionInvalid")]
    fn is_transaction_invalid(&self, vault_id: VaultId, raw_tx: Vec<u8>, at: Option<BlockHash>) -> RpcResult<()>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`RelayApi`].
pub struct Relay<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Relay<C, B> {
    /// Create new `Relay` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Relay {
            client,
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<C, Block, VaultId> RelayApiServer<<Block as BlockT>::Hash, VaultId> for Relay<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RelayRuntimeApi<Block, VaultId>,
    VaultId: Codec,
{
    fn is_transaction_invalid(
        &self,
        vault_id: VaultId,
        raw_tx: Vec<u8>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<()> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.is_transaction_invalid(&at, vault_id, raw_tx)
            .map_err(|err| internal_err(format!("Runtime error: {:?}", err)))?
            .map_err(|err| internal_err(format!("Transaction is valid: {:?}", err)))
    }
}
