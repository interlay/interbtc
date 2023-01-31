// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;
use xcm::v1::{AssetId as XcmAssetId, Fungibility, MultiAssets, WildMultiAsset::All};

impl<T: Config> Pallet<T> {
	// Check the native currency must be more than ExistentialDeposit,
	// other assets always return true
	pub(crate) fn check_existential_deposit(
		asset_id: T::AssetId,
		amount: AssetBalance,
	) -> Option<bool> {
		T::TargetChains::get()
			.iter()
			.find(|(l, _)| *l == T::AssetIdConverter::make_x2_location(&asset_id))
			.map(|&(_, minimum_balance)| amount >= minimum_balance)
	}

	// Make the deposit foreign order
	fn make_deposit_asset_order(recipient: MultiLocation) -> Order<()> {
		Order::DepositAsset {
			assets: All.into(),
			max_assets: u32::max_value(),
			beneficiary: recipient,
		}
	}

	// Transfer zenlink assets which are native to this parachain
	pub(crate) fn make_xcm_lateral_transfer_native(
		location: MultiLocation,
		para_id: ParaId,
		recipient: MultiLocation,
		amount: AssetBalance,
	) -> Xcm<T::RuntimeCall> {
		Xcm::WithdrawAsset {
			assets: MultiAssets::from(MultiAsset::from((
				XcmAssetId::Concrete(location),
				Fungibility::Fungible(amount),
			))),
			effects: vec![Order::DepositReserveAsset {
				assets: All.into(),
				max_assets: u32::max_value(),
				dest: make_x2_location(para_id.into()),
				effects: vec![Self::make_deposit_asset_order(recipient)],
			}],
		}
	}
	// Transfer zenlink assets which are foreign to this parachain
	pub(crate) fn make_xcm_lateral_transfer_foreign(
		reserve_chain: ParaId,
		location: MultiLocation,
		para_id: ParaId,
		recipient: MultiLocation,
		amount: AssetBalance,
	) -> Xcm<T::RuntimeCall> {
		Xcm::WithdrawAsset {
			assets: MultiAssets::from(MultiAsset::from((
				XcmAssetId::Concrete(location),
				Fungibility::Fungible(amount),
			))),
			effects: vec![Order::InitiateReserveWithdraw {
				assets: All.into(),
				reserve: make_x2_location(reserve_chain.into()),
				effects: vec![if para_id == reserve_chain {
					Self::make_deposit_asset_order(recipient)
				} else {
					Order::DepositReserveAsset {
						assets: All.into(),
						max_assets: u32::max_value(),
						dest: make_x2_location(para_id.into()),
						effects: vec![Self::make_deposit_asset_order(recipient)],
					}
				}],
			}],
		}
	}

	pub(crate) fn make_xcm_transfer_to_parachain(
		asset_id: &T::AssetId,
		para_id: ParaId,
		recipient: MultiLocation,
		amount: AssetBalance,
	) -> Result<Xcm<T::RuntimeCall>, XcmError> {
		if !asset_id.is_support() {
			return Err(XcmError::FailedToTransactAsset("Invalid AssetId"))
		}

		let asset_location = T::AssetIdConverter::make_x3_location(asset_id);

		let self_chain_id: u32 = T::SelfParaId::get();
		let asset_chain_id = T::AssetIdConverter::chain_id(asset_id);
		if asset_chain_id == self_chain_id {
			Ok(Self::make_xcm_lateral_transfer_native(asset_location, para_id, recipient, amount))
		} else {
			Ok(Self::make_xcm_lateral_transfer_foreign(
				ParaId::from(asset_chain_id),
				asset_location,
				para_id,
				recipient,
				amount,
			))
		}
	}
}
