//! RPC interface for the BtcRelay Module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, DispatchError};
use std::sync::Arc;

pub use self::gen_client::Client as RefundClient;
pub use module_btc_relay_rpc_runtime_api::BtcRelayApi as BtcRelayRuntimeApi;

#[rpc]
pub trait BtcRelayApi<BlockHash, H256Le> {
    #[rpc(name = "btcRelay_verifyBlockHeaderInclusion")]
    fn verify_block_header_inclusion(
        &self,
        block_hash: H256Le,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<Result<(), DispatchError>>;
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

impl<C, Block, H256Le> BtcRelayApi<<Block as BlockT>::Hash, H256Le> for BtcRelay<C, Block>
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
    ) -> JsonRpcResult<Result<(), DispatchError>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.verify_block_header_inclusion(&at, block_hash)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to verify block header inclusion".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }
}
