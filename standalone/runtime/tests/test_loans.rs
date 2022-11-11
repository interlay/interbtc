use interbtc_runtime_standalone::{CurrencyId::Token, Tokens, KINT};
mod mock;
use loans::{InterestRateModel, JumpModel, Market, MarketState};
use mock::{assert_eq, *};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::{Rate, Ratio};
use sp_runtime::traits::CheckedMul;
use traits::LoansApi;

pub const USER: [u8; 32] = ALICE;
pub const LP: [u8; 32] = BOB;

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

fn free_balance(currency_id: CurrencyId, account_id: &AccountId) -> Balance {
    <Tokens as MultiCurrency<<Runtime as frame_system::Config>::AccountId>>::free_balance(currency_id, account_id)
}

fn reserved_balance(currency_id: CurrencyId, account_id: &AccountId) -> Balance {
    <Tokens as MultiReservableCurrency<<Runtime as frame_system::Config>::AccountId>>::reserved_balance(
        currency_id,
        account_id,
    )
}

fn set_up_market(currency_id: CurrencyId, exchange_rate: FixedU128, lend_token_id: CurrencyId) {
    assert_ok!(OraclePallet::_set_exchange_rate(currency_id, exchange_rate));
    assert_ok!(RuntimeCall::Sudo(SudoCall::sudo {
        call: Box::new(RuntimeCall::Loans(LoansCall::add_market {
            asset_id: currency_id,
            market: market_mock(lend_token_id),
        })),
    })
    .dispatch(origin_of(account_of(ALICE))));

    assert_ok!(RuntimeCall::Sudo(SudoCall::sudo {
        call: Box::new(RuntimeCall::Loans(LoansCall::activate_market { asset_id: currency_id })),
    })
    .dispatch(origin_of(account_of(ALICE))));
}

fn test_real_market<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        // Use real market data for the exchange rates
        set_up_market(
            Token(KINT),
            FixedU128::from_inner(115_942_028_985_507_246_376_810_000),
            LEND_KINT,
        );
        set_up_market(
            Token(KSM),
            FixedU128::from_inner(4_573_498_406_135_805_461_670_000),
            LEND_KSM,
        );
        set_up_market(
            Token(DOT),
            FixedU128::from_inner(324_433_053_239_464_036_596_000),
            LEND_DOT,
        );
        execute()
    });
}

#[test]
fn integration_test_liquidation() {
    test_real_market(|| {
        let kint = Token(KINT);
        let ksm = Token(KSM);
        let user = account_of(USER);
        let lp = account_of(LP);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: kint,
            mint_amount: 1000,
        })
        .dispatch(origin_of(user.clone())));

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &user), 1000);
        assert_eq!(reserved_balance(LEND_KINT, &user), 0);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: ksm,
            mint_amount: 50,
        })
        .dispatch(origin_of(lp.clone())));

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KSM, &lp), 50);
        assert_eq!(reserved_balance(LEND_KSM, &lp), 0);

        assert_ok!(
            RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: kint }).dispatch(origin_of(user.clone()))
        );

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &user), 0);
        assert_eq!(reserved_balance(LEND_KINT, &user), 1000);

        assert_err!(
            RuntimeCall::Loans(LoansCall::borrow {
                asset_id: ksm,
                borrow_amount: 20,
            })
            .dispatch(origin_of(user.clone())),
            LoansError::InsufficientLiquidity
        );

        assert_eq!(free_balance(ksm, &user), 1000000000000);
        assert_ok!(RuntimeCall::Loans(LoansCall::borrow {
            asset_id: ksm,
            borrow_amount: 15,
        })
        .dispatch(origin_of(user.clone())));
        assert_eq!(free_balance(ksm, &user), 1000000000015);

        assert_err!(
            RuntimeCall::Loans(LoansCall::liquidate_borrow {
                borrower: user.clone(),
                liquidation_asset_id: ksm,
                repay_amount: 15,
                collateral_asset_id: kint
            })
            .dispatch(origin_of(lp.clone())),
            LoansError::InsufficientShortfall
        );

        // KINT price drops to half
        let kint_rate = OraclePallet::get_price(OracleKey::ExchangeRate(kint)).unwrap();
        assert_ok!(OraclePallet::_set_exchange_rate(
            kint,
            kint_rate.checked_mul(&2.into()).unwrap()
        ));

        assert_ok!(RuntimeCall::Loans(LoansCall::liquidate_borrow {
            borrower: user.clone(),
            liquidation_asset_id: ksm,
            repay_amount: 7,
            collateral_asset_id: kint
        })
        .dispatch(origin_of(lp.clone())));

        assert_eq!(free_balance(LEND_KINT, &user), 0);
        // borrower's reserved collateral is slashed
        assert_eq!(reserved_balance(LEND_KINT, &user), 610);
        // borrower's borrowed balance is unchanged
        assert_eq!(free_balance(ksm, &user), 1000000000015);

        // the liquidator receives most of the slashed collateral
        assert_eq!(reserved_balance(LEND_KINT, &lp), 0);
        assert_eq!(free_balance(LEND_KINT, &lp), 380);

        // the rest of the slashed collateral routed to the incentive reward account's free balance
        assert_eq!(
            free_balance(LEND_KINT, &LoansPallet::incentive_reward_account_id().unwrap()),
            10
        );
        assert_eq!(
            reserved_balance(LEND_KINT, &LoansPallet::incentive_reward_account_id().unwrap()),
            0
        );
    });
}

#[test]
fn integration_test_lend_token_vault_insufficient_balance() {
    test_real_market(|| {
        let dot = Token(DOT);
        let vault_account_id = account_of(USER);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: dot,
            mint_amount: 1000,
        })
        .dispatch(origin_of(account_of(USER))));

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 0);
        assert_eq!(
            LoansPallet::account_deposits(LEND_DOT, vault_account_id.clone()),
            reserved_balance(LEND_DOT, &vault_account_id)
        );

        let lend_tokens = LoansPallet::free_lend_tokens(dot, &vault_account_id).unwrap();

        // Depositing all the collateral should leave none free for registering as a vault
        assert_ok!(RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: dot })
            .dispatch(origin_of(vault_account_id.clone())));
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 0);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(
            LoansPallet::account_deposits(lend_tokens.currency(), vault_account_id.clone()),
            reserved_balance(LEND_DOT, &vault_account_id)
        );

        let lend_token_vault_id = PrimitiveVaultId::new(vault_account_id.clone(), lend_tokens.currency(), Token(IBTC));
        assert_err!(
            get_register_vault_result(&lend_token_vault_id, lend_tokens),
            TokensError::BalanceTooLow
        );

        // Withdraw the lend_tokens to use them for another purpose
        assert_ok!(RuntimeCall::Loans(LoansCall::withdraw_all_collateral { asset_id: dot })
            .dispatch(origin_of(vault_account_id.clone())));
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 0);

        // This time, registering a vault works because the lend_tokens are unlocked
        assert_ok!(get_register_vault_result(&lend_token_vault_id, lend_tokens));
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 0);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(
            LoansPallet::account_deposits(lend_tokens.currency(), vault_account_id.clone()),
            0
        );
    });
}

#[test]
fn integration_test_lend_token_deposit_insufficient_balance() {
    test_real_market(|| {
        let dot = Token(DOT);
        let vault_account_id = account_of(USER);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: dot,
            mint_amount: 1000,
        })
        .dispatch(origin_of(account_of(USER))));

        let lend_tokens = LoansPallet::free_lend_tokens(dot, &vault_account_id).unwrap();

        // Register a vault with all the available lend_tokens
        let lend_token_vault_id = PrimitiveVaultId::new(vault_account_id.clone(), lend_tokens.currency(), Token(IBTC));
        assert_ok!(get_register_vault_result(&lend_token_vault_id, lend_tokens),);

        assert_err!(
            LoansPallet::do_deposit_collateral(&vault_account_id, lend_tokens.currency(), lend_tokens.amount()),
            TokensError::BalanceTooLow
        );
    });
}

#[test]
fn integration_test_lend_token_transfer_reserved_fails() {
    test_real_market(|| {
        let dot = Token(DOT);
        let vault_account_id = account_of(USER);
        let lp_account_id = account_of(LP);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: dot,
            mint_amount: 1000,
        })
        .dispatch(origin_of(vault_account_id.clone())));

        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 0);
        let lend_tokens = LoansPallet::free_lend_tokens(dot, &vault_account_id).unwrap();

        // Lock some lend_tokens into the lending market
        assert_ok!(LoansPallet::do_deposit_collateral(
            &vault_account_id,
            lend_tokens.currency(),
            lend_tokens.amount() / 2
        ));
        assert_eq!(
            LoansPallet::account_deposits(lend_tokens.currency(), vault_account_id.clone()),
            reserved_balance(lend_tokens.currency(), &vault_account_id)
        );
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 500);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 500);

        let half_lend_tokens = lend_tokens.checked_div(&FixedU128::from_u32(2)).unwrap();
        assert_eq!(
            half_lend_tokens,
            LoansPallet::free_lend_tokens(dot, &vault_account_id).unwrap()
        );

        // Transferring the full amount fails
        assert_err!(
            lend_tokens.transfer(&vault_account_id, &lp_account_id),
            TokensError::BalanceTooLow
        );
        assert_ok!(half_lend_tokens.transfer(&vault_account_id, &lp_account_id));
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 0);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 500);
        assert_eq!(free_balance(LEND_DOT, &lp_account_id), 500);
    });
}
