use crate::{mock::*, Commission, IssueFee};
use currency::Amount;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchResultWithPostInfo};
use primitives::VaultId;
use reward::RewardsApi;
use sp_arithmetic::FixedI128;
use sp_runtime::{DispatchError, FixedPointNumber, FixedU128};

type CapacityRewards = <Test as crate::Config>::CapacityRewards;
type VaultRewards = <Test as crate::Config>::VaultRewards;
type VaultStaking = <Test as crate::Config>::VaultStaking;

fn test_setter<F1, F2>(f: F1, get_storage_value: F2)
where
    F1: Fn(RuntimeOrigin, UnsignedFixedPoint) -> DispatchResultWithPostInfo,
    F2: Fn() -> UnsignedFixedPoint,
{
    run_test(|| {
        let large_value = UnsignedFixedPoint::checked_from_rational::<u128, u128>(101, 100).unwrap(); // 101%
        assert_noop!(f(RuntimeOrigin::root(), large_value), TestError::AboveMaxExpectedValue);

        let valid_value = UnsignedFixedPoint::checked_from_rational::<u128, u128>(100, 100).unwrap(); // 100%
        assert_noop!(f(RuntimeOrigin::signed(6), valid_value), DispatchError::BadOrigin);
        assert_ok!(f(RuntimeOrigin::root(), valid_value));
        assert_eq!(get_storage_value(), valid_value);
    })
}

#[test]
fn should_get_issue_fee() {
    run_test(|| {
        <IssueFee<Test>>::put(UnsignedFixedPoint::checked_from_rational(10, 100).unwrap());
        assert_ok!(
            Fee::get_issue_fee(&Amount::<Test>::new(100, Token(IBTC))),
            Amount::<Test>::new(10, Token(IBTC))
        );
    })
}

#[test]
fn should_set_issue_fee() {
    test_setter(Fee::set_issue_fee, Fee::issue_fee);
}

#[test]
fn should_set_issue_griefing_collateral() {
    test_setter(Fee::set_issue_griefing_collateral, Fee::issue_griefing_collateral);
}

#[test]
fn should_set_redeem_fee() {
    test_setter(Fee::set_redeem_fee, Fee::redeem_fee);
}

#[test]
fn should_set_premium_redeem_fee() {
    test_setter(Fee::set_premium_redeem_fee, Fee::premium_redeem_fee);
}

#[test]
fn should_set_punishment_fee() {
    test_setter(Fee::set_punishment_fee, Fee::punishment_fee);
}

#[test]
fn should_set_replace_griefing_collateral() {
    test_setter(Fee::set_replace_griefing_collateral, Fee::replace_griefing_collateral);
}

#[test]
fn compute_vault_rewards_works_with_commission() {
    run_test(|| {
        let _q: u128 = CapacityRewards::get_total_stake(&()).unwrap();
        let reward_currency = Token(KINT);
        let currency_1 = Token(KSM);
        let currency_2 = Token(DOT);

        let vault_id_1 = VaultId {
            account_id: 1,
            currencies: primitives::VaultCurrencyPair {
                collateral: currency_1,
                wrapped: Token(KBTC),
            },
        };
        let vault_id_2 = VaultId {
            account_id: 2,
            currencies: primitives::VaultCurrencyPair {
                collateral: currency_1,
                wrapped: Token(KBTC),
            },
        };

        // 50% stake in capacity rewards
        CapacityRewards::set_stake(&(), &currency_1, 1000u128).unwrap();
        CapacityRewards::set_stake(&(), &currency_2, 1000u128).unwrap();

        // 25% stake in reward pool
        VaultRewards::set_stake(&currency_1, &vault_id_1, 1000u128).unwrap();
        VaultRewards::set_stake(&currency_1, &vault_id_2, 3000u128).unwrap();

        // 12.5% in staking pool
        VaultStaking::set_stake(&(None, vault_id_1.clone()), &1, 1000u128).unwrap();
        VaultStaking::set_stake(&(None, vault_id_1.clone()), &2, 7000u128).unwrap();

        // set 6.25% commission
        let commission_rate = FixedU128::from_inner(FixedU128::DIV / 16);
        Commission::<Test>::set(&vault_id_1, Some(commission_rate));

        // distribute 1_024_000 tokens of rewards..
        let distributed_reward = Amount::new(1_024_000, reward_currency);
        Tokens::set_balance(
            RuntimeOrigin::root(),
            Fee::fee_pool_account_id(),
            distributed_reward.currency(),
            distributed_reward.amount(),
            0,
        )
        .unwrap();
        CapacityRewards::distribute_reward(
            &(),
            distributed_reward.currency(),
            FixedI128::from(i128::try_from(distributed_reward.amount()).unwrap()),
        )
        .unwrap();

        let nominator_1_share = FixedU128::from_inner(FixedU128::DIV / 64); // 50% * 25% * 12.5%
        let nominator_2_share = FixedU128::from_inner(7 * FixedU128::DIV / 64); // 50% * 25% * 87.5%

        let commission = distributed_reward * nominator_2_share * commission_rate;

        let vault_reward = distributed_reward * nominator_1_share + commission;
        let nominator_reward = distributed_reward * nominator_2_share - commission;

        assert_eq!(
            Fee::compute_vault_rewards(&vault_id_1, &1, reward_currency).unwrap(),
            vault_reward
        );
        assert_eq!(
            Fee::compute_vault_rewards(&vault_id_1, &2, reward_currency).unwrap(),
            nominator_reward
        );

        // if nominator withdraws, vault already receives commission to it's computed rewards
        // will be lower afterwards
        Fee::withdraw_vault_rewards(&vault_id_1, &2, None, reward_currency).unwrap();
        assert_eq!(
            Fee::compute_vault_rewards(&vault_id_1, &1, reward_currency).unwrap(),
            distributed_reward * nominator_1_share * (FixedU128::from(1) - commission_rate)
        );
    })
}
