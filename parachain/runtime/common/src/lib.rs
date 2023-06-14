#![cfg_attr(not(feature = "std"), no_std)]
use core::marker::PhantomData;

use currency::Amount;
use frame_support::{
    pallet_prelude::Get,
    traits::{Currency, OnTimestampSet, OnUnbalanced, TryDrop},
};
use primitives::{BlockNumber, UnsignedFixedPoint};
use sp_runtime::{DispatchError, FixedPointNumber};
use sp_std::prelude::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

// The relay chain is limited to 12s to include parachain blocks.
pub const MILLISECS_PER_BLOCK: u64 = 12000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
pub const YEARS: BlockNumber = DAYS * 365;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type VaultId<T> = primitives::VaultId<AccountId<T>, currency::CurrencyId<T>>;
pub use currency::CurrencyId;
use primitives::{Balance, Nonce};
use xcm::latest::{Instruction, MultiLocation, Weight};
use xcm_executor::traits::ShouldExecute;

fn native_currency_id<T: currency::Config>() -> CurrencyId<T> {
    T::GetNativeCurrencyId::get()
}

pub fn estimate_escrow_reward_rate<T, EscrowAnnuityInstance, EscrowRewardsApi, EscrowCurrency>(
    account_id: AccountId<T>,
    amount: Option<Balance>,
    lock_time: Option<BlockNumber>,
) -> Result<UnsignedFixedPoint, DispatchError>
where
    T: currency::Config
        + escrow::Config<BlockNumber = BlockNumber, Currency = EscrowCurrency>
        + annuity::Config<EscrowAnnuityInstance, Currency = EscrowCurrency>,
    EscrowAnnuityInstance: 'static,
    EscrowRewardsApi: reward::RewardsApi<(), AccountId<T>, Balance, CurrencyId = CurrencyId<T>>,
    EscrowCurrency: Currency<<T as frame_system::Config>::AccountId, Balance = Balance>,
{
    let native_currency = native_currency_id::<T>();
    // withdraw previous rewards
    EscrowRewardsApi::withdraw_reward(&(), &account_id, native_currency)?;
    // increase amount and/or lock_time
    escrow::Pallet::<T>::round_height_and_deposit_for(
        &account_id,
        amount.unwrap_or_default(),
        lock_time.unwrap_or_default(),
    )?;
    // distribute rewards accrued over block count
    let reward = annuity::Pallet::<T, EscrowAnnuityInstance>::min_reward_per_block().saturating_mul(YEARS.into());
    EscrowRewardsApi::distribute_reward(&(), native_currency, reward)?;
    let received = EscrowRewardsApi::compute_reward(&(), &account_id, native_currency)?;
    // NOTE: total_locked is same currency as rewards
    let total_locked = escrow::Pallet::<T>::locked_balance(&account_id).amount;
    // rate is received / total_locked
    Ok(UnsignedFixedPoint::checked_from_rational(received, total_locked).unwrap_or_default())
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
    VaultAnnuityCurrency: Currency<<T as frame_system::Config>::AccountId, Balance = Balance>,
{
    // distribute and withdraw previous rewards
    let native_currency = native_currency_id::<T>();
    fee::Pallet::<T>::distribute_vault_rewards(&vault_id, native_currency)?;
    // distribute rewards accrued over block count
    VaultStakingApi::withdraw_reward(&(None, vault_id.clone()), &vault_id.account_id, native_currency)?;
    let reward = annuity::Pallet::<T, VaultAnnuityInstance>::min_reward_per_block().saturating_mul(YEARS.into());
    VaultCapacityApi::distribute_reward(&(), native_currency, reward)?;
    Amount::<T>::new(reward, native_currency).mint_to(&fee::Pallet::<T>::fee_pool_account_id())?;
    // calculate the collateral
    let collateral = VaultStakingApi::get_stake(&(None, vault_id.clone()), &vault_id.account_id)?;
    // compute and convert rewards to the same currency
    let received = fee::Pallet::<T>::compute_vault_rewards(&vault_id, &vault_id.account_id, native_currency)?;
    let received_value = received.convert_to(vault_id.collateral_currency())?;
    Ok(UnsignedFixedPoint::checked_from_rational(received_value.amount(), collateral).unwrap_or_default())
}

pub struct AndBarrier<T: ShouldExecute, U: ShouldExecute>(PhantomData<(T, U)>);

impl<T: ShouldExecute, U: ShouldExecute> ShouldExecute for AndBarrier<T, U> {
    fn should_execute<Call>(
        origin: &MultiLocation,
        instructions: &mut [Instruction<Call>],
        max_weight: Weight,
        weight_credit: &mut Weight,
    ) -> Result<(), ()> {
        T::should_execute(origin, instructions, max_weight, weight_credit)?;
        U::should_execute(origin, instructions, max_weight, weight_credit)?;
        // only if both returned ok, we return ok
        Ok(())
    }
}

pub struct Transactless<T: ShouldExecute>(PhantomData<T>);

impl<T: ShouldExecute> ShouldExecute for Transactless<T> {
    fn should_execute<Call>(
        origin: &MultiLocation,
        instructions: &mut [Instruction<Call>],
        max_weight: Weight,
        weight_credit: &mut Weight,
    ) -> Result<(), ()> {
        // filter any outer-level Transacts. Any Transact calls sent to other chain should still work.
        let has_transact = instructions.iter().any(|x| matches!(x, Instruction::Transact { .. }));
        if has_transact {
            return Err(());
        }
        // No transact - return result of the wrapped barrier
        T::should_execute(origin, instructions, max_weight, weight_credit)
    }
}

pub struct ToTreasury<T, TreasuryAccount, NativeCurrency>(PhantomData<(T, TreasuryAccount, NativeCurrency)>);

impl<T, TreasuryAccount, NativeCurrency, NegImbalance> OnUnbalanced<NegImbalance>
    for ToTreasury<T, TreasuryAccount, NativeCurrency>
where
    T: frame_system::Config,
    NativeCurrency: Currency<T::AccountId, NegativeImbalance = NegImbalance>,
    NegImbalance: TryDrop,
    TreasuryAccount: Get<T::AccountId>,
{
    fn on_nonzero_unbalanced(amount: NegImbalance) {
        // Must resolve into existing but better to be safe.
        let _ = NativeCurrency::resolve_creating(&TreasuryAccount::get(), amount);
    }
}

pub struct MaybeSetTimestamp<T>(PhantomData<T>);

impl<T> OnTimestampSet<T::Moment> for MaybeSetTimestamp<T>
where
    T: frame_system::Config + pallet_aura::Config + pallet_sudo::Config,
{
    fn on_timestamp_set(moment: T::Moment) {
        // key is not set on mainnet
        if pallet_sudo::Pallet::<T>::key().is_none() {
            // this hook breaks instant-seal so only call when
            // using the mainnet configuration
            <pallet_aura::Pallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(moment);
        }
    }
}
