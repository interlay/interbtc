mod mock;
use currency::Amount;
use mock::{
    assert_eq,
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    nomination_testing_utils::*,
    reward_testing_utils::{BasicRewardPool, StakeHolder},
    *,
};
use vault_registry::DefaultVaultId;

const VAULT_1: [u8; 32] = CAROL;
const VAULT_2: [u8; 32] = DAVE;

// issue fee is 0.15%
const ISSUE_FEE: f64 = 0.0015;

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
    test_with(Token(KSM));
    test_with(Token(DOT));
}

fn withdraw_vault_global_pool_rewards(vault_id: &VaultId) -> i128 {
    let amount = VaultRewardsPallet::compute_reward(vault_id.wrapped_currency(), vault_id).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(vault_id.account_id.clone())));
    amount
}

fn withdraw_local_pool_rewards(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    let amount =
        staking::Pallet::<Runtime>::compute_reward(vault_id.wrapped_currency(), vault_id, nominator_id).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(nominator_id.clone())));
    amount
}

fn get_vault_global_pool_rewards(vault_id: &VaultId) -> i128 {
    VaultRewardsPallet::compute_reward(Token(IBTC), vault_id).unwrap()
}

fn get_local_pool_rewards(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    staking::Pallet::<Runtime>::compute_reward(vault_id.wrapped_currency(), vault_id, nominator_id).unwrap()
}

fn distribute_global_pool(vault_id: &VaultId) {
    FeePallet::distribute_from_reward_pool::<VaultRewardsPallet, staking::Pallet<Runtime>>(vault_id).unwrap();
}

fn get_vault_issued_tokens(vault_id: &VaultId) -> Amount<Runtime> {
    wrapped(VaultRegistryPallet::get_vault_from_id(vault_id).unwrap().issued_tokens)
}

fn get_vault_collateral(vault_id: &VaultId) -> Amount<Runtime> {
    VaultRegistryPallet::compute_collateral(vault_id)
        .unwrap()
        .try_into()
        .unwrap()
}

fn issue_with_vault(currency_id: CurrencyId, vault_id: &VaultId, request_amount: Amount<Runtime>) {
    let (issue_id, _) = RequestIssueBuilder::new(vault_id, request_amount)
        .with_vault(vault_id.clone())
        .request();
    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(vault_id.account_id.clone(), Some(currency_id))
        .assert_execute();
}

#[test]
fn test_vault_fee_pool_withdrawal() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_id_2 = vault_id_of(VAULT_2, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
        let vault_2_stake_id = StakeHolder::Vault(vault_id_2.clone());

        SecurityPallet::set_active_block_number(1);
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_vault(currency_id, &vault_id_1, wrapped(20000));
        issue_with_vault(currency_id, &vault_id_2, wrapped(80000));

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
            .distribute(20000.0 * ISSUE_FEE)
            .deposit_stake(&vault_2_stake_id, get_vault_issued_tokens(&vault_id_2).amount() as f64)
            .distribute(80000.0 * ISSUE_FEE);

        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(&vault_id_1),
            reward_pool.compute_reward(&vault_1_stake_id) as i128
        );
        assert_eq_modulo_rounding!(
            withdraw_vault_global_pool_rewards(&vault_id_2),
            reward_pool.compute_reward(&vault_2_stake_id) as i128
        );
    });
}

#[test]
fn test_new_nomination_withdraws_global_reward() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());

        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_vault(currency_id, &vault_id_1, wrapped(10000));

        assert_nomination_opt_in(&vault_id_1);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
            .distribute(10000.0 * ISSUE_FEE);

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(&vault_id_1),
            reward_pool.compute_reward(&vault_1_stake_id) as i128,
        );
        assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));

        // Vault rewards are withdrawn when a new nominator joins
        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(&vault_id_1), 0 as i128);
        assert_eq_modulo_rounding!(
            get_local_pool_rewards(&vault_id_1, &vault_id_1.account_id),
            reward_pool.compute_reward(&vault_1_stake_id) as i128
        );
    });
}

#[test]
fn test_fee_nomination() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
        let user_stake_id = StakeHolder::Nominator(account_of(USER));

        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_vault(currency_id, &vault_id_1, wrapped(10000));

        let vault_issued_tokens_1 = get_vault_issued_tokens(&vault_id_1);

        assert_nomination_opt_in(&vault_id_1);
        assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));
        issue_with_vault(currency_id, &vault_id_1, wrapped(100000));

        let vault_issued_tokens_2 = get_vault_issued_tokens(&vault_id_1);

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(&vault_1_stake_id, vault_issued_tokens_1.amount() as f64)
            .distribute(10000.0 * ISSUE_FEE);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_collateral(&vault_id_1).amount() as f64)
            .distribute(global_reward_pool.compute_reward(&vault_1_stake_id))
            .deposit_stake(&user_stake_id, DEFAULT_NOMINATION as f64);

        global_reward_pool
            .withdraw_reward(&vault_1_stake_id)
            .deposit_stake(
                &vault_1_stake_id,
                (vault_issued_tokens_2 - vault_issued_tokens_1).amount() as f64,
            )
            .distribute(100000.0 * ISSUE_FEE);

        local_reward_pool.distribute(global_reward_pool.compute_reward(&vault_1_stake_id));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(&vault_id_1),
            global_reward_pool.compute_reward(&vault_1_stake_id) as i128,
        );

        distribute_global_pool(&vault_id_1);

        assert_eq_modulo_rounding!(get_vault_global_pool_rewards(&vault_id_1), 0 as i128);

        assert_eq_modulo_rounding!(
            withdraw_local_pool_rewards(&vault_id_1, &vault_id_1.account_id),
            local_reward_pool.compute_reward(&vault_1_stake_id) as i128,
        );

        assert_eq_modulo_rounding!(
            withdraw_local_pool_rewards(&vault_id_1, &account_of(USER)),
            local_reward_pool.compute_reward(&user_stake_id) as i128,
        );
    });
}

#[test]
fn test_fee_nomination_slashing() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
        let user_stake_id = StakeHolder::Nominator(account_of(USER));

        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_vault(currency_id, &vault_id_1, wrapped(10000));

        assert_nomination_opt_in(&vault_id_1);
        assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));

        // Slash the vault and its nominator
        VaultRegistryPallet::transfer_funds(
            CurrencySource::Collateral(vault_id_1.clone()),
            CurrencySource::FreeBalance(account_of(VAULT_2)),
            &default_nomination(vault_id_1.collateral_currency()),
        )
        .unwrap();

        issue_with_vault(currency_id, &vault_id_1, wrapped(100000));

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
            .distribute(100000.0 * ISSUE_FEE);

        let vault_collateral = get_vault_collateral(&vault_id_1).amount() as f64;
        let nominator_collateral = DEFAULT_NOMINATION as f64;
        let slashed_amount = DEFAULT_NOMINATION as f64;
        // issue fee is 0.5%
        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(&vault_1_stake_id, vault_collateral)
            .deposit_stake(&user_stake_id, nominator_collateral)
            .withdraw_stake(
                &vault_1_stake_id,
                slashed_amount * vault_collateral / (vault_collateral + nominator_collateral),
            )
            .withdraw_stake(
                &user_stake_id,
                slashed_amount * nominator_collateral / (vault_collateral + nominator_collateral),
            )
            .distribute(global_reward_pool.compute_reward(&vault_1_stake_id));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(&vault_id_1),
            local_reward_pool.compute_reward(&vault_1_stake_id) as i128
                + local_reward_pool.compute_reward(&user_stake_id) as i128,
        );
    });
}

#[test]
fn test_fee_nomination_withdrawal() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
        let user_stake_id = StakeHolder::Nominator(account_of(USER));

        SecurityPallet::set_active_block_number(1);
        enable_nomination();
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

        issue_with_vault(currency_id, &vault_id_1, wrapped(10000));

        assert_nomination_opt_in(&vault_id_1);
        assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));
        issue_with_vault(currency_id, &vault_id_1, wrapped(50000));
        assert_withdraw_nominator_collateral(account_of(USER), &vault_id_1, default_nomination(currency_id) / 2);
        issue_with_vault(currency_id, &vault_id_1, wrapped(50000));

        let mut global_reward_pool = BasicRewardPool::default();
        global_reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
            .distribute(50000.0 * ISSUE_FEE);

        let mut local_reward_pool = BasicRewardPool::default();
        local_reward_pool
            .deposit_stake(&vault_1_stake_id, get_vault_collateral(&vault_id_1).amount() as f64)
            .deposit_stake(&user_stake_id, DEFAULT_NOMINATION as f64)
            .distribute(global_reward_pool.compute_reward(&user_stake_id))
            .withdraw_reward(&vault_1_stake_id)
            .withdraw_stake(&user_stake_id, (DEFAULT_NOMINATION / 2) as f64)
            .distribute(global_reward_pool.compute_reward(&vault_1_stake_id));

        assert_eq_modulo_rounding!(
            get_vault_global_pool_rewards(&vault_id_1),
            local_reward_pool.compute_reward(&vault_1_stake_id) as i128
                + local_reward_pool.compute_reward(&user_stake_id) as i128,
        );
    });
}

#[test]
fn integration_test_fee_with_parachain_shutdown_fails() {
    test_with(|currency_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);

        assert_noop!(
            Call::Fee(FeeCall::withdraw_rewards {
                vault_id: vault_id_1.clone(),
                index: None
            })
            .dispatch(origin_of(vault_id_1.account_id)),
            SystemError::CallFiltered
        );
    })
}
