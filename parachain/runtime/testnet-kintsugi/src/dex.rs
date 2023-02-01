use super::{
    parameter_types, AccountId, Balance, CurrencyId, DexGeneral, DexStable, Get, PalletId, ParachainInfo, Runtime,
    RuntimeEvent, StablePoolId, Timestamp, Tokens,
};
use frame_support::dispatch::DispatchError;
use orml_traits::MultiCurrency;
use sp_std::marker::PhantomData;
use xcm::latest::prelude::*;

pub use dex_general::{
    AssetBalance, DexGeneralMultiAssets, GenerateLpAssetId, MultiAssetsHandler, PairInfo, LIQUIDITY, LOCAL,
};

pub use dex_stable::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};

parameter_types! {
    pub const DexGeneralPalletId: PalletId = PalletId(*b"dex/genr");
    pub const DexStablePalletId: PalletId = PalletId(*b"dex/stbl");
    pub const StringLimit: u32 = 50;

    // XCM
    pub SelfParaId: u32 = ParachainInfo::get().into();
}

pub struct MultiAssetsAdaptor<Tokens>(PhantomData<Tokens>);

impl<Tokens> MultiAssetsHandler<AccountId, CurrencyId> for MultiAssetsAdaptor<Tokens>
where
    Tokens: MultiCurrency<AccountId, Balance = Balance, CurrencyId = CurrencyId>,
{
    fn balance_of(asset_id: CurrencyId, who: &AccountId) -> AssetBalance {
        Tokens::free_balance(asset_id, who)
    }

    fn total_supply(asset_id: CurrencyId) -> AssetBalance {
        Tokens::total_issuance(asset_id)
    }

    fn is_exists(asset_id: CurrencyId) -> bool {
        Tokens::total_issuance(asset_id) > AssetBalance::default()
    }

    fn deposit(asset_id: CurrencyId, target: &AccountId, amount: AssetBalance) -> Result<AssetBalance, DispatchError> {
        Tokens::deposit(asset_id, target, amount)?;
        Ok(amount)
    }

    fn withdraw(asset_id: CurrencyId, origin: &AccountId, amount: AssetBalance) -> Result<AssetBalance, DispatchError> {
        Tokens::withdraw(asset_id, origin, amount)?;
        Ok(amount)
    }
}

pub struct PairLpIdentity;
impl GenerateLpAssetId<CurrencyId> for PairLpIdentity {
    fn generate_lp_asset_id(asset_0: CurrencyId, asset_1: CurrencyId) -> Option<CurrencyId> {
        CurrencyId::join_lp_token(asset_0, asset_1)
    }
}

impl dex_general::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MultiAssetsHandler = MultiAssetsAdaptor<Tokens>;
    type PalletId = DexGeneralPalletId;
    type AssetId = CurrencyId;
    type LpGenerate = PairLpIdentity;
    // NOTE: XCM not supported
    type SelfParaId = SelfParaId;
    type WeightInfo = ();
}

pub struct PoolLpGenerate;
impl StablePoolLpCurrencyIdGenerate<CurrencyId, StablePoolId> for PoolLpGenerate {
    fn generate_by_pool_id(pool_id: StablePoolId) -> CurrencyId {
        CurrencyId::StableLpToken(pool_id)
    }
}

pub struct StableAmmVerifyPoolAsset;
impl ValidateCurrency<CurrencyId> for StableAmmVerifyPoolAsset {
    fn validate_pooled_currency(_currencies: &[CurrencyId]) -> bool {
        true
    }

    fn validate_pool_lp_currency(currency_id: CurrencyId) -> bool {
        if Tokens::total_issuance(currency_id) > 0 {
            return false;
        }
        true
    }
}

impl dex_stable::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CurrencyId = CurrencyId;
    type MultiCurrency = Tokens;
    type PoolId = StablePoolId;
    type TimeProvider = Timestamp;
    type EnsurePoolAsset = StableAmmVerifyPoolAsset;
    type LpGenerate = PoolLpGenerate;
    type PoolCurrencySymbolLimit = StringLimit;
    type PalletId = DexStablePalletId;
    type WeightInfo = ();
}

impl dex_swap_router::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StablePoolId = StablePoolId;
    type Balance = Balance;
    type StableCurrencyId = CurrencyId;
    type NormalCurrencyId = CurrencyId;
    type NormalAmm = DexGeneral;
    type StableAMM = DexStable;
    type WeightInfo = ();
}
