// Copyright 2022 Interlay.
// This file is part of Interlay.

// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchError;
use primitives::{Rate, Ratio};
use scale_info::TypeInfo;
use sp_runtime::{FixedU128, RuntimeDebug};
use sp_std::prelude::*;

pub trait LoansApi<CurrencyId, AccountId, Amount> {
    fn do_mint(supplier: &AccountId, amount: &Amount) -> Result<(), DispatchError>;
    fn do_borrow(borrower: &AccountId, borrow: &Amount) -> Result<(), DispatchError>;
    fn do_deposit_collateral(supplier: &AccountId, lend_tokens: &Amount) -> Result<(), DispatchError>;
    fn do_withdraw_collateral(supplier: &AccountId, voucher: &Amount) -> Result<(), DispatchError>;
    fn do_repay_borrow(borrower: &AccountId, borrow: &Amount) -> Result<(), DispatchError>;
    fn do_redeem(supplier: &AccountId, amount: &Amount, voucher: &Amount) -> Result<(), DispatchError>;
    fn recompute_underlying_amount(lend_tokens: &Amount) -> Result<Amount, DispatchError>;
    fn underlying_id(lend_token_id: CurrencyId) -> Result<CurrencyId, DispatchError>;
    fn recompute_collateral_amount(underlying: &Amount) -> Result<Amount, DispatchError>;
}

pub trait LoansMarketDataProvider<CurrencyId, Balance> {
    fn get_market_info(asset_id: CurrencyId) -> Result<MarketInfo, DispatchError>;
    fn get_market_status(asset_id: CurrencyId) -> Result<MarketStatus<Balance>, DispatchError>;
    // for compatibility we keep this func
    fn get_full_interest_rate(asset_id: CurrencyId) -> Option<Rate>;
}

/// MarketInfo contains some static attrs as a subset of Market struct in Loans
#[derive(Default, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct MarketInfo {
    pub collateral_factor: Ratio,
    pub liquidation_threshold: Ratio,
    pub reserve_factor: Ratio,
    pub close_factor: Ratio,
    pub full_rate: Rate,
}

/// MarketStatus contains some dynamic calculated attrs of Market
#[derive(Default, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct MarketStatus<Balance> {
    pub borrow_rate: Rate,
    pub supply_rate: Rate,
    pub exchange_rate: Rate,
    pub utilization: Ratio,
    pub total_borrows: Balance,
    pub total_reserves: Balance,
    pub borrow_index: FixedU128,
}
