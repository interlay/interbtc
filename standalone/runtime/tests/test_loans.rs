use interbtc_runtime_standalone::{CurrencyId::Token, KINT};
mod mock;
use mock::{assert_eq, *};
use pallet_loans::{InterestRateModel, JumpModel, Market, MarketState};
use primitives::{Rate, Ratio, CKINT, CKSM};
use sp_runtime::traits::CheckedMul;

pub const USER: [u8; 32] = ALICE;
pub const LP: [u8; 32] = BOB;

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

fn set_up_market(currency_id: CurrencyId, exchange_rate: FixedU128, ctoken_id: CurrencyId) {
    assert_ok!(OraclePallet::_set_exchange_rate(currency_id, exchange_rate));
    assert_ok!(Call::Sudo(SudoCall::sudo {
        call: Box::new(Call::Loans(LoansCall::add_market {
            asset_id: currency_id,
            market: market_mock(ctoken_id),
        })),
    })
    .dispatch(origin_of(account_of(ALICE))));

    assert_ok!(Call::Sudo(SudoCall::sudo {
        call: Box::new(Call::Loans(LoansCall::activate_market { asset_id: currency_id })),
    })
    .dispatch(origin_of(account_of(ALICE))));
}

fn test_real_market<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        // Use real market data for the exchange rates
        set_up_market(
            Token(KINT),
            FixedU128::from_inner(115_942_028_985_507_246_376_810_000),
            Token(CKINT),
        );
        set_up_market(
            Token(KSM),
            FixedU128::from_inner(4_573_498_406_135_805_461_670_000),
            Token(CKSM),
        );
        execute()
    });
}

#[test]
fn integration_test_issue_expiry_only_parachain_blocks_expired() {
    test_real_market(|| {
        let kint = Token(KINT);
        let ksm = Token(KSM);

        assert_ok!(Call::Loans(LoansCall::mint {
            asset_id: kint,
            mint_amount: 1000,
        })
        .dispatch(origin_of(account_of(USER))));

        assert_ok!(Call::Loans(LoansCall::mint {
            asset_id: ksm,
            mint_amount: 50,
        })
        .dispatch(origin_of(account_of(LP))));

        assert_ok!(Call::Loans(LoansCall::collateral_asset {
            asset_id: kint,
            enable: true,
        })
        .dispatch(origin_of(account_of(USER))));

        assert_err!(
            Call::Loans(LoansCall::borrow {
                asset_id: ksm,
                borrow_amount: 20,
            })
            .dispatch(origin_of(account_of(USER))),
            LoansError::InsufficientLiquidity
        );

        assert_ok!(Call::Loans(LoansCall::borrow {
            asset_id: ksm,
            borrow_amount: 15,
        })
        .dispatch(origin_of(account_of(USER))));

        assert_err!(
            Call::Loans(LoansCall::liquidate_borrow {
                borrower: account_of(USER),
                liquidation_asset_id: ksm,
                repay_amount: 15,
                collateral_asset_id: kint
            })
            .dispatch(origin_of(account_of(LP))),
            LoansError::InsufficientShortfall
        );

        // KINT price drops to half
        let kint_rate = OraclePallet::get_price(OracleKey::ExchangeRate(kint)).unwrap();
        assert_ok!(OraclePallet::_set_exchange_rate(
            kint,
            kint_rate.checked_mul(&2.into()).unwrap()
        ));

        assert_ok!(Call::Loans(LoansCall::liquidate_borrow {
            borrower: account_of(USER),
            liquidation_asset_id: ksm,
            repay_amount: 7,
            collateral_asset_id: kint
        })
        .dispatch(origin_of(account_of(LP))));
    });
}
