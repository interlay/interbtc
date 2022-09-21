use crate::{
    mock::{
        market_mock, new_test_ext, Loans, Origin, Test, ACTIVE_MARKET_MOCK, ALICE,
        MARKET_MOCK,
    },
    Error, InterestRateModel, MarketState,
};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use primitives::{Rate, Ratio, CDOT, CKBTC, IBTC, CurrencyId::{self, Token, ForeignAsset},  DOT as DOT_CURRENCY, CKSM,};
use sp_runtime::{traits::Zero, FixedPointNumber};

const DOT: CurrencyId = Token(DOT_CURRENCY);
const PDOT: CurrencyId = Token(CDOT);
const PUSDT: CurrencyId = Token(CKBTC);
const SDOT: CurrencyId = ForeignAsset(987997280);

macro_rules! rate_model_sanity_check {
    ($call:ident) => {
        new_test_ext().execute_with(|| {
            // Invalid base_rate
            assert_noop!(
                Loans::$call(Origin::root(), SDOT, {
                    let mut market = MARKET_MOCK;
                    market.rate_model = InterestRateModel::new_jump_model(
                        Rate::saturating_from_rational(36, 100),
                        Rate::saturating_from_rational(15, 100),
                        Rate::saturating_from_rational(35, 100),
                        Ratio::from_percent(80),
                    );
                    market
                }),
                Error::<Test>::InvalidRateModelParam
            );
            // Invalid jump_rate
            assert_noop!(
                Loans::$call(Origin::root(), SDOT, {
                    let mut market = MARKET_MOCK;
                    market.rate_model = InterestRateModel::new_jump_model(
                        Rate::saturating_from_rational(5, 100),
                        Rate::saturating_from_rational(36, 100),
                        Rate::saturating_from_rational(37, 100),
                        Ratio::from_percent(80),
                    );
                    market
                }),
                Error::<Test>::InvalidRateModelParam
            );
            // Invalid full_rate
            assert_noop!(
                Loans::$call(Origin::root(), SDOT, {
                    let mut market = MARKET_MOCK;
                    market.rate_model = InterestRateModel::new_jump_model(
                        Rate::saturating_from_rational(5, 100),
                        Rate::saturating_from_rational(15, 100),
                        Rate::saturating_from_rational(57, 100),
                        Ratio::from_percent(80),
                    );
                    market
                }),
                Error::<Test>::InvalidRateModelParam
            );
            // base_rate greater than jump_rate
            assert_noop!(
                Loans::$call(Origin::root(), SDOT, {
                    let mut market = MARKET_MOCK;
                    market.rate_model = InterestRateModel::new_jump_model(
                        Rate::saturating_from_rational(10, 100),
                        Rate::saturating_from_rational(9, 100),
                        Rate::saturating_from_rational(14, 100),
                        Ratio::from_percent(80),
                    );
                    market
                }),
                Error::<Test>::InvalidRateModelParam
            );
            // jump_rate greater than full_rate
            assert_noop!(
                Loans::$call(Origin::root(), SDOT, {
                    let mut market = MARKET_MOCK;
                    market.rate_model = InterestRateModel::new_jump_model(
                        Rate::saturating_from_rational(5, 100),
                        Rate::saturating_from_rational(15, 100),
                        Rate::saturating_from_rational(14, 100),
                        Ratio::from_percent(80),
                    );
                    market
                }),
                Error::<Test>::InvalidRateModelParam
            );
        })
    };
}

#[test]
fn active_market_sets_state_to_active() {
    new_test_ext().execute_with(|| {
        Loans::add_market(Origin::root(), SDOT, MARKET_MOCK).unwrap();
        assert_eq!(Loans::market(SDOT).unwrap().state, MarketState::Pending);
        Loans::activate_market(Origin::root(), SDOT).unwrap();
        assert_eq!(Loans::market(SDOT).unwrap().state, MarketState::Active);
    })
}

#[test]
fn active_market_does_not_modify_unknown_market_currencies() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::activate_market(Origin::root(), SDOT),
            Error::<Test>::MarketDoesNotExist
        );
    })
}

#[test]
fn add_market_can_only_be_used_by_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::add_market(Origin::signed(ALICE), DOT, MARKET_MOCK),
            BadOrigin
        );
    })
}

#[test]
fn add_market_ensures_that_market_state_must_be_pending() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::add_market(Origin::root(), SDOT, ACTIVE_MARKET_MOCK),
            Error::<Test>::NewMarketMustHavePendingState
        );
    })
}

#[test]
fn add_market_has_sanity_checks_for_rate_models() {
    rate_model_sanity_check!(add_market);
}

#[test]
fn add_market_successfully_stores_a_new_market() {
    new_test_ext().execute_with(|| {
        Loans::add_market(Origin::root(), SDOT, MARKET_MOCK).unwrap();
        assert_eq!(Loans::market(SDOT).unwrap(), MARKET_MOCK);
    })
}

#[test]
fn add_market_ensures_that_market_does_not_exist() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::add_market(Origin::root(), SDOT, MARKET_MOCK));
        assert_noop!(
            Loans::add_market(Origin::root(), SDOT, MARKET_MOCK),
            Error::<Test>::MarketAlreadyExists
        );
    })
}

#[test]
fn force_update_market_can_only_be_used_by_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::force_update_market(Origin::signed(ALICE), DOT, MARKET_MOCK),
            BadOrigin
        );
    })
}

#[test]
fn force_update_market_works() {
    new_test_ext().execute_with(|| {
        let mut new_market = market_mock(PDOT);
        new_market.state = MarketState::Active;
        Loans::force_update_market(Origin::root(), DOT, new_market).unwrap();
        assert_eq!(Loans::market(DOT).unwrap().state, MarketState::Active);
        assert_eq!(Loans::market(DOT).unwrap().ptoken_id, PDOT);

        // New ptoken_id must not be in use
        assert_noop!(
            Loans::force_update_market(Origin::root(), DOT, market_mock(PUSDT)),
            Error::<Test>::InvalidPtokenId
        );
        assert_ok!(Loans::force_update_market(
            Origin::root(),
            DOT,
            market_mock(ForeignAsset(1234))
        ));
        assert_eq!(Loans::market(DOT).unwrap().ptoken_id, ForeignAsset(1234));
    })
}

#[test]
fn force_update_market_ensures_that_it_is_not_possible_to_modify_unknown_market_currencies() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::force_update_market(Origin::root(), SDOT, MARKET_MOCK),
            Error::<Test>::MarketDoesNotExist
        );
    })
}

#[test]
fn update_market_has_sanity_checks_for_rate_models() {
    rate_model_sanity_check!(force_update_market);
}

#[test]
fn update_market_ensures_that_it_is_not_possible_to_modify_unknown_market_currencies() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::update_market(
                Origin::root(),
                SDOT,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::MarketDoesNotExist
        );
    })
}

#[test]
fn update_market_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Loans::market(DOT).unwrap().close_factor,
            Ratio::from_percent(50)
        );

        let market = MARKET_MOCK;
        assert_ok!(Loans::update_market(
            Origin::root(),
            DOT,
            None,
            None,
            None,
            Some(Default::default()),
            None,
            None,
            None,
            None,
        ));

        assert_eq!(Loans::market(DOT).unwrap().close_factor, Default::default());
        assert_eq!(Loans::market(DOT).unwrap().supply_cap, market.supply_cap);
    })
}

#[test]
fn update_market_should_not_work_if_with_invalid_params() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Loans::market(DOT).unwrap().close_factor,
            Ratio::from_percent(50)
        );

        // check error code while collateral_factor is [0%, 100%)
        assert_ok!(Loans::update_market(
            Origin::root(),
            DOT,
            Some(Ratio::zero()),
            None,
            None,
            Some(Default::default()),
            None,
            None,
            None,
            None,
        ));
        assert_noop!(
            Loans::update_market(
                Origin::root(),
                DOT,
                Some(Ratio::one()),
                None,
                None,
                Some(Default::default()),
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::InvalidFactor
        );
        // check error code while reserve_factor is 0% or bigger than 100%
        assert_noop!(
            Loans::update_market(
                Origin::root(),
                DOT,
                None,
                None,
                Some(Ratio::zero()),
                Some(Default::default()),
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::InvalidFactor
        );
        assert_noop!(
            Loans::update_market(
                Origin::root(),
                DOT,
                None,
                None,
                Some(Ratio::one()),
                Some(Default::default()),
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::InvalidFactor
        );
        // check error code while cap is zero
        assert_noop!(
            Loans::update_market(
                Origin::root(),
                DOT,
                None,
                None,
                None,
                Some(Default::default()),
                None,
                Some(Rate::from_inner(Rate::DIV / 100 * 90)),
                Some(Zero::zero()),
                None,
            ),
            Error::<Test>::InvalidSupplyCap
        );
    })
}

#[test]
fn update_rate_model_works() {
    new_test_ext().execute_with(|| {
        let new_rate_model = InterestRateModel::new_jump_model(
            Rate::saturating_from_rational(6, 100),
            Rate::saturating_from_rational(15, 100),
            Rate::saturating_from_rational(35, 100),
            Ratio::from_percent(80),
        );
        assert_ok!(Loans::update_rate_model(
            Origin::root(),
            DOT,
            new_rate_model,
        ));
        assert_eq!(Loans::market(DOT).unwrap().rate_model, new_rate_model);

        // Invalid base_rate
        assert_noop!(
            Loans::update_rate_model(
                Origin::root(),
                SDOT,
                InterestRateModel::new_jump_model(
                    Rate::saturating_from_rational(36, 100),
                    Rate::saturating_from_rational(15, 100),
                    Rate::saturating_from_rational(35, 100),
                    Ratio::from_percent(80),
                )
            ),
            Error::<Test>::InvalidRateModelParam
        );
        // Invalid jump_rate
        assert_noop!(
            Loans::update_rate_model(
                Origin::root(),
                SDOT,
                InterestRateModel::new_jump_model(
                    Rate::saturating_from_rational(5, 100),
                    Rate::saturating_from_rational(36, 100),
                    Rate::saturating_from_rational(37, 100),
                    Ratio::from_percent(80),
                )
            ),
            Error::<Test>::InvalidRateModelParam
        );
        // Invalid full_rate
        assert_noop!(
            Loans::update_rate_model(
                Origin::root(),
                SDOT,
                InterestRateModel::new_jump_model(
                    Rate::saturating_from_rational(5, 100),
                    Rate::saturating_from_rational(15, 100),
                    Rate::saturating_from_rational(57, 100),
                    Ratio::from_percent(80),
                )
            ),
            Error::<Test>::InvalidRateModelParam
        );
        // base_rate greater than jump_rate
        assert_noop!(
            Loans::update_rate_model(
                Origin::root(),
                SDOT,
                InterestRateModel::new_jump_model(
                    Rate::saturating_from_rational(10, 100),
                    Rate::saturating_from_rational(9, 100),
                    Rate::saturating_from_rational(14, 100),
                    Ratio::from_percent(80),
                )
            ),
            Error::<Test>::InvalidRateModelParam
        );
        // jump_rate greater than full_rate
        assert_noop!(
            Loans::update_rate_model(
                Origin::root(),
                SDOT,
                InterestRateModel::new_jump_model(
                    Rate::saturating_from_rational(5, 100),
                    Rate::saturating_from_rational(15, 100),
                    Rate::saturating_from_rational(14, 100),
                    Ratio::from_percent(80),
                )
            ),
            Error::<Test>::InvalidRateModelParam
        );
    })
}
