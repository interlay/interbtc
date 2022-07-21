//! RPC interface for the Vault Registry.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use module_oracle_rpc_runtime_api::BalanceWrapper;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

pub use module_vault_registry_rpc_runtime_api::VaultRegistryApi as VaultRegistryRuntimeApi;

#[rpc(client, server)]
pub trait VaultRegistryApi<BlockHash, VaultId, Balance, UnsignedFixedPoint, CurrencyId, AccountId>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    UnsignedFixedPoint: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
    AccountId: Codec,
{
    #[method(name = "vaultRegistry_getVaultCollateral")]
    fn get_vault_collateral(&self, vault_id: VaultId, at: Option<BlockHash>) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "vaultRegistry_getVaultsByAccountId")]
    fn get_vaults_by_account_id(&self, account_id: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<VaultId>>;

    #[method(name = "vaultRegistry_getVaultTotalCollateral")]
    fn get_vault_total_collateral(
        &self,
        vault_id: VaultId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "vaultRegistry_getPremiumRedeemVaults")]
    fn get_premium_redeem_vaults(&self, at: Option<BlockHash>) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>>;

    #[method(name = "vaultRegistry_getVaultsWithIssuableTokens")]
    fn get_vaults_with_issuable_tokens(
        &self,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>>;

    #[method(name = "vaultRegistry_getVaultsWithRedeemableTokens")]
    fn get_vaults_with_redeemable_tokens(
        &self,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>>;

    #[method(name = "vaultRegistry_getIssueableTokensFromVault")]
    fn get_issuable_tokens_from_vault(
        &self,
        vault: VaultId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "vaultRegistry_getCollateralizationFromVault")]
    fn get_collateralization_from_vault(
        &self,
        vault: VaultId,
        only_issued: bool,
        at: Option<BlockHash>,
    ) -> RpcResult<UnsignedFixedPoint>;

    #[method(name = "vaultRegistry_getCollateralizationFromVaultAndCollateral")]
    fn get_collateralization_from_vault_and_collateral(
        &self,
        vault: VaultId,
        collateral: BalanceWrapper<Balance>,
        only_issued: bool,
        at: Option<BlockHash>,
    ) -> RpcResult<UnsignedFixedPoint>;

    #[method(name = "vaultRegistry_getRequiredCollateralForWrapped")]
    fn get_required_collateral_for_wrapped(
        &self,
        amount_btc: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "vaultRegistry_getRequiredCollateralForVault")]
    fn get_required_collateral_for_vault(
        &self,
        vault_id: VaultId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
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

fn handle_response<T, E: std::fmt::Debug>(result: Result<Result<T, DispatchError>, E>, msg: String) -> RpcResult<T> {
    result
        .map_err(|err| internal_err(format!("Runtime error: {:?}: {:?}", msg, err)))?
        .map_err(|err| internal_err(format!("Execution error: {:?}: {:?}", msg, err)))
}

#[async_trait]
impl<C, Block, VaultId, Balance, UnsignedFixedPoint, CurrencyId, AccountId>
    VaultRegistryApiServer<<Block as BlockT>::Hash, VaultId, Balance, UnsignedFixedPoint, CurrencyId, AccountId>
    for VaultRegistry<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: VaultRegistryRuntimeApi<Block, VaultId, Balance, UnsignedFixedPoint, CurrencyId, AccountId>,
    VaultId: Codec,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    UnsignedFixedPoint: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
    AccountId: Codec,
{
    fn get_vault_collateral(
        &self,
        vault_id: VaultId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_vault_collateral(&at, vault_id),
            "Unable to get the vault's collateral".into(),
        )
    }

    fn get_vaults_by_account_id(
        &self,
        account_id: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<VaultId>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_vaults_by_account_id(&at, account_id),
            "Unable to get vault ids".into(),
        )
    }

    fn get_vault_total_collateral(
        &self,
        vault_id: VaultId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_vault_total_collateral(&at, vault_id),
            "Unable to get the vault's collateral".into(),
        )
    }

    fn get_premium_redeem_vaults(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_premium_redeem_vaults(&at),
            "Unable to find a vault below the premium redeem threshold".into(),
        )
    }

    fn get_vaults_with_issuable_tokens(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_vaults_with_issuable_tokens(&at),
            "Unable to find a vault with issuable tokens".into(),
        )
    }

    fn get_vaults_with_redeemable_tokens(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<(VaultId, BalanceWrapper<Balance>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_vaults_with_redeemable_tokens(&at),
            "Unable to find a vault with redeemable tokens".into(),
        )
    }

    fn get_issuable_tokens_from_vault(
        &self,
        vault: VaultId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_issuable_tokens_from_vault(&at, vault),
            "Unable to get issuable tokens from vault".into(),
        )
    }

    fn get_collateralization_from_vault(
        &self,
        vault: VaultId,
        only_issued: bool,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<UnsignedFixedPoint> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_collateralization_from_vault(&at, vault, only_issued),
            "Unable to get collateralization from vault".into(),
        )
    }

    fn get_collateralization_from_vault_and_collateral(
        &self,
        vault: VaultId,
        collateral: BalanceWrapper<Balance>,
        only_issued: bool,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<UnsignedFixedPoint> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_collateralization_from_vault_and_collateral(&at, vault, collateral, only_issued),
            "Unable to get collateralization from vault".into(),
        )
    }

    fn get_required_collateral_for_wrapped(
        &self,
        amount_btc: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.get_required_collateral_for_wrapped(&at, amount_btc, currency_id),
            "Unable to get required collateral for amount".into(),
        )
    }

    fn get_required_collateral_for_vault(
        &self,
        vault_id: VaultId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        handle_response(
            api.get_required_collateral_for_vault(&at, vault_id),
            "Unable to get required collateral for vault".into(),
        )
    }
}
