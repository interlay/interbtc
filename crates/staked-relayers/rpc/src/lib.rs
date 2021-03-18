//! RPC interface for the Staked Relayers.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as StakedRelayersClient;
pub use module_staked_relayers_rpc_runtime_api::StakedRelayersApi as StakedRelayersRuntimeApi;

#[rpc]
pub trait StakedRelayersApi<BlockHash, AccountId> {
    #[rpc(name = "stakedRelayers_isTransactionInvalid")]
    fn is_transaction_invalid(&self, vault_id: AccountId, raw_tx: Vec<u8>, at: Option<BlockHash>) -> Result<()>;
}

/// A struct that implements the [`StakedRelayersApi`].
pub struct StakedRelayers<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> StakedRelayers<C, B> {
    /// Create new `StakedRelayers` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        StakedRelayers {
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

impl<C, Block, AccountId> StakedRelayersApi<<Block as BlockT>::Hash, AccountId> for StakedRelayers<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: StakedRelayersRuntimeApi<Block, AccountId>,
    AccountId: Codec,
{
    fn is_transaction_invalid(
        &self,
        vault_id: AccountId,
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
