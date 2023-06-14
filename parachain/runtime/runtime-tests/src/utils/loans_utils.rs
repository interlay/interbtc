use currency::Amount;
use loans::JumpModel;
use traits::LoansApi;

use crate::setup::*;

pub const fn market_mock(lend_token_id: CurrencyId) -> Market<Balance> {
    Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        state: MarketState::Pending,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        lend_token_id,
    }
}

pub fn activate_market(underlying_id: CurrencyId, lend_token_id: CurrencyId) {
    assert_ok!(RuntimeCall::Loans(LoansCall::add_market {
        asset_id: underlying_id,
        market: market_mock(lend_token_id)
    })
    .dispatch(root()));
    assert_ok!(RuntimeCall::Loans(LoansCall::activate_market {
        asset_id: underlying_id
    })
    .dispatch(root()));
}

pub fn mint_lend_tokens(account_id: AccountId, lend_token_id: CurrencyId) {
    let underlying_id = LoansPallet::underlying_id(lend_token_id).unwrap();
    let balance_to_mint = FUND_LIMIT_CEILING;
    let amount: Amount<Runtime> = Amount::new(balance_to_mint, underlying_id);
    assert_ok!(amount.mint_to(&account_id));

    assert_ok!(RuntimeCall::Loans(LoansCall::mint {
        asset_id: underlying_id,
        mint_amount: balance_to_mint
    })
    .dispatch(origin_of(account_id)));
}

/// Deposits and borrows the same currency, for the sake of having accruing interest in the market
pub fn deposit_and_borrow(account_id: AccountId, amount: Amount<Runtime>) {
    assert_ok!(amount.mint_to(&account_id));
    assert_ok!(RuntimeCall::Loans(LoansCall::mint {
        asset_id: amount.currency(),
        mint_amount: amount.amount()
    })
    .dispatch(origin_of(account_id.clone())));
    assert_ok!(RuntimeCall::Loans(LoansCall::deposit_all_collateral {
        asset_id: amount.currency()
    })
    .dispatch(origin_of(account_id.clone())));
    assert_ok!(RuntimeCall::Loans(LoansCall::borrow {
        asset_id: amount.currency(),
        borrow_amount: amount.amount() / 2,
    })
    .dispatch(origin_of(account_id.clone())));
}

pub fn activate_lending_and_mint(underlying_id: CurrencyId, lend_token_id: CurrencyId) {
    activate_market(underlying_id, lend_token_id);
    for account in iter_endowed_with_lend_token() {
        mint_lend_tokens(account, lend_token_id);
    }
}
