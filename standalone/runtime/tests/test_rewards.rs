mod mock;
use currency::Amount;
use mock::{
    assert_eq,
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    redeem_testing_utils::{ExecuteRedeemBuilder, RequestRedeemBuilder},
    reward_testing_utils::{BasicRewardPool, StakeHolder},
    *,
};
use vault_registry::DefaultVaultId;

const ESCROW_1: [u8; 32] = EVE;
const ESCROW_2: [u8; 32] = FRANK;

const VAULT_1: [u8; 32] = CAROL;
const VAULT_2: [u8; 32] = DAVE;

// issue fee is 0.15%
const ISSUE_FEE: f64 = 0.0015;

// redeem fee is 0.5%
const REDEEM_FEE: f64 = 0.005;

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
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));

            execute(currency_id)
        });
    };
    test_with(Token(KSM));
    test_with(Token(DOT));
    test_with(ForeignAsset(1));
}

fn withdraw_escrow_rewards(account_id: &AccountId, currency_id: CurrencyId) -> i128 {
    let amount = EscrowRewardsPallet::compute_reward(currency_id, account_id).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_escrow_rewards { currency_id }).dispatch(origin_of(account_id.clone())));
    amount
}

fn withdraw_vault_rewards(vault_id: &VaultId, currency_id: CurrencyId) -> i128 {
    let amount = VaultRewardsPallet::compute_reward(currency_id, vault_id).unwrap();
    assert_ok!(Call::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(vault_id.account_id.clone())));
    amount
}

fn distribute_block_rewards(amount: Balance) {
    assert_ok!(TokensPallet::set_balance(
        root(),
        FeeAccount::get(),
        DEFAULT_NATIVE_CURRENCY,
        TokensPallet::accounts(FeeAccount::get(), DEFAULT_NATIVE_CURRENCY).free + amount,
        0
    ));
    assert_ok!(
        <VaultRewardsPallet as Rewards<VaultId, Balance, CurrencyId>>::distribute_reward(
            amount,
            DEFAULT_NATIVE_CURRENCY
        )
    );
}

fn issue_with_vault(currency_id: CurrencyId, vault_id: &VaultId, request_amount: Amount<Runtime>) {
    let (issue_id, _) = RequestIssueBuilder::new(vault_id, request_amount)
        .with_vault(vault_id.clone())
        .request();
    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(vault_id.account_id.clone(), Some(currency_id))
        .assert_execute();
}

fn redeem_with_vault(vault_id: &VaultId, request_amount: Amount<Runtime>) {
    let (redeem_id, _) = RequestRedeemBuilder::new(vault_id, request_amount)
        .with_vault(vault_id.clone())
        .request();
    ExecuteRedeemBuilder::new(redeem_id).assert_execute();
}

#[test]
fn test_vault_rewards() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);
        let vault_id_2 = vault_id_of(VAULT_2, currency_id);
        let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
        let vault_2_stake_id = StakeHolder::Vault(vault_id_2.clone());

        issue_with_vault(currency_id, &vault_id_1, wrapped(20000));
        distribute_block_rewards(1000);
        issue_with_vault(currency_id, &vault_id_2, wrapped(80000));
        distribute_block_rewards(4000);

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(&vault_1_stake_id, 20000.0)
            .distribute(1000.0)
            .deposit_stake(&vault_2_stake_id, 80000.0)
            .distribute(4000.0);

        assert_eq_modulo_rounding!(
            withdraw_vault_rewards(&vault_id_1, DEFAULT_NATIVE_CURRENCY),
            reward_pool.compute_reward(&vault_1_stake_id) as i128
        );
        assert_eq_modulo_rounding!(
            withdraw_vault_rewards(&vault_id_2, DEFAULT_NATIVE_CURRENCY),
            reward_pool.compute_reward(&vault_2_stake_id) as i128
        );
    });
}

// only call once per account
fn escrow_set_balance(account_id: &AccountId, amount: Balance) {
    assert_ok!(Call::Tokens(TokensCall::set_balance {
        who: account_id.clone(),
        currency_id: DEFAULT_NATIVE_CURRENCY,
        new_free: amount,
        new_reserved: 0,
    })
    .dispatch(root()));

    let span = <Runtime as escrow::Config>::Span::get();
    let current_height = SystemPallet::block_number();

    assert_ok!(Call::Escrow(EscrowCall::create_lock {
        amount: amount,
        unlock_height: current_height + span
    })
    .dispatch(origin_of(account_id.clone())));
}

#[test]
fn test_escrow_issue_and_redeem_rewards() {
    test_with(|currency_id| {
        let vault_id_1 = vault_id_of(VAULT_1, currency_id);

        let escrow_1 = account_of(ESCROW_1);
        let escrow_2 = account_of(ESCROW_2);

        escrow_set_balance(&escrow_1, 1000000000000);
        escrow_set_balance(&escrow_2, 2000000000000);

        issue_with_vault(currency_id, &vault_id_1, wrapped(100000));
        redeem_with_vault(&vault_id_1, wrapped(50000));

        let mut reward_pool = BasicRewardPool::default();
        reward_pool
            .deposit_stake(&escrow_1, 1000000000000.0)
            .deposit_stake(&escrow_2, 2000000000000.0)
            .distribute(100000.0 * ISSUE_FEE)
            .distribute(50000.0 * REDEEM_FEE);

        assert_eq_modulo_rounding!(
            withdraw_escrow_rewards(&escrow_1, DEFAULT_WRAPPED_CURRENCY),
            reward_pool.compute_reward(&escrow_1) as i128
        );
        assert_eq_modulo_rounding!(
            withdraw_escrow_rewards(&escrow_2, DEFAULT_WRAPPED_CURRENCY),
            reward_pool.compute_reward(&escrow_2) as i128
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
