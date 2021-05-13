//! # Fee Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/fee.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod ext;
pub mod types;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    transactional,
    weights::Weight,
};
use frame_system::ensure_signed;
use sp_arithmetic::{traits::*, FixedPointNumber};
use sp_runtime::{traits::AccountIdConversion, ModuleId};
use sp_std::{convert::TryInto, vec::*};
use types::{Backing, Inner, Issuing, UnsignedFixedPoint, Version};

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config
    + currency::Config<currency::Collateral>
    + currency::Config<currency::Treasury>
    + sla::Config
    + security::Config
{
    /// The fee module id, used for deriving its sovereign account ID.
    type ModuleId: Get<ModuleId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Fee {
        /// # Issue

        /// Fee share that users need to pay to issue tokens.
        IssueFee get(fn issue_fee) config(): UnsignedFixedPoint<T>;

        /// Default griefing collateral (e.g. DOT/KSM) as a percentage of the locked
        /// collateral of a Vault a user has to lock to issue tokens.
        IssueGriefingCollateral get(fn issue_griefing_collateral) config(): UnsignedFixedPoint<T>;

        /// # Redeem

        /// Fee share that users need to pay to redeem tokens.
        RedeemFee get(fn redeem_fee) config(): UnsignedFixedPoint<T>;

        /// # Refund

        /// Fee share that users need to pay to refund overpaid tokens.
        RefundFee get(fn refund_fee) config(): UnsignedFixedPoint<T>;

        /// # Vault Registry

        /// If users execute a redeem with a Vault flagged for premium redeem,
        /// they can earn a collateral premium, slashed from the Vault.
        PremiumRedeemFee get(fn premium_redeem_fee) config(): UnsignedFixedPoint<T>;

        /// Fee that a Vault has to pay if it fails to execute redeem or replace requests
        /// (for redeem, on top of the slashed value of the request). The fee is
        /// paid in collateral based on the token amount at the current exchange rate.
        PunishmentFee get(fn punishment_fee) config(): UnsignedFixedPoint<T>;

        /// # Replace

        /// Default griefing collateral (e.g. DOT/KSM) as a percentage of the to-be-locked collateral
        /// of the new Vault. This collateral will be slashed and allocated to the replacing Vault
        /// if the to-be-replaced Vault does not transfer BTC on time.
        ReplaceGriefingCollateral get(fn replace_griefing_collateral) config(): UnsignedFixedPoint<T>;

        /// AccountId of the fee pool.
        FeePoolAccountId: T::AccountId;

        /// AccountId of the parachain maintainer.
        MaintainerAccountId get(fn maintainer_account_id) config(): T::AccountId;

        /// Number of blocks for reward accrual.
        EpochPeriod get(fn epoch_period) config(): T::BlockNumber;

        /// Total rewards in issued tokens for the current epoch.
        EpochRewardsIssuing get(fn epoch_rewards_issuing): Issuing<T>;

        /// Total rewards in collateral for the current epoch.
        EpochRewardsBacking get(fn epoch_rewards_backing): Backing<T>;

        /// Total rewards in issued tokens locked for accounts.
        TotalRewardsIssuing: map hasher(blake2_128_concat) T::AccountId => Issuing<T>;

        /// Total rewards in collateral locked for accounts.
        TotalRewardsBacking: map hasher(blake2_128_concat) T::AccountId => Backing<T>;

        /// # Parachain Fee Pool Distribution

        /// Percentage of fees allocated to Vaults.
        VaultRewards get(fn vault_rewards) config(): UnsignedFixedPoint<T>;

        /// Vault issued Issuing / total issued Issuing.
        VaultRewardsIssued get(fn vault_rewards_issued) config(): UnsignedFixedPoint<T>;
        /// Vault locked Backing / total locked Backing.
        VaultRewardsLocked get(fn vault_rewards_locked) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees allocated to Staked Relayers.
        RelayerRewards get(fn relayer_rewards) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees allocated for development.
        MaintainerRewards get(fn maintainer_rewards) config(): UnsignedFixedPoint<T>;

        // NOTE: currently there are no collator rewards
        CollatorRewards get(fn collator_rewards) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees generated by nominated collateral that is given to nominators
        NominationRewards get(fn nomination_rewards) config(): UnsignedFixedPoint<T>;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
    add_extra_genesis {
        // don't allow an invalid reward distribution
        build(|config| {
            Pallet::<T>::ensure_rationals_sum_to_one(
                vec![
                    config.vault_rewards,
                    config.relayer_rewards,
                    config.maintainer_rewards,
                    config.collator_rewards,
                ]
            ).unwrap();

            Pallet::<T>::ensure_rationals_sum_to_one(
                vec![
                    config.vault_rewards_issued,
                    config.vault_rewards_locked,
                ]
            ).unwrap();
        })
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Issuing = Issuing<T>,
        Backing = Backing<T>,
    {
        WithdrawIssuing(AccountId, Issuing),
        WithdrawBacking(AccountId, Backing),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        /// The fee module id, used for deriving its sovereign account ID.
        const ModuleId: ModuleId = <T as Config>::ModuleId::get();

        // Initialize errors
        type Error = Error<T>;

        // Initialize events
        fn deposit_event() = default;

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            // TODO: calculate weight
            0
        }

        /// Allows token reward withdrawal if balance is sufficient.
        ///
        /// # Arguments
        ///
        /// * `origin` - signing account
        /// * `amount` - amount of Issuing
        #[weight = <T as Config>::WeightInfo::withdraw_issuing()]
        #[transactional]
        fn withdraw_issuing(origin, #[compact] amount: Issuing<T>) -> DispatchResult
        {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;
            <TotalRewardsIssuing<T>>::insert(signer.clone(), <TotalRewardsIssuing<T>>::get(signer.clone()).checked_sub(&amount).ok_or(Error::<T>::InsufficientFunds)?);
            ext::treasury::transfer::<T>(Self::fee_pool_account_id(), signer.clone(), amount)?;
            Self::deposit_event(<Event<T>>::WithdrawIssuing(
                signer,
                amount,
            ));
            Ok(())
        }

        /// Allows collateral reward withdrawal if balance is sufficient.
        ///
        /// # Arguments
        ///
        /// * `origin` - signing account
        /// * `amount` - amount of collateral
        #[weight = <T as Config>::WeightInfo::withdraw_backing()]
        #[transactional]
        fn withdraw_backing(origin, #[compact] amount: Backing<T>) -> DispatchResult
        {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;
            <TotalRewardsBacking<T>>::insert(signer.clone(), <TotalRewardsBacking<T>>::get(signer.clone()).checked_sub(&amount).ok_or(Error::<T>::InsufficientFunds)?);
            ext::collateral::transfer::<T>(Self::fee_pool_account_id(), signer.clone(), amount)?;
            Self::deposit_event(<Event<T>>::WithdrawBacking(
                signer,
                amount,
            ));
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    fn begin_block(height: T::BlockNumber) -> DispatchResult {
        // only calculate rewards per epoch
        if height % Self::epoch_period() == 0u32.into() {
            Self::update_rewards_for_epoch()?;
        }

        Ok(())
    }

    /// The account ID of the fee pool.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn fee_pool_account_id() -> T::AccountId {
        <T as Config>::ModuleId::get().into_account()
    }

    // Public functions exposed to other pallets

    /// Updates total rewards in tokens and collateral for all participants and
    /// then clears the current epoch rewards.
    pub fn update_rewards_for_epoch() -> DispatchResult {
        // calculate vault rewards
        let (total_vault_rewards_for_issued_in_issuing, total_vault_rewards_for_locked_in_issuing) =
            Self::vault_rewards_for_epoch_in_issuing()?;
        let (total_vault_rewards_for_issued_in_backing, total_vault_rewards_for_locked_in_backing) =
            Self::vault_rewards_for_epoch_in_backing()?;
        for (account, amount_in_issuing, amount_in_backing) in ext::sla::get_vault_rewards::<T>(
            total_vault_rewards_for_issued_in_issuing,
            total_vault_rewards_for_locked_in_issuing,
            total_vault_rewards_for_issued_in_backing,
            total_vault_rewards_for_locked_in_backing,
        )? {
            // TODO: implement fee distribution for the nomination feature. Sketch pseudocode below
            // let mut rich_vault: RichVault<T> =
            //     VaultRegistry::get_active_rich_vault_from_id(&account)?;
            // let vault_reward_proportion = rich_vault.get_vault_collateral_proportion()
            //     + rich_vault.get_nominator_collateral_proportion()
            //         * (1 - Self::nominator_rewards());
            // let nominator_reward_proportion =
            //     rich_vault.get_nominator_collateral_proportion() * (Self::nominator_rewards());

            // 1. Find amount in dot and polkabtc for each nomination party
            // 2. Iterate through nominators, get their proportion and multiply by dot/polkabtc amounts and increase
            // 3. Do the same as 2. for the Vault

            // increase issuing rewards
            <TotalRewardsIssuing<T>>::insert(
                account.clone(),
                <TotalRewardsIssuing<T>>::get(account.clone())
                    .checked_add(&amount_in_issuing)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
            // increase backing rewards
            <TotalRewardsBacking<T>>::insert(
                account.clone(),
                <TotalRewardsBacking<T>>::get(account.clone())
                    .checked_add(&amount_in_backing)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
        }

        // calculate staked relayer rewards
        let total_relayer_rewards_in_issuing = Self::relayer_rewards_for_epoch_in_issuing()?;
        let total_relayer_rewards_in_backing = Self::relayer_rewards_for_epoch_in_backing()?;
        for (account, amount_in_issuing, amount_in_backing) in
            ext::sla::get_relayer_rewards::<T>(total_relayer_rewards_in_issuing, total_relayer_rewards_in_backing)?
        {
            // increase issuing rewards
            <TotalRewardsIssuing<T>>::insert(
                account.clone(),
                <TotalRewardsIssuing<T>>::get(account.clone())
                    .checked_add(&amount_in_issuing)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
            // increase backing rewards
            <TotalRewardsBacking<T>>::insert(
                account.clone(),
                <TotalRewardsBacking<T>>::get(account.clone())
                    .checked_add(&amount_in_backing)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
        }

        // calculate maintainer rewards
        let maintainer_account_id = Self::maintainer_account_id();
        // increase issued rewards
        let total_maintainer_rewards_in_issuing = Self::maintainer_rewards_for_epoch_in_issuing()?;
        <TotalRewardsIssuing<T>>::insert(
            maintainer_account_id.clone(),
            <TotalRewardsIssuing<T>>::get(maintainer_account_id.clone())
                .checked_add(&total_maintainer_rewards_in_issuing)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );
        // increase backing rewards
        let total_maintainer_rewards_in_backing = Self::maintainer_rewards_for_epoch_in_backing()?;
        <TotalRewardsBacking<T>>::insert(
            maintainer_account_id.clone(),
            <TotalRewardsBacking<T>>::get(maintainer_account_id)
                .checked_add(&total_maintainer_rewards_in_backing)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );

        // clear total rewards for current epoch
        <EpochRewardsIssuing<T>>::kill();
        <EpochRewardsBacking<T>>::kill();
        Ok(())
    }

    /// Increase the total amount of tokens generated in this epoch.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount of tokens
    pub fn increase_issuing_rewards_for_epoch(amount: Issuing<T>) {
        <EpochRewardsIssuing<T>>::set(Self::epoch_rewards_issuing() + amount);
    }

    /// Increase the total amount of collateral generated in this epoch.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount of collateral
    pub fn increase_backing_rewards_for_epoch(amount: Backing<T>) {
        <EpochRewardsBacking<T>>::set(Self::epoch_rewards_backing() + amount);
    }

    pub fn get_issuing_rewards(account_id: &T::AccountId) -> Issuing<T> {
        <TotalRewardsIssuing<T>>::get(account_id)
    }

    pub fn get_backing_rewards(account_id: &T::AccountId) -> Backing<T> {
        <TotalRewardsBacking<T>>::get(account_id)
    }

    /// Calculate the required issue fee in tokens.
    ///
    /// # Arguments
    ///
    /// * `amount` - issue amount in tokens
    pub fn get_issue_fee(amount: Issuing<T>) -> Result<Issuing<T>, DispatchError> {
        Self::btc_for(amount, <IssueFee<T>>::get())
    }

    /// Calculate the required issue griefing collateral.
    ///
    /// # Arguments
    ///
    /// * `amount` - issue amount in collateral (at current exchange rate)
    pub fn get_issue_griefing_collateral(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        Self::backing_for(amount, <IssueGriefingCollateral<T>>::get())
    }

    /// Calculate the required redeem fee in tokens. Upon execution, the
    /// rewards should be forwarded to the fee pool instead of being burned.
    ///
    /// # Arguments
    ///
    /// * `amount` - redeem amount in tokens
    pub fn get_redeem_fee(amount: Issuing<T>) -> Result<Issuing<T>, DispatchError> {
        Self::btc_for(amount, <RedeemFee<T>>::get())
    }

    /// Calculate the premium redeem fee in collateral for a user to get if redeeming
    /// with a Vault below the premium redeem threshold.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount in collateral (at current exchange rate)
    pub fn get_premium_redeem_fee(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        Self::backing_for(amount, <PremiumRedeemFee<T>>::get())
    }

    /// Calculate punishment fee for a Vault that fails to execute a redeem
    /// request before the expiry.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount in collateral (at current exchange rate)
    pub fn get_punishment_fee(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        Self::backing_for(amount, <PunishmentFee<T>>::get())
    }

    /// Calculate the required replace griefing collateral.
    ///
    /// # Arguments
    ///
    /// * `amount` - replace amount in collateral (at current exchange rate)
    pub fn get_replace_griefing_collateral(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        Self::backing_for(amount, <ReplaceGriefingCollateral<T>>::get())
    }

    /// Calculate the fee portion of a total amount. For `amount = fee + refund_amount`, this
    /// function returns `fee`.
    ///
    /// # Arguments
    ///
    /// * `amount` - total amount in tokens
    pub fn get_refund_fee_from_total(amount: Issuing<T>) -> Result<Issuing<T>, DispatchError> {
        // calculate 'percentage' = x / (1+x)
        let percentage = <RefundFee<T>>::get()
            .checked_div(
                &<RefundFee<T>>::get()
                    .checked_add(&UnsignedFixedPoint::<T>::one())
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            )
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        Self::btc_for(amount, percentage)
    }

    pub fn btc_for(amount: Issuing<T>, percentage: UnsignedFixedPoint<T>) -> Result<Issuing<T>, DispatchError> {
        Self::inner_to_btc(Self::calculate_for(Self::btc_to_inner(amount)?, percentage)?)
    }

    pub fn backing_for(amount: Backing<T>, percentage: UnsignedFixedPoint<T>) -> Result<Backing<T>, DispatchError> {
        Self::inner_to_backing(Self::calculate_for(Self::backing_to_inner(amount)?, percentage)?)
    }

    // Private functions internal to this pallet

    fn btc_to_inner(x: Issuing<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_btc(x: Inner<T>) -> Result<Issuing<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<Issuing<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn backing_to_inner(x: Backing<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_backing(x: Inner<T>) -> Result<Backing<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<Backing<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    /// Take the `percentage` of an `amount`
    fn calculate_for(amount: Inner<T>, percentage: UnsignedFixedPoint<T>) -> Result<Inner<T>, DispatchError> {
        // we add 0.5 before we do the final integer division to round the result we return.
        // note that unwrapping is safe because we use a constant
        let rounding_addition = UnsignedFixedPoint::<T>::checked_from_rational(1, 2).unwrap();

        UnsignedFixedPoint::<T>::checked_from_integer(amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_mul(&percentage)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_add(&rounding_addition)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .into_inner()
            .checked_div(&UnsignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow.into())
    }

    #[allow(dead_code)]
    /// Helper for validating the `chain_spec` parameters
    fn ensure_rationals_sum_to_one(dist: Vec<UnsignedFixedPoint<T>>) -> DispatchResult {
        let sum = dist.iter().fold(UnsignedFixedPoint::<T>::default(), |a, &b| a + b);
        let one = UnsignedFixedPoint::<T>::checked_from_integer(Pallet::<T>::btc_to_inner(1u32.into())?)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ensure!(sum == one, Error::<T>::InvalidRewardDist);
        Ok(())
    }

    /// Total epoch rewards in issued tokens for vaults
    fn vault_rewards_for_epoch_in_issuing() -> Result<(Issuing<T>, Issuing<T>), DispatchError> {
        let total_vault_rewards = Self::btc_for(Self::epoch_rewards_issuing(), Self::vault_rewards())?;
        Ok((
            Self::btc_for(total_vault_rewards, Self::vault_rewards_issued())?,
            Self::btc_for(total_vault_rewards, Self::vault_rewards_locked())?,
        ))
    }

    /// Total epoch rewards in collateral for vaults
    fn vault_rewards_for_epoch_in_backing() -> Result<(Backing<T>, Backing<T>), DispatchError> {
        let total_vault_rewards = Self::backing_for(Self::epoch_rewards_backing(), Self::vault_rewards())?;
        Ok((
            Self::backing_for(total_vault_rewards, Self::vault_rewards_issued())?,
            Self::backing_for(total_vault_rewards, Self::vault_rewards_locked())?,
        ))
    }

    /// Total epoch rewards in issued tokens for staked relayers
    fn relayer_rewards_for_epoch_in_issuing() -> Result<Issuing<T>, DispatchError> {
        Self::btc_for(Self::epoch_rewards_issuing(), Self::relayer_rewards())
    }

    /// Total epoch rewards in collateral for staked relayers
    fn relayer_rewards_for_epoch_in_backing() -> Result<Backing<T>, DispatchError> {
        Self::backing_for(Self::epoch_rewards_backing(), Self::relayer_rewards())
    }

    /// Total epoch rewards in issued tokens for maintainers
    fn maintainer_rewards_for_epoch_in_issuing() -> Result<Issuing<T>, DispatchError> {
        Self::btc_for(Self::epoch_rewards_issuing(), Self::maintainer_rewards())
    }

    /// Total epoch rewards in collateral for maintainers
    fn maintainer_rewards_for_epoch_in_backing() -> Result<Backing<T>, DispatchError> {
        Self::backing_for(Self::epoch_rewards_backing(), Self::maintainer_rewards())
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        InsufficientFunds,
        InvalidRewardDist,
    }
}
