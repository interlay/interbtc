mod mock;
use mock::{
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    *,
};

const PROOF_SUBMITTER: [u8; 32] = BOB;
const VAULT1: [u8; 32] = CAROL;
const VAULT2: [u8; 32] = DAVE;
const ISSUE_RELAYER: [u8; 32] = EVE;
const RELAYER_1: [u8; 32] = FRANK;
const RELAYER_2: [u8; 32] = GRACE;

fn test_with(execute: impl Fn(Currency) -> ()) {
    ExtBuilder::build().execute_with(|| {
        setup_dot_reward();
        execute(Currency::DOT);
    });
    ExtBuilder::build().execute_with(|| {
        execute(Currency::PolkaBTC);
    });
}

fn setup_relayer(relayer: [u8; 32], sla: u32, stake: u128) {
    UserData::force_to(
        relayer,
        UserData {
            free_balance: stake,
            ..Default::default()
        },
    );
    // register as staked relayer
    assert_ok!(Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(stake))
        .dispatch(origin_of(account_of(relayer))));
    for _ in 0..sla {
        SlaModule::event_update_relayer_sla(&account_of(relayer), sla::types::RelayerEvent::BlockSubmission).unwrap();
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

#[derive(Copy, Clone)]
enum Currency {
    DOT,
    PolkaBTC,
}

fn get_epoch_rewards(currency: Currency) -> u128 {
    match currency {
        Currency::DOT => FeeModule::epoch_rewards_dot(),
        Currency::PolkaBTC => FeeModule::epoch_rewards_polka_btc(),
    }
}

fn get_rewards(currency: Currency, account: [u8; 32]) -> u128 {
    match currency {
        Currency::DOT => {
            let amount = FeeModule::get_dot_rewards(&account_of(account));
            assert_noop!(
                Call::Fee(FeeCall::withdraw_dot(amount + 1)).dispatch(origin_of(account_of(account))),
                FeeError::InsufficientFunds,
            );
            assert_ok!(Call::Fee(FeeCall::withdraw_dot(amount)).dispatch(origin_of(account_of(account))));
            amount
        }
        Currency::PolkaBTC => {
            let amount = FeeModule::get_polka_btc_rewards(&account_of(account));
            assert_noop!(
                Call::Fee(FeeCall::withdraw_dot(amount + 1)).dispatch(origin_of(account_of(account))),
                FeeError::InsufficientFunds,
            );
            assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(amount)).dispatch(origin_of(account_of(account))));
            amount
        }
    }
}

fn setup_dot_reward() {
    VaultRegistryModule::slash_collateral(
        CurrencySource::FreeBalance(account_of(FAUCET)),
        CurrencySource::FreeBalance(FeeModule::fee_pool_account_id()),
        1000,
    )
    .unwrap();
    FeeModule::increase_dot_rewards_for_epoch(1000);
}

fn set_issued_and_backing(vault: [u8; 32], amount_issued: u128, backing: u128) {
    // we want issued to be 100 times amount_issued, _including fees_
    let amount_issued = 100 * amount_issued;
    let fee = FeeModule::get_issue_fee_from_total(amount_issued).unwrap();
    let request_amount = amount_issued - fee;

    let (issue_id, _) = RequestIssueBuilder::new(request_amount).with_vault(vault).request();
    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(PROOF_SUBMITTER, true)
        .with_relayer(Some(ISSUE_RELAYER))
        .assert_execute();

    CoreVaultData::force_to(
        vault,
        CoreVaultData {
            backing_collateral: 100 * backing,
            ..CoreVaultData::vault(vault)
        },
    );
    VaultRegistryModule::slash_collateral(
        CurrencySource::Backing(account_of(PROOF_SUBMITTER)),
        CurrencySource::FreeBalance(account_of(FAUCET)),
        CurrencySource::<Runtime>::Backing(account_of(PROOF_SUBMITTER))
            .current_balance()
            .unwrap(),
    )
    .unwrap();
}

#[test]
fn test_vault_fee_pool_withdrawal() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 200, 800);
        set_issued_and_backing(VAULT2, 800, 200);

        let epoch_rewards = get_epoch_rewards(currency);
        let vault_rewards = (epoch_rewards * 70) / 100; // set at 70% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 26% of the vault pool (20% of the 90% awarded by issued,
        // and 80% of the 10% awarded by collateral
        assert_eq_modulo_rounding!(get_rewards(currency, VAULT1), (vault_rewards * 26) / 100);
        // second vault gets the other 74%
        assert_eq_modulo_rounding!(get_rewards(currency, VAULT2), (vault_rewards * 74) / 100);
    })
}

#[test]
fn test_vault_fee_pool_withdrawal_with_liquidated_vaults() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 200, 800);
        set_issued_and_backing(VAULT2, 800, 200);

        drop_exchange_rate_and_liquidate(VAULT2);

        let epoch_rewards = get_epoch_rewards(currency);
        let vault_rewards = (epoch_rewards * 70) / 100; // set at 70% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 100% of the vault pool
        assert_eq_modulo_rounding!(get_rewards(currency, VAULT1), vault_rewards);
        // second vault gets nothing
        assert_eq_modulo_rounding!(get_rewards(currency, VAULT2), 0);
    })
}

#[test]
fn test_vault_fee_pool_withdrawal_over_multiple_epochs() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 200, 800);

        let epoch_1_rewards = get_epoch_rewards(currency);
        let vault_epoch_1_rewards = (epoch_1_rewards * 70) / 100; // set at 70% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        set_issued_and_backing(VAULT2, 800, 200);

        let epoch_2_rewards = get_epoch_rewards(currency);
        let vault_epoch_2_rewards = (epoch_2_rewards * 70) / 100; // set at 70% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets all of vault_epoch_1_rewards, plus 26% of the
        // vault_epoch_2_rewards (20% of the 90% awarded by issued,
        // and 80% of the 10% awarded by collateral
        assert_eq_modulo_rounding!(
            get_rewards(currency, VAULT1),
            vault_epoch_1_rewards + (vault_epoch_2_rewards * 26) / 100,
        );
        // second vault gets the other 74%
        assert_eq_modulo_rounding!(get_rewards(currency, VAULT2), (vault_epoch_2_rewards * 74) / 100,);
    })
}

#[test]
fn test_relayer_fee_pool_withdrawal() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 1000, 1000);

        // make the used relayer irrelevant in fee calculations
        SlaModule::event_update_relayer_sla(
            &account_of(ISSUE_RELAYER),
            sla::types::RelayerEvent::FalseInvalidVoteOrReport,
        )
        .unwrap();

        setup_relayer(RELAYER_1, 20, 100);
        setup_relayer(RELAYER_2, 40, 200);

        let epoch_rewards = get_epoch_rewards(currency);
        let relayer_rewards = (epoch_rewards * 20) / 100; // set at 20% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 20% of the vault pool
        assert_eq_modulo_rounding!(get_rewards(currency, RELAYER_1), (relayer_rewards * 20) / 100,);
        // second vault gets the other 80%
        assert_eq_modulo_rounding!(get_rewards(currency, RELAYER_2), (relayer_rewards * 80) / 100,);
    })
}

#[test]
fn test_maintainer_fee_pool_withdrawal() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 1000, 1000);

        let epoch_rewards = get_epoch_rewards(currency);
        let maintainer_rewards = (epoch_rewards * 10) / 100; // set at 10% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        assert_eq_modulo_rounding!(get_rewards(currency, MAINTAINER), maintainer_rewards);
    })
}
