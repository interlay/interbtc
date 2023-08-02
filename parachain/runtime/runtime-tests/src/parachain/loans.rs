use crate::setup::{assert_eq, *};
use currency::Amount;
use loans::{InterestRateModel, JumpModel, Market, MarketState};
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

#[allow(unused)]
fn free_balance_amount(currency_id: CurrencyId, account_id: &AccountId) -> Amount<Runtime> {
    let balance = free_balance(currency_id, account_id);
    Amount::new(balance, currency_id)
}

fn reserved_balance_amount(currency_id: CurrencyId, account_id: &AccountId) -> Amount<Runtime> {
    let balance = reserved_balance(currency_id, account_id);
    Amount::new(balance, currency_id)
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
        set_up_market(
            Token(IBTC),
            // Any value. This will not be considered when converting from IBTC to IBTC.
            FixedU128::one(),
            LEND_IBTC,
        );
        execute()
    });
}

pub fn almost_equal(target: u128, value: u128, precision: u8) -> bool {
    let target = target as i128;
    let value = value as i128;
    let diff = (target - value).abs() as u128;
    let delta = 10_u128.pow(precision.into());
    diff < delta
}

#[test]
fn integration_test_liquidation() {
    test_real_market(|| {
        let kint = Token(KINT);
        let one_kint = KINT.one();
        let ksm = Token(KSM);
        let one_ksm = KSM.one();
        let lend_kint_precision = KINT.decimals();
        let user = account_of(USER);
        let lp = account_of(LP);
        set_balance(user.clone(), kint, 1000 * one_kint);
        set_balance(user.clone(), ksm, 1000 * one_ksm);
        set_balance(lp.clone(), ksm, 1000 * one_ksm);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: kint,
            mint_amount: 1000 * one_kint,
        })
        .dispatch(origin_of(user.clone())));

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &user), 1000 * one_kint);
        assert_eq!(reserved_balance(LEND_KINT, &user), 0);

        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: ksm,
            mint_amount: 50 * one_ksm,
        })
        .dispatch(origin_of(lp.clone())));

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KSM, &lp), 50 * one_ksm);
        assert_eq!(reserved_balance(LEND_KSM, &lp), 0);

        assert_ok!(
            RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: kint }).dispatch(origin_of(user.clone()))
        );

        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &user), 0);
        assert_eq!(reserved_balance(LEND_KINT, &user), 1000 * one_kint);

        assert_err!(
            RuntimeCall::Loans(LoansCall::borrow {
                asset_id: ksm,
                borrow_amount: 40 * one_ksm,
            })
            .dispatch(origin_of(user.clone())),
            LoansError::InsufficientLiquidity
        );

        assert_eq!(free_balance(ksm, &user), 1000 * one_ksm);
        assert_ok!(RuntimeCall::Loans(LoansCall::borrow {
            asset_id: ksm,
            borrow_amount: 15 * one_ksm,
        })
        .dispatch(origin_of(user.clone())));
        assert_eq!(free_balance(ksm, &user), 1015 * one_ksm);

        assert_err!(
            RuntimeCall::Loans(LoansCall::liquidate_borrow {
                borrower: user.clone(),
                liquidation_asset_id: ksm,
                repay_amount: 15 * one_ksm,
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
            repay_amount: 7 * one_ksm,
            collateral_asset_id: kint
        })
        .dispatch(origin_of(lp.clone())));

        assert_eq!(free_balance(LEND_KINT, &user), 0);
        // borrower's reserved collateral is slashed
        assert_eq!(
            almost_equal(reserved_balance(LEND_KINT, &user), 610 * one_kint, lend_kint_precision),
            true
        );
        // borrower's borrowed balance is unchanged
        assert_eq!(free_balance(ksm, &user), 1015 * one_ksm);

        // the liquidator receives most of the slashed collateral
        assert_eq!(reserved_balance(LEND_KINT, &lp), 0);
        assert_eq!(
            almost_equal(free_balance(LEND_KINT, &lp), 380 * one_kint, lend_kint_precision),
            true
        );

        // the rest of the slashed collateral routed to the incentive reward account's free balance
        assert_eq!(
            almost_equal(
                free_balance(LEND_KINT, &LoansPallet::incentive_reward_account_id()),
                10 * one_kint,
                lend_kint_precision
            ),
            true
        );
        assert_eq!(
            reserved_balance(LEND_KINT, &LoansPallet::incentive_reward_account_id()),
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
            LoansPallet::account_deposits(LEND_DOT, &vault_account_id.clone()),
            reserved_balance_amount(LEND_DOT, &vault_account_id)
        );

        let lend_tokens = LoansPallet::free_lend_tokens(dot, &vault_account_id).unwrap();

        // Depositing all the collateral should leave none free for registering as a vault
        assert_ok!(RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: dot })
            .dispatch(origin_of(vault_account_id.clone())));
        assert_eq!(free_balance(LEND_DOT, &vault_account_id), 0);
        assert_eq!(reserved_balance(LEND_DOT, &vault_account_id), 1000);
        assert_eq!(
            LoansPallet::account_deposits(lend_tokens.currency(), &vault_account_id),
            reserved_balance_amount(LEND_DOT, &vault_account_id)
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
            LoansPallet::account_deposits(lend_tokens.currency(), &vault_account_id).amount(),
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
            LoansPallet::do_deposit_collateral(&vault_account_id, &lend_tokens),
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
            &(lend_tokens / 2)
        ));
        assert_eq!(
            LoansPallet::account_deposits(lend_tokens.currency(), &vault_account_id),
            reserved_balance_amount(lend_tokens.currency(), &vault_account_id)
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

#[test]
fn integration_test_switching_the_backing_collateral_works() {
    test_real_market(|| {
        let dot = Token(DOT);
        let one_dot = DOT.one();
        let ibtc = Token(IBTC);
        let one_ibtc = IBTC.one();
        let dot_collateral = Amount::<Runtime>::new(1000 * one_dot, dot);
        // amount of Satoshis the user will attempt to be undercollateralized by
        let shortfall_satoshis = 10;

        set_balance(account_of(USER), ibtc, 1000 * one_ibtc);
        set_balance(account_of(USER), dot, 1000 * one_dot);

        // deposit DOT, enable as collateral, and also borrow DOT
        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: dot,
            mint_amount: dot_collateral.amount(),
        })
        .dispatch(origin_of(account_of(USER))));
        assert_ok!(RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: dot })
            .dispatch(origin_of(account_of(USER))));
        assert_ok!(RuntimeCall::Loans(LoansCall::borrow {
            asset_id: dot,
            borrow_amount: dot_collateral.amount() / 2
        })
        .dispatch(origin_of(account_of(USER))));

        let required_ibtc_collateral = dot_collateral.convert_to(ibtc).unwrap();
        // 1000 DOT should be equal to 0.30823 IBTC at the configured exchange rate
        assert_eq!(required_ibtc_collateral.amount(), one_ibtc * 30823 / 100000);

        // deposit insufficient IBTC collateral to cover the existing loan
        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: ibtc,
            mint_amount: required_ibtc_collateral.amount() - shortfall_satoshis,
        })
        .dispatch(origin_of(account_of(USER))));
        assert_ok!(RuntimeCall::Loans(LoansCall::deposit_all_collateral { asset_id: ibtc })
            .dispatch(origin_of(account_of(USER))));

        // IBTC collateral is insufficient, disabling DOT collateral should fail
        assert_err!(
            RuntimeCall::Loans(LoansCall::withdraw_all_collateral { asset_id: dot })
                .dispatch(origin_of(account_of(USER))),
            LoansError::InsufficientLiquidity
        );

        // deposit more IBTC (auto-locked as collateral), this time it does cover the loan
        assert_ok!(RuntimeCall::Loans(LoansCall::mint {
            asset_id: ibtc,
            mint_amount: shortfall_satoshis,
        })
        .dispatch(origin_of(account_of(USER))));

        // must be able to disable DOT as collateral, because the IBTC collateral suffices
        assert_ok!(RuntimeCall::Loans(LoansCall::withdraw_all_collateral { asset_id: dot })
            .dispatch(origin_of(account_of(USER))));
    });
}
