use pallet_loans::JumpModel;

use crate::{assert_eq, *};

pub const fn market_mock(ptoken_id: CurrencyId) -> Market<Balance> {
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
        ptoken_id,
    }
}

pub fn activate_market(underlying_id: CurrencyId, ptoken_id: CurrencyId) {
    assert_ok!(Call::Loans(LoansCall::add_market {
        asset_id: underlying_id,
        market: market_mock(ptoken_id)
    })
    .dispatch(root()));
    assert_ok!(Call::Loans(LoansCall::activate_market {
        asset_id: underlying_id
    })
    .dispatch(root()));
}

pub fn mint_ptokens(account_id: AccountId, underlying_id: CurrencyId) {
    let balance_to_mint = FUND_LIMIT_CEILING;
    let amount: Amount<Runtime> = Amount::new(balance_to_mint, underlying_id);
    assert_ok!(amount.mint_to(&account_id));

    assert_ok!(Call::Loans(LoansCall::mint {
        asset_id: Token(DOT),
        mint_amount: balance_to_mint
    })
    .dispatch(origin_of(account_id)));
}
