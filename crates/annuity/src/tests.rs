use frame_support::{assert_ok, traits::Currency};

/// Tests for Annuity
use crate::mock::*;

#[test]
fn should_calculate_emission_rewards() {
    run_test(|| {
        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_1_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), unit(12000));

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_2_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), unit(9000));

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_3_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), unit(6000));

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_4_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), unit(3000));
    })
}

#[test]
fn should_set_reward_per_wrapped() {
    run_test(|| {
        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_1_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::min_reward_per_block(), unit(12000));
        assert_ok!(Annuity::set_reward_per_wrapped(Origin::root(), 100));
        assert_eq!(Annuity::min_reward_per_block(), 10000000000);
    })
}
