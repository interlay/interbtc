//! RPC interface for the Relay Pallet.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as RelayClient;
pub use module_relay_rpc_runtime_api::RelayApi as RelayRuntimeApi;

#[rpc]
pub trait RelayApi<BlockHash, VaultId> {
    #[rpc(name = "relay_isTransactionInvalid")]
    fn is_transaction_invalid(&self, vault_id: VaultId, raw_tx: Vec<u8>, at: Option<BlockHash>) -> Result<()>;
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

impl<C, Block, VaultId> RelayApi<<Block as BlockT>::Hash, VaultId> for Relay<C, Block>
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
    ) -> Result<()> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.is_transaction_invalid(&at, vault_id, raw_tx).map_or_else(
            |e| {
                Err(RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to check if transaction is invalid.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
            |result| {
                result.map_err(|e| RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Transaction is valid.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
        )
    }
}
