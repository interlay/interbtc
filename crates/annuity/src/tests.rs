use frame_support::{assert_ok, traits::Currency};

/// Tests for Annuity
use crate::mock::*;

#[test]
fn should_calculate_emission_rewards() {
    run_test(|| {
        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_1_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(
            Annuity::reward_per_block(),
            YEAR_1_REWARDS / EmissionPeriod::get() as u128
        );

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_2_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(
            Annuity::reward_per_block(),
            YEAR_2_REWARDS / EmissionPeriod::get() as u128
        );

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_3_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(
            Annuity::reward_per_block(),
            YEAR_3_REWARDS / EmissionPeriod::get() as u128
        );

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_4_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(
            Annuity::reward_per_block(),
            YEAR_4_REWARDS / EmissionPeriod::get() as u128
        );
    })
}

#[test]
fn should_set_reward_per_wrapped() {
    run_test(|| {
        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_1_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(
            Annuity::min_reward_per_block(),
            YEAR_1_REWARDS / EmissionPeriod::get() as u128
        );
        let reward_per_wrapped = 100;
        assert_ok!(Annuity::set_reward_per_wrapped(
            RuntimeOrigin::root(),
            reward_per_wrapped
        ));
        assert_eq!(
            Annuity::min_reward_per_block(),
            reward_per_wrapped * TotalWrapped::get()
        );
    })
}
