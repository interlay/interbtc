use super::{
    parameter_types, AccountId, Balance, CurrencyId, Get, PalletId, ParachainInfo, Runtime, RuntimeEvent, StablePoolId,
    Timestamp, Tokens, ZenlinkProtocol, ZenlinkStableAmm,
};
use frame_support::dispatch::DispatchError;
use orml_traits::MultiCurrency;
use sp_std::{marker::PhantomData, vec, vec::Vec};
use xcm::latest::prelude::*;

use zenlink_protocol::ConvertMultiLocation;
pub use zenlink_protocol::{
    make_x2_location, AssetBalance, GenerateLpAssetId, MultiAssetsHandler, PairInfo, TransactorAdaptor, TrustedParas,
    ZenlinkMultiAssets, LIQUIDITY, LOCAL,
};

pub use zenlink_stable_amm::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};

parameter_types! {
    pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
    pub const StableAmmPalletId: PalletId = PalletId(*b"stbl/amm");
    pub const StringLimit: u32 = 50;

    // XCM
    pub SelfParaId: u32 = ParachainInfo::get().into();
    pub ZenlinkRegisteredParaChains: Vec<(MultiLocation, u128)> = vec![];
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

pub struct NoConvert;
impl<AssetId> ConvertMultiLocation<AssetId> for NoConvert {
    fn chain_id(_asset_id: &AssetId) -> u32 {
        0
    }
    fn make_x3_location(_asset_id: &AssetId) -> MultiLocation {
        Default::default()
    }
}

impl zenlink_protocol::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MultiAssetsHandler = MultiAssetsAdaptor<Tokens>;
    type PalletId = ZenlinkPalletId;
    type AssetId = CurrencyId;
    type LpGenerate = PairLpIdentity;
    // NOTE: XCM not supported
    type XcmExecutor = ();
    type SelfParaId = SelfParaId;
    type TargetChains = ZenlinkRegisteredParaChains;
    type AccountIdConverter = ();
    // no-op since XCM is disabled
    type AssetIdConverter = NoConvert;
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

impl zenlink_stable_amm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CurrencyId = CurrencyId;
    type MultiCurrency = Tokens;
    type PoolId = StablePoolId;
    type TimeProvider = Timestamp;
    type EnsurePoolAsset = StableAmmVerifyPoolAsset;
    type LpGenerate = PoolLpGenerate;
    type PoolCurrencySymbolLimit = StringLimit;
    type PalletId = StableAmmPalletId;
    type WeightInfo = ();
}

impl zenlink_swap_router::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StablePoolId = StablePoolId;
    type Balance = Balance;
    type StableCurrencyId = CurrencyId;
    type NormalCurrencyId = CurrencyId;
    type NormalAmm = ZenlinkProtocol;
    type StableAMM = ZenlinkStableAmm;
    type WeightInfo = ();
}
