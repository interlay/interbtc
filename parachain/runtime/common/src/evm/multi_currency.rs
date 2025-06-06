//! ERC20 compatible EVM precompile(s) for interacting with currencies.

use evm_utils::*;
use fp_evm::{ExitError, PrecompileFailure};
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::MaxEncodedLen,
};
use orml_traits::MultiCurrency;
use pallet_evm::{AddressMapping, GasWeightMapping, IsPrecompileResult, PrecompileHandle, PrecompileResult};
use primitives::{is_currency_precompile, CurrencyId, CurrencyInfo, ForeignAssetId, LpToken, StablePoolId};
use sp_core::{Get, H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{marker::PhantomData, prelude::*};
use xcm::VersionedMultiLocation;

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
    fn max_db_read<T: CurrencyApis>() -> usize;
    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>>;
    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>>;
    fn decimals<T: CurrencyApis>(&self) -> Option<u32>;
}

struct ForeignAssetInfo(ForeignAssetId);

impl RuntimeCurrencyInfo for ForeignAssetInfo {
    // NOTE: `name`, `symbol` and `coingecko_id` are not bounded
    // estimate uses 32 bytes each
    fn max_db_read<T: CurrencyApis>() -> usize {
        // ForeignAsset: Twox64Concat(8+4) + AssetMetadata(...)
        // AssetMetadata: 4 + name + symbol + 16 + location + CustomMetadata(16 + coingecko_id)
        144 + VersionedMultiLocation::max_encoded_len()
    }

    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        Some(orml_asset_registry::Metadata::<T>::get(self.0)?.name)
    }

    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        Some(orml_asset_registry::Metadata::<T>::get(self.0)?.symbol)
    }

    fn decimals<T: CurrencyApis>(&self) -> Option<u32> {
        Some(orml_asset_registry::Metadata::<T>::get(self.0)?.decimals)
    }
}

struct StablePoolInfo(StablePoolId);

impl RuntimeCurrencyInfo for StablePoolInfo {
    fn max_db_read<T: CurrencyApis>() -> usize {
        // StableLpToken: Blake2_128(16) + PoolId(4) + Pool(..)
        16 + 4
            + dex_stable::Pool::<
                <T as dex_stable::Config>::PoolId,
                <T as dex_stable::Config>::CurrencyId,
                <T as frame_system::Config>::AccountId,
                <T as dex_stable::Config>::PoolCurrencyLimit,
                <T as dex_stable::Config>::PoolCurrencySymbolLimit,
            >::max_encoded_len()
    }

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
    fn max_db_read<T: CurrencyApis>() -> usize {
        sp_std::cmp::max(
            // ForeignAsset: Twox64Concat(8 + 4) + AssetMetadata(..)
            ForeignAssetInfo::max_db_read::<T>(),
            // StableLpToken: Blake2_128(16) + PoolId(4) + Pool(..)
            StablePoolInfo::max_db_read::<T>(),
        )
    }

    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            LpToken::Token(token) => Some(token.name().as_bytes().to_vec()),
            LpToken::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).name::<T>(),
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).name::<T>(),
        }
    }

    fn symbol<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            LpToken::Token(token) => Some(token.symbol().as_bytes().to_vec()),
            LpToken::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).symbol::<T>(),
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).symbol::<T>(),
        }
    }

    fn decimals<T: CurrencyApis>(&self) -> Option<u32> {
        match self {
            LpToken::Token(token) => Some(token.decimals().into()),
            LpToken::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).decimals::<T>(),
            LpToken::StableLpToken(stable_pool_id) => StablePoolInfo(*stable_pool_id).decimals::<T>(),
        }
    }
}

impl RuntimeCurrencyInfo for CurrencyId {
    fn max_db_read<T: CurrencyApis>() -> usize {
        vec![
            // ForeignAsset: Twox64Concat(8 + 4) + AssetMetadata(..)
            ForeignAssetInfo::max_db_read::<T>(),
            // LendToken: Blake2_128(16) + CurrencyId(11) + CurrencyId(11) + UnderlyingAssetId(..)
            38 + LpToken::max_db_read::<T>(),
            // LpToken: MAX(token0) + MAX(token1)
            LpToken::max_db_read::<T>() + LpToken::max_db_read::<T>(),
            // StableLpToken: Blake2_128(16) + PoolId(4) + Pool(..)
            StablePoolInfo::max_db_read::<T>(),
        ]
        .into_iter()
        .max()
        .unwrap_or_default()
    }

    fn name<T: CurrencyApis>(&self) -> Option<Vec<u8>> {
        match self {
            CurrencyId::Token(token) => Some(token.name().as_bytes().to_vec()),
            CurrencyId::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).name::<T>(),
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
            CurrencyId::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).symbol::<T>(),
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
            CurrencyId::ForeignAsset(foreign_asset_id) => ForeignAssetInfo(*foreign_asset_id).decimals::<T>(),
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
    T::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + From<orml_tokens::Call<T>>,
    <T::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<T::AccountId>>,
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
    T::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + From<orml_tokens::Call<T>>,
    <T::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<T::AccountId>>,
{
    fn execute_inner(handle: &mut impl PrecompileHandle, currency_id: CurrencyId) -> PrecompileResult {
        let input = handle.input();
        let caller = handle.context().caller;
        let caller_account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(caller);

        match Call::new(input)? {
            Call::Name => {
                handle.record_db_read::<T>(CurrencyId::max_db_read::<T>())?;

                Ok(new_precompile_output(EvmString(
                    currency_id.name::<T>().ok_or(RevertReason::ReadFailed)?,
                )))
            }
            Call::Symbol => {
                handle.record_db_read::<T>(CurrencyId::max_db_read::<T>())?;

                Ok(new_precompile_output(EvmString(
                    currency_id.symbol::<T>().ok_or(RevertReason::ReadFailed)?,
                )))
            }
            Call::Decimals => {
                handle.record_db_read::<T>(CurrencyId::max_db_read::<T>())?;

                Ok(new_precompile_output::<U256>(
                    currency_id.decimals::<T>().ok_or(RevertReason::ReadFailed)?.into(),
                ))
            }
            Call::TotalSupply => {
                // TotalIssuance: Twox64Concat(8 + CurrencyId(11)) + CurrencyId(11) + Balance(16)
                handle.record_db_read::<T>(46)?;

                let total_supply = <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::total_issuance(currency_id);
                Ok(new_precompile_output::<U256>(total_supply.into()))
            }
            Call::BalanceOf { account } => {
                // Accounts: Blake2_128(16) + AccountId(32) + Twox64Concat(8 + CurrencyId(11)) + AccountData(..)
                // AccountData: Balance(16) + Balance(16) + Balance(16)
                handle.record_db_read::<T>(115)?;

                let account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(account);
                let balance =
                    <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(currency_id, &account_id);
                Ok(new_precompile_output::<U256>(balance.into()))
            }
            Call::Transfer { recipient, amount } => {
                let recipient_account_id = <T as pallet_evm::Config>::AddressMapping::into_account_id(recipient);

                Self::dispatch_inner(
                    handle,
                    Into::<T::RuntimeCall>::into(orml_tokens::Call::<T>::transfer {
                        dest: T::Lookup::unlookup(recipient_account_id),
                        currency_id,
                        amount: amount.try_into().map_err(|_| RevertReason::ValueIsTooLarge)?,
                    }),
                    caller_account_id,
                )?;

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

    fn dispatch_inner(
        handle: &mut impl PrecompileHandle,
        call: T::RuntimeCall,
        origin: T::AccountId,
    ) -> Result<(), PrecompileFailure> {
        let dispatch_info = call.get_dispatch_info();

        // check there is sufficient gas to execute this call
        let remaining_gas = handle.remaining_gas();
        let required_gas = T::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        if required_gas > remaining_gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
        handle.record_external_cost(
            Some(dispatch_info.weight.ref_time()),
            Some(dispatch_info.weight.proof_size()),
        )?;

        // dispatch call to runtime
        let post_dispatch_info = call.dispatch(Some(origin).into()).map_err(RevertReason::from)?;

        let used_weight = post_dispatch_info.actual_weight.unwrap_or(dispatch_info.weight);
        let used_gas = T::GasWeightMapping::weight_to_gas(used_weight);
        handle.record_cost(used_gas)?;

        // refund weights if call was cheaper
        handle.refund_external_cost(
            Some(dispatch_info.weight.ref_time().saturating_sub(used_weight.ref_time())),
            Some(
                dispatch_info
                    .weight
                    .proof_size()
                    .saturating_sub(used_weight.proof_size()),
            ),
        );

        Ok(())
    }
}

trait WeightHelper: PrecompileHandle {
    fn record_db_read<T: pallet_evm::Config>(&mut self, data_max_encoded_len: usize) -> Result<(), ExitError>;
}

impl<H: PrecompileHandle> WeightHelper for H {
    fn record_db_read<T: pallet_evm::Config>(&mut self, data_max_encoded_len: usize) -> Result<(), ExitError> {
        self.record_cost(T::GasWeightMapping::weight_to_gas(
            <T as frame_system::Config>::DbWeight::get().reads(1),
        ))?;
        // TODO: benchmark precompile to record ref time
        self.record_external_cost(None, Some(data_max_encoded_len as u64))
    }
}
