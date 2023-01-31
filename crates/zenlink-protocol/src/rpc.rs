// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![allow(clippy::type_complexity)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PairInfo<AccountId, AssetBalance, AssetId> {
	pub asset_0: AssetId,
	pub asset_1: AssetId,

	pub account: AccountId,
	pub total_liquidity: AssetBalance,
	pub holding_liquidity: AssetBalance,
	pub reserve_0: AssetBalance,
	pub reserve_1: AssetBalance,
	pub lp_asset_id: AssetId,
	pub status: u8,
}

impl<T: Config> Pallet<T> {
	pub fn supply_out_amount(supply: AssetBalance, path: Vec<T::AssetId>) -> AssetBalance {
		Self::get_amount_out_by_path(supply, &path).map_or(AssetBalance::default(), |amounts| {
			*amounts.last().unwrap_or(&AssetBalance::default())
		})
	}

	pub fn desired_in_amount(desired_amount: AssetBalance, path: Vec<T::AssetId>) -> AssetBalance {
		Self::get_amount_in_by_path(desired_amount, &path)
			.map_or(AssetBalance::default(), |amounts| {
				*amounts.first().unwrap_or(&AssetBalance::default())
			})
	}

	pub fn get_estimate_lptoken(
		asset_0: T::AssetId,
		asset_1: T::AssetId,
		amount_0_desired: AssetBalance,
		amount_1_desired: AssetBalance,
		amount_0_min: AssetBalance,
		amount_1_min: AssetBalance,
	) -> AssetBalance {
		let sorted_pair = Self::sort_asset_id(asset_0, asset_1);
		match Self::pair_status(sorted_pair) {
			Trading(metadata) => {
				let reserve_0 = T::MultiAssetsHandler::balance_of(asset_0, &metadata.pair_account);
				let reserve_1 = T::MultiAssetsHandler::balance_of(asset_1, &metadata.pair_account);
				Self::calculate_added_amount(
					amount_0_desired,
					amount_1_desired,
					amount_0_min,
					amount_1_min,
					reserve_0,
					reserve_1,
				)
				.map_or(Zero::zero(), |(amount_0, amount_1)| {
					Self::calculate_liquidity(
						amount_0,
						amount_1,
						reserve_0,
						reserve_1,
						metadata.total_supply,
					)
				})
			},
			_ => Zero::zero(),
		}
	}

	pub fn get_pair_by_asset_id(
		asset_0: T::AssetId,
		asset_1: T::AssetId,
	) -> Option<PairInfo<T::AccountId, AssetBalance, T::AssetId>> {
		let pair_account = Self::pair_account_id(asset_0, asset_1);
		let lp_asset_id = Self::lp_asset_id(&asset_0, &asset_1)?;

		let status = match Self::pair_status(Self::sort_asset_id(asset_0, asset_1)) {
			Trading(_) => 0,
			Bootstrap(_) => 1,
			Disable => return None,
		};

		Some(PairInfo {
			asset_0,
			asset_1,
			account: pair_account.clone(),
			total_liquidity: T::MultiAssetsHandler::total_supply(lp_asset_id),
			holding_liquidity: Zero::zero(),
			reserve_0: T::MultiAssetsHandler::balance_of(asset_0, &pair_account),
			reserve_1: T::MultiAssetsHandler::balance_of(asset_1, &pair_account),
			lp_asset_id,
			status,
		})
	}

	pub fn get_sovereigns_info(asset_id: &T::AssetId) -> Vec<(u32, T::AccountId, AssetBalance)> {
		T::TargetChains::get()
			.iter()
			.filter_map(|(location, _)| match location.interior {
				Junctions::X1(Junction::Parachain(id)) => {
					if let Ok(sovereign) = T::AccountIdConverter::convert_ref(location) {
						Some((id, sovereign))
					} else {
						None
					}
				},
				_ => None,
			})
			.map(|(para_id, account)| {
				let balance = T::MultiAssetsHandler::balance_of(*asset_id, &account);

				(para_id, account, balance)
			})
			.collect::<Vec<_>>()
	}
}
