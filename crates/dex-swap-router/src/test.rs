// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::{
    mock::{CurrencyId::*, *},
    StableSwapMode::FromBase,
    *,
};
use dex_general::DEFAULT_FEE_RATE;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

const INITIAL_A_VALUE: Balance = 50;
const SWAP_FEE: Balance = 1e7 as Balance;
const ADMIN_FEE: Balance = 0;

fn setup_stable_pools() {
    assert_ok!(DexStable::create_base_pool(
        RawOrigin::Root.into(),
        vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL)],
        vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL],
        INITIAL_A_VALUE,
        SWAP_FEE,
        ADMIN_FEE,
        USER1,
        Vec::from("basic_pool_lp"),
    ));

    let pool_id = DexStable::next_pool_id() - 1;
    let first_pool_lp_currency_id = StableLP(pool_id);

    assert_ok!(DexStable::add_liquidity(
        RawOrigin::Signed(USER1).into(),
        0,
        vec![1e18 as Balance, 1e18 as Balance, 1e6 as Balance],
        0,
        USER1,
        u64::MAX,
    ));

    assert_ok!(DexStable::create_meta_pool(
        RawOrigin::Root.into(),
        vec![Token(TOKEN4_SYMBOL), first_pool_lp_currency_id],
        vec![TOKEN4_DECIMAL, 18],
        INITIAL_A_VALUE,
        SWAP_FEE,
        ADMIN_FEE,
        USER1,
        Vec::from("pool_lp"),
    ));

    assert_ok!(DexStable::add_liquidity(
        RawOrigin::Signed(USER1).into(),
        1,
        vec![1e6 as Balance, 1e18 as Balance],
        0,
        USER1,
        u64::MAX,
    ));
}

fn setup_pools() {
    assert_ok!(DexGeneral::create_pair(
        RawOrigin::Root.into(),
        TOKEN1_ASSET_ID,
        TOKEN2_ASSET_ID,
        DEFAULT_FEE_RATE,
    ));
    assert_ok!(DexGeneral::add_liquidity(
        RawOrigin::Signed(USER1).into(),
        TOKEN1_ASSET_ID,
        TOKEN2_ASSET_ID,
        1e18 as Balance,
        1e18 as Balance,
        0,
        0,
        u64::MAX
    ));
}

#[test]
fn swap_exact_tokens_for_tokens_with_amount_slippage_should_failed() {
    new_test_ext().execute_with(|| {
        setup_stable_pools();
        setup_pools();

        let routes = vec![
            Route::General(vec![TOKEN2_ASSET_ID, TOKEN1_ASSET_ID]),
            Route::Stable(StablePath::<PoolId, CurrencyId> {
                pool_id: 1,
                base_pool_id: 0,
                mode: FromBase,
                from_currency: Token(TOKEN1_SYMBOL),
                to_currency: Token(TOKEN4_SYMBOL),
            }),
        ];

        assert_noop!(
            DexSwapRouter::swap_exact_tokens_for_tokens(
                RawOrigin::Signed(USER2).into(),
                1e16 as Balance,
                u128::MAX,
                routes,
                USER1,
                u64::MAX,
            ),
            Error::<Test>::AmountSlippage
        );
    })
}

#[test]
fn swap_exact_tokens_for_tokens_should_work() {
    new_test_ext().execute_with(|| {
        setup_stable_pools();
        setup_pools();

        let routes = vec![
            Route::General(vec![TOKEN2_ASSET_ID, TOKEN1_ASSET_ID]),
            Route::Stable(StablePath::<PoolId, CurrencyId> {
                pool_id: 1,
                base_pool_id: 0,
                mode: FromBase,
                from_currency: Token(TOKEN1_SYMBOL),
                to_currency: Token(TOKEN4_SYMBOL),
            }),
        ];
        let token1_balance_before = Tokens::accounts(USER1, Token(TOKEN1_SYMBOL)).free;
        let token2_balance_before = Tokens::accounts(USER1, Token(TOKEN2_SYMBOL)).free;
        let token3_balance_before = Tokens::accounts(USER1, Token(TOKEN3_SYMBOL)).free;
        let token4_balance_before = Tokens::accounts(USER2, Token(TOKEN4_SYMBOL)).free;

        assert_ok!(DexSwapRouter::swap_exact_tokens_for_tokens(
            RawOrigin::Signed(USER1).into(),
            1e16 as Balance,
            0,
            routes,
            USER2,
            u64::MAX,
        ));

        assert_eq!(
            Tokens::accounts(USER1, Token(TOKEN1_SYMBOL)).free,
            token1_balance_before
        );
        assert_eq!(
            Tokens::accounts(USER1, Token(TOKEN2_SYMBOL)).free,
            token2_balance_before - 1e16 as Balance
        );
        assert_eq!(
            Tokens::accounts(USER1, Token(TOKEN3_SYMBOL)).free,
            token3_balance_before
        );
        assert_eq!(
            Tokens::accounts(USER2, Token(TOKEN4_SYMBOL)).free,
            token4_balance_before + 9854
        );
    })
}

#[test]
fn test_validate_routes() {
    new_test_ext().execute_with(|| {
        fn stable(input: CurrencyId, output: CurrencyId) -> Route<PoolId, CurrencyId> {
            Route::Stable(StablePath::<PoolId, CurrencyId> {
                pool_id: 1,
                base_pool_id: 0,
                mode: FromBase,
                from_currency: input,
                to_currency: output,
            })
        }

        // single routes..
        assert_ok!(DexSwapRouter::validate_routes(&[Route::General(vec![
            Token(1),
            Token(2)
        ]),]));
        assert_ok!(DexSwapRouter::validate_routes(&[stable(Token(2), Token(3))]));

        // 2 routes
        assert_ok!(DexSwapRouter::validate_routes(&[
            Route::General(vec![Token(1), Token(2)]),
            stable(Token(2), Token(3))
        ]));

        // many routes
        assert_ok!(DexSwapRouter::validate_routes(&[
            Route::General(vec![Token(1), Token(2), Token(3)]),
            stable(Token(3), Token(2)),
            stable(Token(2), Token(1)),
            Route::General(vec![Token(1), Token(2)]),
            Route::General(vec![Token(2), Token(1)]),
        ]));

        // a "gap" in the routes - output of one route does match input of next
        assert_noop!(
            DexSwapRouter::validate_routes(&[
                Route::General(vec![Token(1), Token(2)]),
                Route::General(vec![Token(1), Token(4)]),
            ]),
            Error::<Test>::InvalidRoutes
        );

        // empty output currency
        assert_noop!(
            DexSwapRouter::validate_routes(&[Route::General(vec![]), Route::General(vec![Token(1), Token(4)]),]),
            Error::<Test>::InvalidPath
        );

        // empty input currency
        assert_noop!(
            DexSwapRouter::validate_routes(&[Route::General(vec![Token(1), Token(4)]), Route::General(vec![])]),
            Error::<Test>::InvalidPath
        );
    })
}
