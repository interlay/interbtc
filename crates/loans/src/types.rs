use crate::{Config, InterestRateModel};
use codec::MaxEncodedLen;
use currency::Amount;
use frame_support::pallet_prelude::*;
use primitives::{CurrencyId, Liquidity, Rate, Ratio, Shortfall};
use scale_info::TypeInfo;

// TODO: `cargo doc` crashes on this type, remove the `hidden` macro
// when upgrading rustc in case that fixes it
/// Container for account liquidity information
#[doc(hidden)]
#[derive(Eq, PartialEq, Clone, RuntimeDebug)]
pub enum AccountLiquidity<T: Config> {
    Liquidity(Amount<T>),
    Shortfall(Amount<T>),
}

impl<T: Config> AccountLiquidity<T> {
    pub fn from_collateral_and_debt(
        collateral_value: Amount<T>,
        borrow_value: Amount<T>,
    ) -> Result<Self, DispatchError> {
        let account_liquidity = if collateral_value.gt(&borrow_value)? {
            AccountLiquidity::Liquidity(collateral_value.checked_sub(&borrow_value)?)
        } else {
            AccountLiquidity::Shortfall(borrow_value.checked_sub(&collateral_value)?)
        };
        Ok(account_liquidity)
    }

    pub fn currency(&self) -> CurrencyId {
        match &self {
            AccountLiquidity::Liquidity(x) | AccountLiquidity::Shortfall(x) => x.currency(),
        }
    }

    pub fn liquidity(&self) -> Amount<T> {
        if let AccountLiquidity::Liquidity(x) = &self {
            return x.clone();
        }
        Amount::<T>::zero(self.currency())
    }

    pub fn shortfall(&self) -> Amount<T> {
        if let AccountLiquidity::Shortfall(x) = &self {
            return x.clone();
        }
        Amount::<T>::zero(self.currency())
    }

    pub fn to_rpc_tuple(&self) -> Result<(Liquidity, Shortfall), DispatchError> {
        Ok((
            self.liquidity().to_unsigned_fixed_point()?,
            self.shortfall().to_unsigned_fixed_point()?,
        ))
    }
}

/// Container for borrow balance information
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo, MaxEncodedLen)]
pub struct BorrowSnapshot<Balance> {
    /// Principal Total balance (with accrued interest), after applying the most recent balance-changing action.
    /// In other words, this is the amount of underlying borrowed that is to be paid back eventually.
    pub principal: Balance,
    /// InterestIndex Global borrowIndex as of the most recent balance-changing action
    pub borrow_index: Rate,
}

/// Container for earned amount information
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct EarnedSnapshot<Balance> {
    /// Total deposit interest, after applying the most recent balance-changing action
    pub total_earned_prior: Balance,
    /// Exchange rate, after applying the most recent balance-changing action
    pub exchange_rate_prior: Rate,
}

/// The current state of a market. For more information, see [Market].
#[derive(
    serde::Deserialize,
    serde::Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    codec::Decode,
    codec::Encode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum MarketState {
    Active,
    Pending,
    // Unclear why the `Supervision` state is required at all, since it's not used anywhere.
    // Could just reuse the `Pending` state to temporarily halt a market.
    Supervision,
}

/// Market.
///
/// A large pool of liquidity where accounts can lend and borrow.
#[derive(
    serde::Deserialize,
    serde::Serialize,
    Clone,
    PartialEq,
    Eq,
    codec::Decode,
    codec::Encode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct Market<Balance> {
    /// The secure collateral ratio
    pub collateral_factor: Ratio,
    /// The collateral ratio when a borrower can be liquidated. Higher than the `collateral_factor` and lower than
    /// 100%.
    pub liquidation_threshold: Ratio,
    /// Fraction of interest currently set aside for reserves
    pub reserve_factor: Ratio,
    /// The percent, ranging from 0% to 100%, of a liquidatable account's
    /// borrow that can be repaid in a single liquidate transaction.
    pub close_factor: Ratio,
    /// Liquidation incentive ratio
    pub liquidate_incentive: Rate,
    /// Liquidation share set aside for reserves
    pub liquidate_incentive_reserved_factor: Ratio,
    /// Current interest rate model being used
    pub rate_model: InterestRateModel,
    /// Current market state
    pub state: MarketState,
    /// Upper bound of supplying
    pub supply_cap: Balance,
    /// Upper bound of borrowing
    pub borrow_cap: Balance,
    /// LendToken asset id
    pub lend_token_id: CurrencyId,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default, MaxEncodedLen)]
pub struct RewardMarketState<BlockNumber, Balance> {
    pub index: Balance,
    /// total amount of staking asset user deposited
    pub block: BlockNumber,
}
