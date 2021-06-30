mod mock;
use mock::{
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    nomination_testing_utils::*,
    reward_testing_utils::{BasicRewardPool, MAINTAINER_REWARDS, VAULT_REWARDS},
    *,
};

const VAULT_1: [u8; 32] = CAROL;
const VAULT_2: [u8; 32] = DAVE;
const ISSUE_RELAYER: [u8; 32] = EVE;
const RELAYER_1: [u8; 32] = FRANK;
const RELAYER_2: [u8; 32] = GRACE;

// issue fee is 0.5%
const ISSUE_FEE: f64 = 0.005;

fn setup_relayer(relayer: [u8; 32], sla: u32, stake: u128) {
    UserData::force_to(
        relayer,
        UserData {
            free_balance: stake,
            ..Default::default()
        },
    );
    // increase sla for block submission
    for _ in 0..sla {
        SlaPallet::event_update_vault_sla(&account_of(relayer), sla::Action::StoreBlock).unwrap();
    }
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

fn withdraw_vault_global_pool_rewards(account: [u8; 32]) -> i128 {
    let amount = RewardVaultPallet::compute_reward(INTERBTC, &RewardPool::Global, &account_of(account)).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_vault_rewards()).dispatch(origin_of(account_of(account))));
    amount
}

fn withdraw_local_pool_rewards(pool_id: [u8; 32], account: [u8; 32]) -> i128 {
    let amount =
        RewardVaultPallet::compute_reward(INTERBTC, &RewardPool::Local(account_of(pool_id)), &account_of(account))
            .unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_vault_rewards()).dispatch(origin_of(account_of(account))));
    amount
}

fn get_vault_global_pool_rewards(account: [u8; 32]) -> i128 {
    RewardVaultPallet::compute_reward(INTERBTC, &RewardPool::Global, &account_of(account)).unwrap()
}

fn get_local_pool_rewards(pool_id: [u8; 32], account: [u8; 32]) -> i128 {
    RewardVaultPallet::compute_reward(INTERBTC, &RewardPool::Local(account_of(pool_id)), &account_of(account)).unwrap()
}

fn distribute_global_pool(pool_id: [u8; 32]) {
    FeePallet::distribute_global_pool::<reward::RewardsCurrencyAdapter<Runtime, (), GetCollateralCurrencyId>>(
        &account_of(pool_id),
    )
    .unwrap();
    FeePallet::distribute_global_pool::<reward::RewardsCurrencyAdapter<Runtime, (), GetWrappedCurrencyId>>(
        &account_of(pool_id),
    )
    .unwrap();
}

fn get_vault_sla(account: [u8; 32]) -> i128 {
    SlaPallet::vault_sla(account_of(account))
        .into_inner()
        .checked_div(FixedI128::accuracy())
        .unwrap()
}

fn get_vault_collateral(account: [u8; 32]) -> i128 {
    VaultRegistryPallet::get_vault_from_id(&account_of(account))
        .unwrap()
        .collateral
        .try_into()
        .unwrap()
}

fn issue_with_relayer_and_vault(relayer: [u8; 32], vault: [u8; 32], request_amount: u128) {
    let (issue_id, _) = RequestIssueBuilder::new(request_amount).with_vault(vault).request();
    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(vault, true)
        .with_relayer(Some(relayer))
        .assert_execute();
}

#[test]
fn test_vault_fee_pool_withdrawal() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 20000);
        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_2, 80000);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute((20000.0 * ISSUE_FEE) * VAULT_REWARDS)
            .deposit_stake(VAULT_2, get_vault_sla(VAULT_2) as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute((80000.0 * ISSUE_FEE) * VAULT_REWARDS);

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
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 10000);

        assert_nomination_opt_in(VAULT_1);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(ISSUE_RELAYER, 7.0)
            .distribute((10000.0 * ISSUE_FEE) * VAULT_REWARDS);

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            reward_pool.compute_reward(VAULT_1) as i128,
        );
        assert_nominate_collateral(USER, VAULT_1, DEFAULT_NOMINATION);

        // Vault rewards are withdrawn when a new nominator joins
        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(VAULT_1), 0 as i128,);
        assert_eq_modulo_rounding!(get_local_pool_rewards(VAULT_1, VAULT_1), 0 as i128,);
    });
}

#[test]
fn test_fee_nomination() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 10000);

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(USER, VAULT_1, DEFAULT_NOMINATION);
        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 100000);

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(ISSUE_RELAYER, get_vault_sla(ISSUE_RELAYER) as f64)
            .distribute((100000.0 * ISSUE_FEE) * VAULT_REWARDS);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(VAULT_1, get_vault_collateral(VAULT_1) as f64)
            .deposit_stake(USER, DEFAULT_NOMINATION as f64)
            .distribute(global_reward_pool.compute_reward(VAULT_1));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(VAULT_1),
            local_reward_pool.compute_reward(VAULT_1) as i128 + local_reward_pool.compute_reward(USER) as i128,
        );

        distribute_global_pool(VAULT_1);

        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(VAULT_1), 0 as i128,);

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
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 10000);

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(USER, VAULT_1, DEFAULT_NOMINATION);

        // Slash the vault and its nominator
        VaultRegistryPallet::transfer_funds(
            CurrencySource::Collateral(account_of(VAULT_1)),
            CurrencySource::FreeBalance(account_of(VAULT_2)),
            DEFAULT_NOMINATION,
        )
        .unwrap();

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 100000);

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(ISSUE_RELAYER, get_vault_sla(ISSUE_RELAYER) as f64)
            .distribute((100000.0 * ISSUE_FEE) * VAULT_REWARDS);

        let vault_collateral = get_vault_collateral(VAULT_1) as f64;
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
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 10000);

        assert_nomination_opt_in(VAULT_1);
        assert_nominate_collateral(USER, VAULT_1, DEFAULT_NOMINATION);
        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 50000);
        assert_withdraw_nominator_collateral(USER, VAULT_1, DEFAULT_NOMINATION / 2);
        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 50000);

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(ISSUE_RELAYER, get_vault_sla(ISSUE_RELAYER) as f64)
            .distribute((50000.0 * ISSUE_FEE) * VAULT_REWARDS);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(VAULT_1, get_vault_collateral(VAULT_1) as f64)
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
fn test_relayer_fee_pool_withdrawal() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        setup_relayer(RELAYER_1, 20, 100);
        setup_relayer(RELAYER_2, 33, 200); // 33 + 7 = 40

        // RELAYER_2 initializes the relay and submits 6 blocks
        issue_with_relayer_and_vault(RELAYER_2, VAULT_1, 100000);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .deposit_stake(RELAYER_1, 20.0)
            .deposit_stake(RELAYER_2, 40.0)
            .distribute((100000.0 * ISSUE_FEE) * VAULT_REWARDS);

        // first relayer gets 33% of the pool
        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(RELAYER_1),
            reward_pool.compute_reward(RELAYER_1) as i128
        );
        // second relayer gets the remaining 66%
        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(RELAYER_2),
            reward_pool.compute_reward(RELAYER_2) as i128
        );
    });
}

#[test]
fn test_maintainer_fee_pool_withdrawal() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));

        issue_with_relayer_and_vault(ISSUE_RELAYER, VAULT_1, 100000);

        let maintainer_rewards = (100000.0 * ISSUE_FEE) * MAINTAINER_REWARDS;
        let maintainer_account_id = FeePallet::maintainer_account_id();
        let maintainer_balance = TreasuryPallet::get_free_balance(&maintainer_account_id);

        assert_eq_modulo_rounding!(maintainer_balance, maintainer_rewards as u128);
    });
}

#[test]
fn integration_test_fee_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Fee(FeeCall::withdraw_vault_rewards()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
