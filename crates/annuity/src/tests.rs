use frame_support::traits::Currency;

/// Tests for Annuity
use crate::mock::*;

#[test]
fn should_calculate_emission_rewards() {
    run_test(|| {
        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_1_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), 12000);

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_2_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), 9000);

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_3_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), 6000);

        <Balances as Currency<AccountId>>::make_free_balance_be(&Annuity::account_id(), YEAR_4_REWARDS);
        Annuity::update_reward_per_block();
        assert_eq!(Annuity::reward_per_block(), 3000);
    })
}
