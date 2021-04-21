//! # PolkaBTC Fee Module
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
    ensure, transactional,
    weights::Weight,
};
use frame_system::ensure_signed;
use sp_arithmetic::{traits::*, FixedPointNumber};
use sp_std::{convert::TryInto, vec::*};
use types::{Inner, PolkaBTC, UnsignedFixedPoint, DOT};

/// The pallet's configuration trait.
pub trait Config:
    frame_system::Config + collateral::Config + treasury::Config + sla::Config + security::Config
{
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

        /// Fee share that users need to pay to issue PolkaBTC.
        IssueFee get(fn issue_fee) config(): UnsignedFixedPoint<T>;

        /// Default griefing collateral (in DOT) as a percentage of the locked
        /// collateral of a Vault a user has to lock to issue PolkaBTC.
        IssueGriefingCollateral get(fn issue_griefing_collateral) config(): UnsignedFixedPoint<T>;

        /// # Redeem

        /// Fee share that users need to pay to redeem PolkaBTC.
        RedeemFee get(fn redeem_fee) config(): UnsignedFixedPoint<T>;

        /// # Refund

        /// Fee share that users need to pay to refund overpaid PolkaBTC.
        RefundFee get(fn refund_fee) config(): UnsignedFixedPoint<T>;

        /// # Vault Registry

        /// If users execute a redeem with a Vault flagged for premium redeem,
        /// they can earn a DOT premium, slashed from the Vault's collateral.
        PremiumRedeemFee get(fn premium_redeem_fee) config(): UnsignedFixedPoint<T>;

        /// Fee paid to Vaults to auction / force-replace undercollateralized Vaults.
        /// This is slashed from the replaced Vault's collateral.
        AuctionRedeemFee get(fn auction_redeem_fee) config(): UnsignedFixedPoint<T>;

        /// Fee that a Vault has to pay if it fails to execute redeem or replace requests
        /// (for redeem, on top of the slashed BTC-in-DOT value of the request). The fee is
        /// paid in DOT based on the PolkaBTC amount at the current exchange rate.
        PunishmentFee get(fn punishment_fee) config(): UnsignedFixedPoint<T>;

        /// # Replace

        /// Default griefing collateral (in DOT) as a percentage of the to-be-locked DOT collateral
        /// of the new Vault. This collateral will be slashed and allocated to the replacing Vault
        /// if the to-be-replaced Vault does not transfer BTC on time.
        ReplaceGriefingCollateral get(fn replace_griefing_collateral) config(): UnsignedFixedPoint<T>;

        /// AccountId of the fee pool.
        FeePoolAccountId get(fn fee_pool_account_id) config(): T::AccountId;

        /// AccountId of the parachain maintainer.
        MaintainerAccountId get(fn maintainer_account_id) config(): T::AccountId;

        /// Number of blocks for reward accrual.
        EpochPeriod get(fn epoch_period) config(): T::BlockNumber;

        /// Total rewards in `PolkaBTC` for the current epoch.
        EpochRewardsPolkaBTC get(fn epoch_rewards_polka_btc): PolkaBTC<T>;

        /// Total rewards in `DOT` for the current epoch.
        EpochRewardsDOT get(fn epoch_rewards_dot): DOT<T>;

        /// Total rewards in `PolkaBTC` locked for accounts.
        TotalRewardsPolkaBTC: map hasher(blake2_128_concat) T::AccountId => PolkaBTC<T>;

        /// Total rewards in `DOT` locked for accounts.
        TotalRewardsDOT: map hasher(blake2_128_concat) T::AccountId => DOT<T>;

        /// # Parachain Fee Pool Distribution

        /// Percentage of fees allocated to Vaults.
        VaultRewards get(fn vault_rewards) config(): UnsignedFixedPoint<T>;

        /// Vault issued PolkaBTC / total issued PolkaBTC.
        VaultRewardsIssued get(fn vault_rewards_issued) config(): UnsignedFixedPoint<T>;
        /// Vault locked DOT / total locked DOT.
        VaultRewardsLocked get(fn vault_rewards_locked) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees allocated to Staked Relayers.
        RelayerRewards get(fn relayer_rewards) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees allocated for development.
        MaintainerRewards get(fn maintainer_rewards) config(): UnsignedFixedPoint<T>;

        // NOTE: currently there are no collator rewards
        CollatorRewards get(fn collator_rewards) config(): UnsignedFixedPoint<T>;

        /// Percentage of fees generated by nominated collateral that is given to nominators
        NominationRewards get(fn nomination_rewards) config(): UnsignedFixedPoint<T>;
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
        PolkaBTC = PolkaBTC<T>,
        DOT = DOT<T>,
    {
        WithdrawPolkaBTC(AccountId, PolkaBTC),
        WithdrawDOT(AccountId, DOT),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
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

        /// Allows PolkaBTC reward withdrawal if balance is sufficient.
        ///
        /// # Arguments
        ///
        /// * `origin` - signing account
        /// * `amount` - amount of PolkaBTC
        #[weight = <T as Config>::WeightInfo::withdraw_polka_btc()]
        #[transactional]
        fn withdraw_polka_btc(origin, amount: PolkaBTC<T>) -> DispatchResult
        {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;
            <TotalRewardsPolkaBTC<T>>::insert(signer.clone(), <TotalRewardsPolkaBTC<T>>::get(signer.clone()).checked_sub(&amount).ok_or(Error::<T>::InsufficientFunds)?);
            ext::treasury::transfer::<T>(Self::fee_pool_account_id(), signer.clone(), amount)?;
            Self::deposit_event(<Event<T>>::WithdrawPolkaBTC(
                signer,
                amount,
            ));
            Ok(())
        }

        /// Allows DOT reward withdrawal if balance is sufficient.
        ///
        /// # Arguments
        ///
        /// * `origin` - signing account
        /// * `amount` - amount of DOT
        #[weight = <T as Config>::WeightInfo::withdraw_dot()]
        #[transactional]
        fn withdraw_dot(origin, amount: DOT<T>) -> DispatchResult
        {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let signer = ensure_signed(origin)?;
            <TotalRewardsDOT<T>>::insert(signer.clone(), <TotalRewardsDOT<T>>::get(signer.clone()).checked_sub(&amount).ok_or(Error::<T>::InsufficientFunds)?);
            ext::collateral::transfer::<T>(Self::fee_pool_account_id(), signer.clone(), amount)?;
            Self::deposit_event(<Event<T>>::WithdrawDOT(
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

    // Public functions exposed to other pallets

    /// Updates total rewards in PolkaBTC and DOT for all participants and
    /// then clears the current epoch rewards.
    pub fn update_rewards_for_epoch() -> DispatchResult {
        // calculate vault rewards
        let (total_vault_rewards_for_issued_in_polka_btc, total_vault_rewards_for_locked_in_polka_btc) =
            Self::vault_rewards_for_epoch_in_polka_btc()?;
        let (total_vault_rewards_for_issued_in_dot, total_vault_rewards_for_locked_in_dot) =
            Self::vault_rewards_for_epoch_in_dot()?;
        for (account, amount_in_polka_btc, amount_in_dot) in ext::sla::get_vault_rewards::<T>(
            total_vault_rewards_for_issued_in_polka_btc,
            total_vault_rewards_for_locked_in_polka_btc,
            total_vault_rewards_for_issued_in_dot,
            total_vault_rewards_for_locked_in_dot,
        )? {
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

            // increase polka_btc rewards
            <TotalRewardsPolkaBTC<T>>::insert(
                account.clone(),
                <TotalRewardsPolkaBTC<T>>::get(account.clone())
                    .checked_add(&amount_in_polka_btc)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
            // increase dot rewards
            <TotalRewardsDOT<T>>::insert(
                account.clone(),
                <TotalRewardsDOT<T>>::get(account.clone())
                    .checked_add(&amount_in_dot)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
        }

        // calculate staked relayer rewards
        let total_relayer_rewards_in_polka_btc = Self::relayer_rewards_for_epoch_in_polka_btc()?;
        let total_relayer_rewards_in_dot = Self::relayer_rewards_for_epoch_in_dot()?;
        for (account, amount_in_polka_btc, amount_in_dot) in
            ext::sla::get_relayer_rewards::<T>(total_relayer_rewards_in_polka_btc, total_relayer_rewards_in_dot)?
        {
            // increase polka_btc rewards
            <TotalRewardsPolkaBTC<T>>::insert(
                account.clone(),
                <TotalRewardsPolkaBTC<T>>::get(account.clone())
                    .checked_add(&amount_in_polka_btc)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
            // increase dot rewards
            <TotalRewardsDOT<T>>::insert(
                account.clone(),
                <TotalRewardsDOT<T>>::get(account.clone())
                    .checked_add(&amount_in_dot)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );
        }

        // calculate maintainer rewards
        let maintainer_account_id = Self::maintainer_account_id();
        // increase polka_DOT rewards
        let total_maintainer_rewards_in_polka_btc = Self::maintainer_rewards_for_epoch_in_polka_btc()?;
        <TotalRewardsPolkaBTC<T>>::insert(
            maintainer_account_id.clone(),
            <TotalRewardsPolkaBTC<T>>::get(maintainer_account_id.clone())
                .checked_add(&total_maintainer_rewards_in_polka_btc)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );
        // increase dot rewards
        let total_maintainer_rewards_in_dot = Self::maintainer_rewards_for_epoch_in_dot()?;
        <TotalRewardsDOT<T>>::insert(
            maintainer_account_id.clone(),
            <TotalRewardsDOT<T>>::get(maintainer_account_id)
                .checked_add(&total_maintainer_rewards_in_dot)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );

        // clear total rewards for current epoch
        <EpochRewardsPolkaBTC<T>>::kill();
        <EpochRewardsDOT<T>>::kill();
        Ok(())
    }

    /// Increase the total amount of PolkaBTC generated in this epoch.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount of PolkaBTC
    pub fn increase_polka_btc_rewards_for_epoch(amount: PolkaBTC<T>) {
        <EpochRewardsPolkaBTC<T>>::set(Self::epoch_rewards_polka_btc() + amount);
    }

    /// Increase the total amount of DOT generated in this epoch.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount of DOT
    pub fn increase_dot_rewards_for_epoch(amount: DOT<T>) {
        <EpochRewardsDOT<T>>::set(Self::epoch_rewards_dot() + amount);
    }

    pub fn get_polka_btc_rewards(account_id: &T::AccountId) -> PolkaBTC<T> {
        <TotalRewardsPolkaBTC<T>>::get(account_id)
    }

    pub fn get_dot_rewards(account_id: &T::AccountId) -> DOT<T> {
        <TotalRewardsDOT<T>>::get(account_id)
    }

    /// Calculate the required issue fee in PolkaBTC.
    ///
    /// # Arguments
    ///
    /// * `amount` - issue amount in PolkaBTC
    pub fn get_issue_fee(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(amount, <IssueFee<T>>::get())
    }

    /// Calculate the required issue griefing collateral in DOT.
    ///
    /// # Arguments
    ///
    /// * `amount` - issue amount in DOT (at current exchange rate)
    pub fn get_issue_griefing_collateral(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <IssueGriefingCollateral<T>>::get())
    }

    /// Calculate the required redeem fee in PolkaBTC. Upon execution, the
    /// rewards should be forwarded to the fee pool instead of being burned.
    ///
    /// # Arguments
    ///
    /// * `amount` - redeem amount in PolkaBTC
    pub fn get_redeem_fee(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(amount, <RedeemFee<T>>::get())
    }

    /// Calculate the premium redeem fee in DOT for a user to get if redeeming
    /// with a Vault below the premium redeem threshold.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount in DOT (at current exchange rate)
    pub fn get_premium_redeem_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <PremiumRedeemFee<T>>::get())
    }

    /// Calculate the auction redeem fee in DOT for a new Vault to receive for
    /// successfully auctioning another Vault.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount in DOT (at current exchange rate)
    pub fn get_auction_redeem_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <AuctionRedeemFee<T>>::get())
    }

    /// Calculate punishment fee for a Vault that fails to execute a redeem
    /// request before the expiry.
    ///
    /// # Arguments
    ///
    /// * `amount` - amount in DOT (at current exchange rate)
    pub fn get_punishment_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <PunishmentFee<T>>::get())
    }

    /// Calculate the required replace griefing collateral in DOT.
    ///
    /// # Arguments
    ///
    /// * `amount` - replace amount in DOT (at current exchange rate)
    pub fn get_replace_griefing_collateral(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <ReplaceGriefingCollateral<T>>::get())
    }

    /// Calculate the fee portion of a total amount. For `amount = fee + refund_polkabtc`, this
    /// function returns `fee`.
    ///
    /// # Arguments
    ///
    /// * `amount` - total amount in PolkaBTC
    pub fn get_refund_fee_from_total(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
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

    pub fn btc_for(amount: PolkaBTC<T>, percentage: UnsignedFixedPoint<T>) -> Result<PolkaBTC<T>, DispatchError> {
        Self::inner_to_btc(Self::calculate_for(Self::btc_to_inner(amount)?, percentage)?)
    }

    pub fn dot_for(amount: DOT<T>, percentage: UnsignedFixedPoint<T>) -> Result<DOT<T>, DispatchError> {
        Self::inner_to_dot(Self::calculate_for(Self::dot_to_inner(amount)?, percentage)?)
    }

    // Private functions internal to this pallet

    fn btc_to_inner(x: PolkaBTC<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_btc(x: Inner<T>) -> Result<PolkaBTC<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<PolkaBTC<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn dot_to_inner(x: DOT<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_dot(x: Inner<T>) -> Result<DOT<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<DOT<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
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

    /// Total epoch rewards in PolkaBTC for vaults
    fn vault_rewards_for_epoch_in_polka_btc() -> Result<(PolkaBTC<T>, PolkaBTC<T>), DispatchError> {
        let total_vault_rewards = Self::btc_for(Self::epoch_rewards_polka_btc(), Self::vault_rewards())?;
        Ok((
            Self::btc_for(total_vault_rewards, Self::vault_rewards_issued())?,
            Self::btc_for(total_vault_rewards, Self::vault_rewards_locked())?,
        ))
    }

    /// Total epoch rewards in DOT for vaults
    fn vault_rewards_for_epoch_in_dot() -> Result<(DOT<T>, DOT<T>), DispatchError> {
        let total_vault_rewards = Self::dot_for(Self::epoch_rewards_dot(), Self::vault_rewards())?;
        Ok((
            Self::dot_for(total_vault_rewards, Self::vault_rewards_issued())?,
            Self::dot_for(total_vault_rewards, Self::vault_rewards_locked())?,
        ))
    }

    /// Total epoch rewards in PolkaBTC for staked relayers
    fn relayer_rewards_for_epoch_in_polka_btc() -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(Self::epoch_rewards_polka_btc(), Self::relayer_rewards())
    }

    /// Total epoch rewards in DOT for staked relayers
    fn relayer_rewards_for_epoch_in_dot() -> Result<DOT<T>, DispatchError> {
        Self::dot_for(Self::epoch_rewards_dot(), Self::relayer_rewards())
    }

    /// Total epoch rewards in PolkaBTC for maintainers
    fn maintainer_rewards_for_epoch_in_polka_btc() -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(Self::epoch_rewards_polka_btc(), Self::maintainer_rewards())
    }

    /// Total epoch rewards in DOT for maintainers
    fn maintainer_rewards_for_epoch_in_dot() -> Result<DOT<T>, DispatchError> {
        Self::dot_for(Self::epoch_rewards_dot(), Self::maintainer_rewards())
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
