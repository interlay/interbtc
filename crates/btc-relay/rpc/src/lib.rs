//! RPC interface for the BtcRelay Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, DispatchError};
use std::sync::Arc;

pub use module_btc_relay_rpc_runtime_api::BtcRelayApi as BtcRelayRuntimeApi;

#[rpc(client, server)]
pub trait BtcRelayApi<BlockHash, H256Le> {
    #[method(name = "btcRelay_verifyBlockHeaderInclusion")]
    fn verify_block_header_inclusion(
        &self,
        block_hash: H256Le,
        at: Option<BlockHash>,
    ) -> RpcResult<Result<(), DispatchError>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`BtcRelayApi`].
pub struct BtcRelay<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> BtcRelay<C, B> {
    /// Create new `BtcRelay` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        BtcRelay {
            client,
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<C, Block, H256Le> BtcRelayApiServer<<Block as BlockT>::Hash, H256Le> for BtcRelay<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: BtcRelayRuntimeApi<Block, H256Le>,
    H256Le: Codec,
{
    fn verify_block_header_inclusion(
        &self,
        block_hash: H256Le,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Result<(), DispatchError>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.verify_block_header_inclusion(&at, block_hash)
            .map_err(|e| internal_err(format!("execution error: Unable to dry run extrinsic {:?}", e)))
    }
}
