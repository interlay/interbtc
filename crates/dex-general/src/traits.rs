// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

pub trait ValidateAsset<AssetId> {
    fn validate_asset(a: &AssetId) -> bool;
}

impl<AssetId> ValidateAsset<AssetId> for () {
    fn validate_asset(_a: &AssetId) -> bool {
        true
    }
}

pub trait GenerateLpAssetId<AssetId> {
    fn generate_lp_asset_id(asset_0: AssetId, asset_1: AssetId) -> Option<AssetId>;
}

pub trait ExportDexGeneral<AccountId, AssetId> {
    fn get_amount_in_by_path(amount_out: AssetBalance, path: &[AssetId]) -> Result<Vec<AssetBalance>, DispatchError>;

    fn get_amount_out_by_path(amount_in: AssetBalance, path: &[AssetId]) -> Result<Vec<AssetBalance>, DispatchError>;

    fn inner_swap_assets_for_exact_assets(
        who: &AccountId,
        amount_out: AssetBalance,
        amount_in_max: AssetBalance,
        path: &[AssetId],
        recipient: &AccountId,
    ) -> DispatchResult;

    fn inner_swap_exact_assets_for_assets(
        who: &AccountId,
        amount_in: AssetBalance,
        amount_out_min: AssetBalance,
        path: &[AssetId],
        recipient: &AccountId,
    ) -> DispatchResult;

    fn inner_add_liquidity(
        who: &AccountId,
        asset_0: AssetId,
        asset_1: AssetId,
        amount_0_desired: AssetBalance,
        amount_1_desired: AssetBalance,
        amount_0_min: AssetBalance,
        amount_1_min: AssetBalance,
    ) -> DispatchResult;

    fn inner_remove_liquidity(
        who: &AccountId,
        asset_0: AssetId,
        asset_1: AssetId,
        remove_liquidity: AssetBalance,
        amount_0_min: AssetBalance,
        amount_1_min: AssetBalance,
        recipient: &AccountId,
    ) -> DispatchResult;
}
