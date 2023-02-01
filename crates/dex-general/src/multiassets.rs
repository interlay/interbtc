// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

pub trait MultiAssetsHandler<AccountId, AssetId: Copy> {
    fn balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance;

    fn total_supply(asset_id: AssetId) -> AssetBalance;

    fn is_exists(asset_id: AssetId) -> bool;

    fn transfer(asset_id: AssetId, origin: &AccountId, target: &AccountId, amount: AssetBalance) -> DispatchResult {
        let withdrawn = Self::withdraw(asset_id, origin, amount)?;
        let _ = Self::deposit(asset_id, target, withdrawn)?;

        Ok(())
    }

    fn deposit(asset_id: AssetId, target: &AccountId, amount: AssetBalance) -> Result<AssetBalance, DispatchError>;

    fn withdraw(asset_id: AssetId, origin: &AccountId, amount: AssetBalance) -> Result<AssetBalance, DispatchError>;
}

impl<AccountId, AssetId: Copy> MultiAssetsHandler<AccountId, AssetId> for () {
    fn balance_of(_asset_id: AssetId, _who: &AccountId) -> AssetBalance {
        Default::default()
    }

    fn total_supply(_asset_id: AssetId) -> AssetBalance {
        Default::default()
    }

    fn is_exists(_asset_id: AssetId) -> bool {
        false
    }

    fn transfer(_asset_id: AssetId, _origin: &AccountId, _target: &AccountId, _amount: AssetBalance) -> DispatchResult {
        Ok(())
    }

    fn deposit(_asset_id: AssetId, _target: &AccountId, _amount: AssetBalance) -> Result<AssetBalance, DispatchError> {
        Ok(Default::default())
    }

    fn withdraw(_asset_id: AssetId, _origin: &AccountId, _amount: AssetBalance) -> Result<AssetBalance, DispatchError> {
        Ok(Default::default())
    }
}

pub struct DexGeneralMultiAssets<T, Native = (), Local = (), Other = ()>(PhantomData<(T, Native, Local, Other)>);

impl<T: Config<AssetId = AssetId>, NativeCurrency, Local, Other> MultiAssetsHandler<T::AccountId, AssetId>
    for DexGeneralMultiAssets<Pallet<T>, NativeCurrency, Local, Other>
where
    NativeCurrency: Currency<T::AccountId>,
    Local: MultiAssetsHandler<T::AccountId, AssetId>,
    Other: MultiAssetsHandler<T::AccountId, AssetId>,
{
    fn balance_of(asset_id: AssetId, who: &<T as frame_system::Config>::AccountId) -> AssetBalance {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(self_chain_id) => {
                NativeCurrency::free_balance(who).saturated_into::<AssetBalance>()
            }
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => Local::balance_of(asset_id, who),
            RESERVED if asset_id.chain_id == self_chain_id => Other::balance_of(asset_id, who),
            _ if asset_id.is_foreign(self_chain_id) => Pallet::<T>::foreign_balance_of(asset_id, who),
            _ => Default::default(),
        }
    }

    fn total_supply(asset_id: AssetId) -> AssetBalance {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                NativeCurrency::total_issuance().saturated_into::<AssetBalance>()
            }
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => Local::total_supply(asset_id),
            RESERVED if asset_id.chain_id == self_chain_id => Other::total_supply(asset_id),
            _ if asset_id.is_foreign(self_chain_id) => Pallet::<T>::foreign_total_supply(asset_id),
            _ => Default::default(),
        }
    }

    fn is_exists(asset_id: AssetId) -> bool {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.chain_id == self_chain_id => asset_id.is_native(T::SelfParaId::get()),
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => Local::is_exists(asset_id),
            RESERVED if asset_id.chain_id == self_chain_id => Other::is_exists(asset_id),
            _ if asset_id.is_foreign(T::SelfParaId::get()) => Pallet::<T>::foreign_is_exists(asset_id),
            _ => Default::default(),
        }
    }

    fn transfer(
        asset_id: AssetId,
        origin: &<T as frame_system::Config>::AccountId,
        target: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                NativeCurrency::transfer(origin, target, balance_amount, KeepAlive)
            }
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Local::transfer(asset_id, origin, target, amount)
            }
            RESERVED if asset_id.chain_id == self_chain_id => Other::transfer(asset_id, origin, target, amount),
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_transfer(asset_id, origin, target, amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }

    fn deposit(
        asset_id: AssetId,
        target: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                let _ = NativeCurrency::deposit_creating(target, balance_amount);

                Ok(amount)
            }
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => Local::deposit(asset_id, target, amount),
            RESERVED if asset_id.chain_id == self_chain_id => Other::deposit(asset_id, target, amount),
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_mint(asset_id, target, amount).map(|_| amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }

    fn withdraw(
        asset_id: AssetId,
        origin: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(self_chain_id) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                let _ = NativeCurrency::withdraw(
                    origin,
                    balance_amount,
                    WithdrawReasons::TRANSFER,
                    ExistenceRequirement::AllowDeath,
                )?;

                Ok(amount)
            }
            LOCAL | LIQUIDITY if asset_id.chain_id == self_chain_id => Local::withdraw(asset_id, origin, amount),
            RESERVED if asset_id.chain_id == self_chain_id => Other::withdraw(asset_id, origin, amount),
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_burn(asset_id, origin, amount).map(|_| amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }
}
