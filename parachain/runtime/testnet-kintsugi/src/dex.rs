use super::{
    parameter_types, Balance, CurrencyId, DexGeneral, DexStable, PalletId, Runtime, RuntimeEvent, StablePoolId,
    Timestamp, Tokens,
};

pub use dex_general::{AssetBalance, GenerateLpAssetId, PairInfo};
pub use dex_stable::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};

parameter_types! {
    pub const DexGeneralPalletId: PalletId = PalletId(*b"dex/genr");
    pub const DexStablePalletId: PalletId = PalletId(*b"dex/stbl");
    pub const StringLimit: u32 = 50;
    pub const MaxSwaps:u16 = 4;
    pub const MaxBootstrapRewards: u32 = 1000;
    pub const MaxBootstrapLimits:u32 = 1000;
}

pub struct PairLpIdentity;
impl GenerateLpAssetId<CurrencyId> for PairLpIdentity {
    fn generate_lp_asset_id(asset_0: CurrencyId, asset_1: CurrencyId) -> Option<CurrencyId> {
        CurrencyId::join_lp_token(asset_0, asset_1)
    }
}

impl dex_general::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MultiCurrency = Tokens;
    type PalletId = DexGeneralPalletId;
    type AssetId = CurrencyId;
    type LpGenerate = PairLpIdentity;
    type WeightInfo = ();
    type MaxSwaps = MaxSwaps;
    type MaxBootstrapRewards = MaxBootstrapRewards;
    type MaxBootstrapLimits = MaxBootstrapLimits;
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
    type CurrencyId = CurrencyId;
    type NormalAmm = DexGeneral;
    type StableAMM = DexStable;
    type MaxSwaps = MaxSwaps;
    type WeightInfo = ();
}
