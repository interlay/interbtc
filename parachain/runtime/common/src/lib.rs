#![cfg_attr(not(feature = "std"), no_std)]
use core::marker::PhantomData;

use currency::Amount;
use frame_support::{
    pallet_prelude::Get,
    traits::{Currency, OnUnbalanced, TryDrop},
};
use primitives::BlockNumber;
use sp_runtime::{DispatchError, FixedPointNumber};
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
use xcm::latest::{Instruction, MultiLocation, Weight};
use xcm_executor::traits::ShouldExecute;

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
