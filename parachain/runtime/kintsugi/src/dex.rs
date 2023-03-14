use super::{
    parameter_types, Balance, CurrencyId, DexGeneral, DexStable, DispatchResult, Farming, Get, Loans, OnRuntimeUpgrade,
    PalletId, Rate, Ratio, Runtime, RuntimeEvent, RuntimeOrigin, StablePoolId, Timestamp, Tokens, Vec, Weight, KBTC,
    KINT, KSM,
};
use sp_runtime::{traits::Zero, FixedPointNumber};

#[cfg(feature = "try-runtime")]
use frame_support::ensure;

pub use dex_general::{AssetBalance, GenerateLpAssetId, PairInfo};
pub use dex_stable::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};

parameter_types! {
    pub const DexGeneralPalletId: PalletId = PalletId(*b"dex/genr");
    pub const DexStablePalletId: PalletId = PalletId(*b"dex/stbl");
    pub const StringLimit: u32 = 50;
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
    type WeightInfo = ();
}

pub struct DexSetup;

fn create_pair_and_update_rewards(asset_0: CurrencyId, asset_1: CurrencyId, rewards: Balance, fee_rate: Balance) {
    if let Err(err) = DexGeneral::create_pair(RuntimeOrigin::root(), asset_0, asset_1, fee_rate) {
        log::error!("Could not create pair: {:?}", err);
    } else {
        if let Err(err) = Farming::update_reward_schedule(
            RuntimeOrigin::root(),
            CurrencyId::join_lp_token(asset_0, asset_1).expect("currencies are valid; qed"),
            CurrencyId::Token(KINT),
            // period is 5 (blocks per minute)
            // so period count for three months
            // is 60 minutes * 24 hours * 92 days
            132480,
            rewards,
        ) {
            log::error!("Could not update rewards: {:?}", err);
        }
    }
}

impl OnRuntimeUpgrade for DexSetup {
    fn on_runtime_upgrade() -> Weight {
        create_pair_and_update_rewards(
            CurrencyId::Token(KBTC),
            CurrencyId::Token(KSM),
            45000000000000000, // 45,000 KINT
            15,                // 0.15%
        );

        create_pair_and_update_rewards(
            CurrencyId::Token(KBTC),
            CurrencyId::ForeignAsset(3), // USDT
            40000000000000000,           // 40,000 KINT
            15,                          // 0.15%
        );

        create_pair_and_update_rewards(
            CurrencyId::Token(KSM),
            CurrencyId::Token(KINT),
            35000000000000000, // 35,000 KINT
            25,                // 0.25%
        );

        Default::default()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        ensure!(
            dex_general::PairStatuses::<Runtime>::iter().collect::<Vec<_>>().len() == 0,
            "Should not have pools"
        );
        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
        ensure!(
            dex_general::PairStatuses::<Runtime>::iter().collect::<Vec<_>>().len() == 3,
            "Should have pools"
        );
        Ok(())
    }
}

pub struct LoansSetup;

fn add_and_activate_market(token: CurrencyId, market: loans::Market<Balance>) {
    if let Err(err) = Loans::add_market(RuntimeOrigin::root(), token, market) {
        log::error!("Could not add market: {:?}", err);
    } else {
        if let Err(err) = Loans::activate_market(RuntimeOrigin::root(), token) {
            log::error!("Could not activate market: {:?}", err);
        }
    }
}

impl OnRuntimeUpgrade for LoansSetup {
    fn on_runtime_upgrade() -> Weight {
        add_and_activate_market(
            CurrencyId::Token(KBTC),
            loans::Market {
                close_factor: Ratio::from_percent(50),
                collateral_factor: Ratio::from_percent(63),
                liquidation_threshold: Ratio::from_percent(67),
                liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
                liquidate_incentive_reserved_factor: Ratio::zero(),
                state: loans::MarketState::Pending,
                rate_model: loans::InterestRateModel::Jump(loans::JumpModel {
                    base_rate: Rate::zero(),
                    jump_rate: Rate::from_inner(Rate::DIV / 100 * 5),
                    full_rate: Rate::from_inner(Rate::DIV / 100 * 50),
                    jump_utilization: Ratio::from_percent(90),
                }),
                reserve_factor: Ratio::from_percent(20),
                supply_cap: 2000000000, // 20 KBTC
                borrow_cap: 2000000000, // 20 KBTC
                lend_token_id: CurrencyId::LendToken(1),
            },
        );

        add_and_activate_market(
            CurrencyId::Token(KSM),
            loans::Market {
                close_factor: Ratio::from_percent(50),
                collateral_factor: Ratio::from_percent(54),
                liquidation_threshold: Ratio::from_percent(61),
                liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
                liquidate_incentive_reserved_factor: Ratio::zero(),
                state: loans::MarketState::Pending,
                rate_model: loans::InterestRateModel::Jump(loans::JumpModel {
                    base_rate: Rate::zero(),
                    jump_rate: Rate::from_inner(Rate::DIV / 100 * 15),
                    full_rate: Rate::from_inner(Rate::DIV / 100 * 40),
                    jump_utilization: Ratio::from_percent(90),
                }),
                reserve_factor: Ratio::from_percent(20),
                supply_cap: 30000000000000000, // 30,000 KSM
                borrow_cap: 30000000000000000, // 30,000 KSM
                lend_token_id: CurrencyId::LendToken(2),
            },
        );

        add_and_activate_market(
            CurrencyId::ForeignAsset(3), // USDT
            loans::Market {
                close_factor: Ratio::from_percent(50),
                collateral_factor: Ratio::from_percent(65),
                liquidation_threshold: Ratio::from_percent(69),
                liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
                liquidate_incentive_reserved_factor: Ratio::zero(),
                state: loans::MarketState::Pending,
                rate_model: loans::InterestRateModel::Jump(loans::JumpModel {
                    base_rate: Rate::zero(),
                    jump_rate: Rate::from_inner(Rate::DIV / 100 * 15),
                    full_rate: Rate::from_inner(Rate::DIV / 100 * 40),
                    jump_utilization: Ratio::from_percent(90),
                }),
                reserve_factor: Ratio::from_percent(20),
                supply_cap: 800000000000, // 800,000 USDT
                borrow_cap: 800000000000, // 800,000 USDT
                lend_token_id: CurrencyId::LendToken(3),
            },
        );

        Default::default()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        ensure!(
            loans::Markets::<Runtime>::iter().collect::<Vec<_>>().len() == 0,
            "Should not have markets"
        );
        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
        ensure!(
            loans::Markets::<Runtime>::iter().collect::<Vec<_>>().len() == 3,
            "Should have markets"
        );
        Ok(())
    }
}
