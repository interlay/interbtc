mod mock;
use mock::{
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    reward_testing_utils::RewardPool,
    *,
};

const VAULT_1: [u8; 32] = CAROL;
const VAULT_2: [u8; 32] = DAVE;
const ISSUE_RELAYER: [u8; 32] = EVE;
const RELAYER_1: [u8; 32] = FRANK;
const RELAYER_2: [u8; 32] = GRACE;

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
        SlaPallet::event_update_relayer_sla(&account_of(relayer), sla::types::RelayerEvent::StoreBlock).unwrap();
    }
}

// assert that a and b differ by at most 1
macro_rules! assert_eq_modulo_rounding {
    ($left:expr, $right:expr $(,)?) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                if (*left_val > *right_val && *left_val - *right_val > 1)
                    || (*right_val > *left_val && *right_val - *left_val > 1)
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

fn get_vault_rewards(account: [u8; 32]) -> i128 {
    let amount = RewardWrappedVaultPallet::compute_reward(&account_of(account)).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_vault_wrapped_rewards()).dispatch(origin_of(account_of(account))));
    amount
}

fn get_relayer_rewards(account: [u8; 32]) -> i128 {
    let amount = RewardWrappedRelayerPallet::compute_reward(&account_of(account)).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_relayer_wrapped_rewards()).dispatch(origin_of(account_of(account))));
    amount
}

fn get_vault_sla(account: [u8; 32]) -> i128 {
    SlaPallet::vault_sla(account_of(account))
        .into_inner()
        .checked_div(FixedI128::accuracy())
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

        // issue fee is 0.5%
        let mut reward_pool = RewardPool::default();
        reward_pool
            .deposit_stake(VAULT_1, get_vault_sla(VAULT_1) as f64)
            .distribute((20000.0 * 0.005) * 0.7) // set at 70% in tests
            .deposit_stake(VAULT_2, get_vault_sla(VAULT_2) as f64)
            .distribute((80000.0 * 0.005) * 0.7);

        assert_eq_modulo_rounding!(get_vault_rewards(VAULT_1), reward_pool.compute_reward(VAULT_1) as i128);
        assert_eq_modulo_rounding!(get_vault_rewards(VAULT_2), reward_pool.compute_reward(VAULT_2) as i128);
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

        let mut reward_pool = RewardPool::default();
        reward_pool
            .deposit_stake(RELAYER_1, 20.0)
            .deposit_stake(RELAYER_2, 40.0)
            .distribute((100000.0 * 0.005) * 0.2); // set at 20% in tests

        // first relayer gets 33% of the pool
        assert_eq_modulo_rounding!(
            get_relayer_rewards(RELAYER_1),
            reward_pool.compute_reward(RELAYER_1) as i128
        );
        // second relayer gets the remaining 66%
        assert_eq_modulo_rounding!(
            get_relayer_rewards(RELAYER_2),
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

        let maintainer_rewards = (100000.0 * 0.005) * 0.1; // set at 10% in tests
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
            Call::Fee(FeeCall::withdraw_vault_collateral_rewards()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Fee(FeeCall::withdraw_vault_wrapped_rewards()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Fee(FeeCall::withdraw_relayer_collateral_rewards()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Fee(FeeCall::withdraw_relayer_wrapped_rewards()).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
