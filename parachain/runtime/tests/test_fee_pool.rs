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

fn test_with(execute: impl Fn(Currency)) {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        setup_backing_reward();
        execute(Currency::Backing);
    });
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        execute(Currency::Issuing);
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
        SlaPallet::event_update_relayer_sla(&account_of(relayer), sla::types::RelayerEvent::BlockSubmission).unwrap();
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
    Backing,
    Issuing,
}

fn get_epoch_rewards(currency: Currency) -> u128 {
    match currency {
        Currency::Backing => FeePallet::epoch_rewards_backing(),
        Currency::Issuing => FeePallet::epoch_rewards_issuing(),
    }
}

fn get_rewards(currency: Currency, account: [u8; 32]) -> u128 {
    match currency {
        Currency::Backing => {
            let amount = FeePallet::get_backing_rewards(&account_of(account));
            assert_noop!(
                Call::Fee(FeeCall::withdraw_backing(amount + 1)).dispatch(origin_of(account_of(account))),
                FeeError::InsufficientFunds,
            );
            assert_ok!(Call::Fee(FeeCall::withdraw_backing(amount)).dispatch(origin_of(account_of(account))));
            amount
        }
        Currency::Issuing => {
            let amount = FeePallet::get_issuing_rewards(&account_of(account));
            assert_noop!(
                Call::Fee(FeeCall::withdraw_backing(amount + 1)).dispatch(origin_of(account_of(account))),
                FeeError::InsufficientFunds,
            );
            assert_ok!(Call::Fee(FeeCall::withdraw_issuing(amount)).dispatch(origin_of(account_of(account))));
            amount
        }
    }
}

fn setup_backing_reward() {
    VaultRegistryPallet::transfer_funds(
        CurrencySource::FreeBalance(account_of(FAUCET)),
        CurrencySource::FreeBalance(FeePallet::fee_pool_account_id()),
        1000,
    )
    .unwrap();
    FeePallet::increase_backing_rewards_for_epoch(1000);
}

fn set_issued_and_backing(vault: [u8; 32], amount_issued: u128, backing: u128) {
    SecurityPallet::set_active_block_number(1);

    // we want issued to be 100 times amount_issued, _including fees_
    let request_amount = 100 * amount_issued;

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
    VaultRegistryPallet::transfer_funds(
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
        assert_ok!(FeePallet::update_rewards_for_epoch());

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
        assert_ok!(FeePallet::update_rewards_for_epoch());

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
        assert_ok!(FeePallet::update_rewards_for_epoch());

        set_issued_and_backing(VAULT2, 800, 200);

        let epoch_2_rewards = get_epoch_rewards(currency);
        let vault_epoch_2_rewards = (epoch_2_rewards * 70) / 100; // set at 70% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeePallet::update_rewards_for_epoch());

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

fn reset_relayer_sla(account: AccountId) {
    let tmp = SlaPallet::relayer_correct_theft_report();

    assert_ok!(Call::Sla(SlaCall::set_relayer_sla(
        sla::types::RelayerEvent::CorrectTheftReport,
        FixedI128::from(-100)
    ))
    .dispatch(root()));

    SlaPallet::event_update_relayer_sla(&account, sla::types::RelayerEvent::CorrectTheftReport).unwrap();

    // restore value
    assert_ok!(Call::Sla(SlaCall::set_relayer_sla(
        sla::types::RelayerEvent::CorrectTheftReport,
        tmp
    ))
    .dispatch(root()));
}

#[test]
fn test_relayer_fee_pool_withdrawal() {
    test_with(|currency| {
        set_issued_and_backing(VAULT1, 1000, 1000);

        // make the used relayer irrelevant in fee calculations
        reset_relayer_sla(account_of(ISSUE_RELAYER));

        setup_relayer(RELAYER_1, 20, 100);
        setup_relayer(RELAYER_2, 40, 200);

        let epoch_rewards = get_epoch_rewards(currency);
        let relayer_rewards = (epoch_rewards * 20) / 100; // set at 20% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeePallet::update_rewards_for_epoch());

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
        assert_ok!(FeePallet::update_rewards_for_epoch());

        assert_eq_modulo_rounding!(get_rewards(currency, MAINTAINER), maintainer_rewards);
    })
}

#[test]
fn integration_test_fee_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Fee(FeeCall::withdraw_issuing(0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Fee(FeeCall::withdraw_backing(0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
