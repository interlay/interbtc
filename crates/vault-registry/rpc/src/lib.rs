//! RPC interface for the Vault Registry.

pub use self::gen_client::Client as VaultRegistryClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use module_vault_registry_rpc_runtime_api::{
    BalanceWrapper, VaultRegistryApi as VaultRegistryRuntimeApi,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait VaultRegistryApi<BlockHash, AccountId, PolkaBTC, DOT>
where
    PolkaBTC: Codec + MaybeDisplay + MaybeFromStr,
    DOT: Codec + MaybeDisplay + MaybeFromStr,
{
    #[rpc(name = "vaultRegistry_getTotalCollateralization")]
    fn get_total_collateralization(&self, at: Option<BlockHash>) -> Result<u64>;

    #[rpc(name = "vaultRegistry_getFirstVaultWithSufficientCollateral")]
    fn get_first_vault_with_sufficient_collateral(
        &self,
        amount: PolkaBTC,
        at: Option<BlockHash>,
    ) -> Result<AccountId>;

    #[rpc(name = "vaultRegistry_getFirstVaultWithSufficientTokens")]
    fn get_first_vault_with_sufficient_tokens(
        &self,
        amount: PolkaBTC,
        at: Option<BlockHash>,
    ) -> Result<AccountId>;

    #[rpc(name = "vaultRegistry_getIssueableTokensFromVault")]
    fn get_issuable_tokens_from_vault(
        &self,
        vault: AccountId,
        at: Option<BlockHash>,
    ) -> Result<PolkaBTC>;

    #[rpc(name = "vaultRegistry_getCollateralizationFromVault")]
    fn get_collateralization_from_vault(
        &self,
        vault: AccountId,
        at: Option<BlockHash>,
    ) -> Result<u64>;

    #[rpc(name = "vaultRegistry_getRequiredCollateralForPolkabtc")]
    fn get_required_collateral_for_polkabtc(
        &self,
        amount_btc: BalanceWrapper<PolkaBTC>,
        at: Option<BlockHash>,
    ) -> Result<BalanceWrapper<DOT>>;
}

/// A struct that implements the [`VaultRegistryApi`].
pub struct VaultRegistry<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> VaultRegistry<C, B> {
    /// Create new `VaultRegistry` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        VaultRegistry {
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

impl<C, Block, AccountId, PolkaBTC, DOT>
    VaultRegistryApi<<Block as BlockT>::Hash, AccountId, PolkaBTC, DOT> for VaultRegistry<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: VaultRegistryRuntimeApi<Block, AccountId, PolkaBTC, DOT>,
    AccountId: Codec,
    PolkaBTC: Codec + MaybeDisplay + MaybeFromStr,
    DOT: Codec + MaybeDisplay + MaybeFromStr,
{
    fn get_total_collateralization(&self, at: Option<<Block as BlockT>::Hash>) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_total_collateralization(&at).map_or_else(
            |e| {
                Err(RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to get total collateralization.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
            |result| {
                result.map_err(|e| RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to get total collateralization.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
        )
    }

    fn get_first_vault_with_sufficient_collateral(
        &self,
        amount: PolkaBTC,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<AccountId> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_first_vault_with_sufficient_collateral(&at, amount)
            .map_or_else(
                |e| {
                    Err(RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to find a vault with sufficient collateral.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
                |result| {
                    result.map_err(|e| RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to find a vault with sufficient collateral.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
            )
    }
    fn get_first_vault_with_sufficient_tokens(
        &self,
        amount: PolkaBTC,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<AccountId> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_first_vault_with_sufficient_tokens(&at, amount)
            .map_or_else(
                |e| {
                    Err(RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to find a vault with sufficient tokens.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
                |result| {
                    result.map_err(|e| RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to find a vault with sufficient tokens.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
            )
    }
    fn get_issuable_tokens_from_vault(
        &self,
        vault: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<PolkaBTC> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_issuable_tokens_from_vault(&at, vault).map_or_else(
            |e| {
                Err(RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to get issuable tokens from vault.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
            |result| {
                result.map_err(|e| RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to get issuable tokens from vault.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
        )
    }
    fn get_collateralization_from_vault(
        &self,
        vault: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<u64> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.get_collateralization_from_vault(&at, vault)
            .map_or_else(
                |e| {
                    Err(RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to get collateralization from vault.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
                |result| {
                    result.map_err(|e| RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to get collateralization from vault.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
            )
    }

    fn get_required_collateral_for_polkabtc(
        &self,
        amount_btc: BalanceWrapper<PolkaBTC>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BalanceWrapper<DOT>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.get_required_collateral_for_polkabtc(&at, amount_btc)
            .map_or_else(
                |e| {
                    Err(RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to get required collateral for amount.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
                |result| {
                    result.map_err(|e| RpcError {
                        code: ErrorCode::ServerError(Error::RuntimeError.into()),
                        message: "Unable to get required collateral for amount.".into(),
                        data: Some(format!("{:?}", e).into()),
                    })
                },
            )
    }
}
