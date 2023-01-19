#![cfg_attr(not(feature = "std"), no_std)]
use currency::Amount;
use primitives::BlockNumber;
use sp_runtime::{traits::Get as _, DispatchError, FixedPointNumber};
use sp_std::prelude::*;

// The relay chain is limited to 12s to include parachain blocks.
pub const MILLISECS_PER_BLOCK: u64 = 12000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
pub const YEARS: BlockNumber = DAYS * 365;
use primitives::UnsignedFixedPoint;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type VaultId<T> = primitives::VaultId<AccountId<T>, currency::CurrencyId<T>>;
pub use currency::CurrencyId;
use primitives::{Balance, Nonce};

fn native_currency_id<T: currency::Config>() -> CurrencyId<T> {
    T::GetNativeCurrencyId::get()
}

pub fn estimate_vault_reward_rate<T, VaultAnnuityInstance, VaultStakingApi, VaultCapacityApi, VaultAnnuityCurrency>(
    vault_id: VaultId<T>,
) -> Result<UnsignedFixedPoint, DispatchError>
where
    T: oracle::Config
        + currency::Config<UnsignedFixedPoint = UnsignedFixedPoint, Balance = Balance>
        + fee::Config<UnsignedFixedPoint = UnsignedFixedPoint>
        + annuity::Config<VaultAnnuityInstance, Currency = VaultAnnuityCurrency>,
    VaultStakingApi: reward::RewardsApi<(Option<Nonce>, VaultId<T>), AccountId<T>, Balance, CurrencyId = CurrencyId<T>>,
    VaultCapacityApi: reward::RewardsApi<(), CurrencyId<T>, Balance, CurrencyId = CurrencyId<T>>,
    VaultAnnuityInstance: 'static,
    VaultAnnuityCurrency:
        frame_support::traits::tokens::currency::Currency<<T as frame_system::Config>::AccountId, Balance = Balance>,
{
    // distribute and withdraw previous rewards
    let native_currency = native_currency_id::<T>();
    fee::Pallet::<T>::distribute_vault_rewards(&vault_id, native_currency)?;
    // distribute rewards accrued over block count
    VaultStakingApi::withdraw_reward(&(None, vault_id.clone()), &vault_id.account_id, native_currency)?;
    let reward = annuity::Pallet::<T, VaultAnnuityInstance>::min_reward_per_block().saturating_mul(YEARS.into());
    VaultCapacityApi::distribute_reward(&(), native_currency, reward)?;
    Amount::<T>::new(reward, native_currency).mint_to(&fee::Pallet::<T>::fee_pool_account_id())?;
    // compute and convert rewards
    let received = fee::Pallet::<T>::compute_vault_rewards(&vault_id, &vault_id.account_id, native_currency)?;
    let received_as_wrapped = oracle::Pallet::<T>::collateral_to_wrapped(received, native_currency)?;
    // convert collateral stake to same currency
    let collateral = VaultStakingApi::get_stake(&(None, vault_id.clone()), &vault_id.account_id)?;
    let collateral_as_wrapped = oracle::Pallet::<T>::collateral_to_wrapped(collateral, vault_id.collateral_currency())?; // rate is received / collateral
    Ok(UnsignedFixedPoint::checked_from_rational(received_as_wrapped, collateral_as_wrapped).unwrap_or_default())
}
