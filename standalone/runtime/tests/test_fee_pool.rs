mod mock;
use currency::Amount;
use mock::{
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    nomination_testing_utils::*,
    reward_testing_utils::BasicRewardPool,
    *,
};
use staking::Staking;

const VAULT_1: [u8; 32] = CAROL;
const VAULT_2: [u8; 32] = DAVE;
const ISSUE_RELAYER: [u8; 32] = EVE;

// issue fee is 0.5%
const ISSUE_FEE: f64 = 0.005;

fn default_nomination(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_NOMINATION, currency_id)
}

// assert that a and b differ by at most 1
macro_rules! assert_eq_modulo_rounding {
    ($left:expr, $right:expr $(,)?) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                if (*left_val > *right_val && *left_val - *right_val > 5)
                    || (*right_val > *left_val && *right_val - *left_val > 5)
                {
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    panic!(
                        r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                        &*left_val, &*right_val
                    )
                }
            }
        }
    }};
}

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| execute(currency_id));
    };
    test_with(CurrencyId::KSM);
    test_with(CurrencyId::DOT);
}

fn withdraw_vault_global_pool_rewards(account: [u8; 32]) -> i128 {
    let amount = VaultRewardsPallet::compute_reward(KBTC, &account_of(account)).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_rewards(account_of(account))).dispatch(origin_of(account_of(account))));
    amount
}

fn withdraw_local_pool_rewards(pool_id: [u8; 32], account: [u8; 32]) -> i128 {
    let amount = staking::StakingCurrencyAdapter::<Runtime, GetWrappedCurrencyId>::compute_reward(
        &account_of(pool_id),
        &account_of(account),
    )
    .unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_rewards(account_of(pool_id))).dispatch(origin_of(account_of(account))));
    amount
}

fn get_vault_global_pool_rewards(account: [u8; 32]) -> i128 {
    VaultRewardsPallet::compute_reward(KBTC, &account_of(account)).unwrap()
}

fn get_local_pool_rewards(pool_id: [u8; 32], account: [u8; 32]) -> i128 {
    staking::StakingCurrencyAdapter::<Runtime, GetWrappedCurrencyId>::compute_reward(
        &account_of(pool_id),
        &account_of(account),
    )
    .unwrap()
}

fn distribute_global_pool(pool_id: [u8; 32]) {
    FeePallet::distribute_from_reward_pool::<
        reward::RewardsCurrencyAdapter<Runtime, (), GetWrappedCurrencyId>,
        staking::StakingCurrencyAdapter<Runtime, GetWrappedCurrencyId>,
    >(&account_of(pool_id))
    .unwrap();
}

fn get_vault_issued_tokens(account: [u8; 32]) -> Amount<Runtime> {
    wrapped(
        VaultRegistryPallet::get_vault_from_id(&account_of(account))
            .unwrap()
            .issued_tokens,
    )
}

fn get_vault_collateral(account: [u8; 32]) -> Amount<Runtime> {
    VaultRegistryPallet::compute_collateral(&account_of(account))
        .unwrap()
        .try_into()
        .unwrap()
}

fn issue_with_relayer_and_vault(
    currency_id: CurrencyId,
    relayer: [u8; 32],
    vault: [u8; 32],
    request_amount: Amount<Runtime>,
) {
    let (issue_id, _) = RequestIssueBuilder::new(currency_id, request_amount)
        .with_vault(vault)
        .request();
    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(vault, Some(currency_id))
        .with_relayer(Some(relayer))
        .assert_execute();
}

#[test]
fn test_vault_fee_pool_withdrawal() {
    test_with(|currency_id| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(20000));
        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_2, wrapped(80000));

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_issued_tokens(VAULT_1).amount() as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute(20000.0 * ISSUE_FEE)
            .deposit_stake(VAULT_2, get_vault_issued_tokens(VAULT_2).amount() as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute(80000.0 * ISSUE_FEE);

        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(VAULT_1),
            reward_pool.compute_reward(VAULT_1) as i128
        );
        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(VAULT_2),
            reward_pool.compute_reward(VAULT_2) as i128
        );
    });
}

#[test]
fn test_new_nomination_withdraws_global_reward() {
    test_with(|currency_id| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(10000));

        assert_nomination_opt_in(VAULT_1);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_issued_tokens(VAULT_1).amount() as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute(10000.0 * ISSUE_FEE);

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            reward_pool.compute_reward(VAULT_1) as i128,
        );
        assert_nominate_collateral(VAULT_1, USER, default_nomination(currency_id));

        // Vault rewards are withdrawn when a new nominator joins
        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(VAULT_1), 0 as i128);
        assert_eq_modulo_rounding!(
            get_local_pool_rewards(VAULT_1, VAULT_1),
            reward_pool.compute_reward(VAULT_1) as i128
        );
    });
}

#[test]
fn test_fee_nomination() {
    test_with(|currency_id| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(10000));

        let vault_issued_tokens_1 = get_vault_issued_tokens(VAULT_1);

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(VAULT_1, USER, default_nomination(currency_id));
        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(100000));

        let vault_issued_tokens_2 = get_vault_issued_tokens(VAULT_1);

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, vault_issued_tokens_1.amount() as f64)
            .distribute(10000.0 * ISSUE_FEE);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(VAULT_1, get_vault_collateral(VAULT_1).amount() as f64)
            .distribute(global_reward_pool.compute_reward(VAULT_1))
            .deposit_stake(USER, DEFAULT_NOMINATION as f64);

        global_reward_pool
            .withdraw_reward(VAULT_1)
            .deposit_stake(VAULT_1, (vault_issued_tokens_2 - vault_issued_tokens_1).amount() as f64)
            .distribute(100000.0 * ISSUE_FEE);

        local_reward_pool.distribute(global_reward_pool.compute_reward(VAULT_1));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            global_reward_pool.compute_reward(VAULT_1) as i128,
        );

        distribute_global_pool(VAULT_1);

        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(VAULT_1), 0 as i128);

        assert_eq_modulo_rounding!(
            withdraw_local_pool_rewards(VAULT_1, VAULT_1),
            local_reward_pool.compute_reward(VAULT_1) as i128,
        );

        assert_eq_modulo_rounding!(
            withdraw_local_pool_rewards(VAULT_1, USER),
            local_reward_pool.compute_reward(USER) as i128,
        );
    });
}

#[test]
fn test_fee_nomination_slashing() {
    test_with(|currency_id| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(10000));

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(VAULT_1, USER, default_nomination(currency_id));

        // Slash the vault and its nominator
        VaultRegistryPallet::transfer_funds(
            CurrencySource::Collateral(account_of(VAULT_1)),
            CurrencySource::FreeBalance(account_of(VAULT_2)),
            &default_nomination(VaultRegistryPallet::get_collateral_currency(&account_of(VAULT_1)).unwrap()),
        )
        .unwrap();

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(100000));

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, get_vault_issued_tokens(VAULT_1).amount() as f64)
            .distribute(100000.0 * ISSUE_FEE);

        let vault_collateral = get_vault_collateral(VAULT_1).amount() as f64;
        let nominator_collateral = DEFAULT_NOMINATION as f64;
        let slashed_amount = DEFAULT_NOMINATION as f64;
        // issue fee is 0.5%
        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(VAULT_1, vault_collateral)
            .deposit_stake(USER, nominator_collateral)
            .withdraw_stake(
                VAULT_1,
                slashed_amount * vault_collateral / (vault_collateral + nominator_collateral),
            )
            .withdraw_stake(
                USER,
                slashed_amount * nominator_collateral / (vault_collateral + nominator_collateral),
            )
            .distribute(global_reward_pool.compute_reward(VAULT_1));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            local_reward_pool.compute_reward(VAULT_1) as i128 + local_reward_pool.compute_reward(USER) as i128,
        );
    });
}

#[test]
fn test_fee_nomination_withdrawal() {
    test_with(|currency_id| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(10000));

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(VAULT_1, USER, default_nomination(currency_id));
        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(50000));
        assert_withdraw_nominator_collateral(USER, VAULT_1, default_nomination(currency_id) / 2);
        issue_with_relayer_and_vault(currency_id, ISSUE_RELAYER, VAULT_1, wrapped(50000));

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, get_vault_issued_tokens(VAULT_1).amount() as f64)
            .distribute(50000.0 * ISSUE_FEE);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(VAULT_1, get_vault_collateral(VAULT_1).amount() as f64)
            .deposit_stake(USER, DEFAULT_NOMINATION as f64)
            .distribute(global_reward_pool.compute_reward(VAULT_1))
            .withdraw_reward(VAULT_1)
            .withdraw_stake(USER, (DEFAULT_NOMINATION / 2) as f64)
            .distribute(global_reward_pool.compute_reward(VAULT_1));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            local_reward_pool.compute_reward(VAULT_1) as i128 + local_reward_pool.compute_reward(USER) as i128,
        );
    });
}

#[test]
fn integration_test_fee_with_parachain_shutdown_fails() {
    test_with(|_currency_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Fee(FeeCall::withdraw_rewards(account_of(ALICE))).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
