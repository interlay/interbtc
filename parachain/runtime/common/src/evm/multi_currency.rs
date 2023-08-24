//! ERC20 compatible EVM precompile(s) for interacting with currencies.

use evm_utils::*;
use orml_traits::MultiCurrency;
use pallet_evm::{AddressMapping, IsPrecompileResult, PrecompileHandle, PrecompileResult};
use primitives::{is_currency_precompile, CurrencyId, CurrencyInfo, ForeignAssetId, LpToken, StablePoolId};
use sp_core::{H160, U256};
use sp_std::{marker::PhantomData, prelude::*};

pub trait CurrencyApis:
    orml_asset_registry::Config<AssetId = ForeignAssetId>
    + loans::Config<CurrencyId = CurrencyId>
    + dex_stable::Config<CurrencyId = CurrencyId, PoolId = StablePoolId>
{
}

impl<Api> CurrencyApis for Api where
    Api: orml_asset_registry::Config<AssetId = ForeignAssetId>
        + loans::Config<CurrencyId = CurrencyId>
        + dex_stable::Config<CurrencyId = CurrencyId, PoolId = StablePoolId>
{
}

pub trait RuntimeCurrencyInfo {
    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>>;
    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>>;
    fn decimals<T: CurrencyApis>(&self) -> Option<u32>;
}

struct StablePoolInfo(StablePoolId);

impl RuntimeCurrencyInfo for StablePoolInfo {
    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        let pool = dex_stable::Pools::<T>::get(self.0)?;
        let mut vec = Vec::new();
        vec.extend_from_slice(&b"LP "[..]);
        vec.extend_from_slice(
            &pool
                .get_currency_ids()
                .into_iter()
                .map(|currency_id| currency_id.name::<T>())
                .collect::<Option<Vec<_>>>()?
                .join(&b" - "[..]),
        );
        Some(vec)
    }

    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        dex_stable::Pools::<T>::get(self.0).map(|pool| pool.info().lp_currency_symbol.to_vec())
    }

    fn decimals<T: CurrencyApis>(&self) -> Option<u32> {
        dex_stable::Pools::<T>::get(self.0).map(|pool| pool.info().lp_currency_decimal.into())
    }
}

impl RuntimeCurrencyInfo for LpToken {
    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            LpToken::Token(token) => Some(token.name().as_bytes().to_vec()),
            LpToken::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.name)
            }
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).name::<T>(),
        }
    }

    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            LpToken::Token(token) => Some(token.symbol().as_bytes().to_vec()),
            LpToken::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.symbol)
            }
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).symbol::<T>(),
        }
    }

    fn decimals<T: CurrencyApis>(&self) -> Option<u32> {
        match self {
            LpToken::Token(token) => Some(token.decimals().into()),
            LpToken::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.decimals)
            }
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).decimals::<T>(),
        }
    }
}

impl RuntimeCurrencyInfo for CurrencyId {
    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            CurrencyId::Token(token) => Some(token.name().as_bytes().to_vec()),
            CurrencyId::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.name)
            }
            CurrencyId::LendToken(_) => {
                let mut vec = Vec::new();
                vec.extend_from_slice(&b"q"[..]);
                vec.extend_from_slice(&loans::UnderlyingAssetId::<T>::get(self)?.name::<T>()?);
                Some(vec)
            }
            CurrencyId::LpToken(token_0, token_1) => {
                let mut vec = Vec::new();
                vec.extend_from_slice(&b"LP "[..]);
                vec.extend_from_slice(&token_0.name::<T>()?);
                vec.extend_from_slice(&b" - "[..]);
                vec.extend_from_slice(&token_1.name::<T>()?);
                Some(vec)
            }
            CurrencyId::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).name::<T>(),
        }
    }

    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            CurrencyId::Token(token) => Some(token.symbol().as_bytes().to_vec()),
            CurrencyId::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.symbol)
            }
            CurrencyId::LendToken(_) => {
                let mut vec = Vec::new();
                vec.extend_from_slice(&b"Q"[..]);
                vec.extend_from_slice(&loans::UnderlyingAssetId::<T>::get(self)?.symbol::<T>()?);
                Some(vec)
            }
            CurrencyId::LpToken(token_0, token_1) => {
                let mut vec = Vec::new();
                vec.extend_from_slice(&b"LP_"[..]);
                vec.extend_from_slice(&token_0.symbol::<T>()?);
                vec.extend_from_slice(&b"_"[..]);
                vec.extend_from_slice(&token_1.symbol::<T>()?);
                Some(vec)
            }
            CurrencyId::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).symbol::<T>(),
        }
    }

    fn decimals<T: CurrencyApis>(&self) -> Option<u32> {
        match self {
            CurrencyId::Token(token) => Some(token.decimals().into()),
            CurrencyId::ForeignAsset(foreign_asset_id) => {
                Some(orml_asset_registry::Metadata::<T>::get(foreign_asset_id)?.decimals)
            }
            CurrencyId::LendToken(_) => loans::UnderlyingAssetId::<T>::get(self)?.decimals::<T>(),
            CurrencyId::LpToken(_, _) => Some(18u32),
            CurrencyId::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).decimals::<T>(),
        }
    }
}

#[allow(unused)]
#[derive(Debug, evm_macro::EvmCall)]
enum Call {
    #[selector = "name()"]
    Name,
    #[selector = "symbol()"]
    Symbol,
    #[selector = "decimals()"]
    Decimals,
    #[selector = "totalSupply()"]
    TotalSupply,
    #[selector = "balanceOf(address)"]
    BalanceOf { account: H160 },
    #[selector = "transfer(address,uint256)"]
    Transfer { recipient: H160, amount: U256 },
    #[selector = "allowance(address,address)"]
    Allowance { owner: H160, spender: H160 },
    #[selector = "approve(address,uint256)"]
    Approve { spender: H160, amount: U256 },
    #[selector = "transferFrom(address,address,uint256)"]
    TransferFrom {
        sender: H160,
        recipient: H160,
        amount: U256,
    },
}

#[allow(unused)]
#[derive(evm_macro::EvmEvent)]
enum Event {
    #[selector = "Transfer(address,address,uint256)"]
    Transfer {
        #[indexed]
        from: H160,
        #[indexed]
        to: H160,
        value: U256,
    },
    #[selector = "Approval(address,address,uint256)"]
    Approval {
        #[indexed]
        owner: H160,
        #[indexed]
        spender: H160,
        value: U256,
    },
}

pub struct MultiCurrencyPrecompiles<T>(PhantomData<T>);

impl<T> PartialPrecompileSet for MultiCurrencyPrecompiles<T>
where
    T: CurrencyApis + currency::Config + pallet_evm::Config,
{
    fn new() -> Self {
        Self(Default::default())
    }

    fn execute<R: pallet_evm::Config>(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        let currency_id = match CurrencyId::try_from(handle.context().address) {
            Ok(currency_id) => currency_id,
            // not precompile address or other decoding error
            _ => return None,
        };

        match Self::execute_inner(handle, currency_id) {
            Ok(output) => Some(Ok(output)),
            Err(failure) => Some(Err(failure)),
        }
    }

    fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: is_currency_precompile(&address),
            extra_cost: 0,
        }
    }

    fn used_addresses(&self) -> Vec<H160> {
        // we can't know this ahead of time
        vec![]
    }
}

impl<T> MultiCurrencyPrecompiles<T>
where
    T: CurrencyApis + currency::Config + pallet_evm::Config,
{
    fn execute_inner(handle: &mut impl PrecompileHandle, currency_id: CurrencyId) -> PrecompileResult {
        let input = handle.input();
        let caller = handle.context().caller;
        let caller_account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(caller);

        match Call::new(input)? {
            Call::Name => Ok(new_precompile_output(EvmString(
                currency_id.name::<T>().ok_or(RevertReason::ReadFailed)?,
            ))),
            Call::Symbol => Ok(new_precompile_output(EvmString(
                currency_id.symbol::<T>().ok_or(RevertReason::ReadFailed)?,
            ))),
            Call::Decimals => Ok(new_precompile_output(Into::<U256>::into(
                currency_id.decimals::<T>().ok_or(RevertReason::ReadFailed)?,
            ))),
            Call::TotalSupply => {
                let total_supply = <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::total_issuance(currency_id);
                Ok(new_precompile_output(Into::<U256>::into(total_supply)))
            }
            Call::BalanceOf { account } => {
                let account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(account);
                let balance =
                    <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(currency_id, &account_id);
                Ok(new_precompile_output(Into::<U256>::into(balance)))
            }
            Call::Transfer { recipient, amount } => {
                let recipient_account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(recipient);

                <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
                    currency_id,
                    &caller_account_id,
                    &recipient_account_id,
                    amount.try_into().map_err(|_| RevertReason::ValueIsTooLarge)?,
                )
                .map_err(RevertReason::from)?;

                Event::Transfer {
                    from: caller,
                    to: recipient,
                    value: amount,
                }
                .log(handle)?;

                Ok(new_precompile_output(true))
            }
            Call::Allowance { .. } => Err(RevertReason::NotSupported.into()),
            Call::Approve { .. } => Err(RevertReason::NotSupported.into()),
            Call::TransferFrom { .. } => Err(RevertReason::NotSupported.into()),
        }
    }
}
