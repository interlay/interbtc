mod mock;
use mock::{
    issue_testing_utils::{self, execute_issue, execute_refund, request_issue, ExecuteIssueBuilder},
    redeem_testing_utils::{cancel_redeem, setup_cancelable_redeem},
    *,
};

const USER: [u8; 32] = issue_testing_utils::USER;
const VAULT: [u8; 32] = issue_testing_utils::VAULT;
const PROOF_SUBMITTER: [u8; 32] = issue_testing_utils::PROOF_SUBMITTER;

// Would have been a const, but `FixedI128::from` is not a const fn
fn initial_sla() -> FixedI128 {
    FixedI128::from(60)
}

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();

        SlaModule::set_vault_sla(&account_of(VAULT), initial_sla());
        SlaModule::set_vault_sla(&account_of(PROOF_SUBMITTER), initial_sla());

        execute()
    })
}

#[test]
fn test_sla_increase_for_issue() {
    test_with(|| {
        let (issue_id, issue) = request_issue(1000);
        execute_issue(issue_id);

        // check the sla increase for processing the issue
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(1000, issue.amount + issue.fee).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(VAULT)),
            initial_sla() + expected_sla_increase
        );
    })
}

#[test]
fn test_sla_increase_for_proof_submitter() {
    test_with(|| {
        let (issue_id, _) = request_issue(1000);
        execute_issue(issue_id);

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(PROOF_SUBMITTER)),
            initial_sla() + SlaModule::vault_submitted_issue_proof()
        );
    })
}

#[test]
fn test_sla_increase_for_submitting_proof_for_issue_against_self() {
    test_with(|| {
        // vault receives issue & executes it himself. Should get both SLA rewards

        let (issue_id, issue) = request_issue(1000);
        ExecuteIssueBuilder::new(issue_id)
            .with_submitter(VAULT, true)
            .assert_execute();

        let expected_sla_increase_for_issue = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(1000, issue.amount + issue.fee).unwrap();
        let expected_sla_increase_for_proof_submission = SlaModule::vault_submitted_issue_proof();

        // check that the vault who submitted the proof is rewarded with both SLA rewards
        assert_eq!(
            SlaModule::vault_sla(account_of(VAULT)),
            initial_sla() + expected_sla_increase_for_issue + expected_sla_increase_for_proof_submission
        );
    })
}

#[test]
fn test_sla_increase_for_refund() {
    test_with(|| {
        let (issue_id, issue) = request_issue(1000);

        // make sure we don't have enough collateral to fulfil the overpayment
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: 2000,
                ..CoreVaultData::vault(VAULT)
            },
        );

        // overpay by a factor of 4
        ExecuteIssueBuilder::new(issue_id)
            .with_amount(4 * (issue.amount + issue.fee))
            .assert_execute();

        let expected_sla_increase_for_issue = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(1000, issue.amount + issue.fee).unwrap();

        // check that the vault who submitted the proof is rewarded for issue
        assert_eq!(
            SlaModule::vault_sla(account_of(VAULT)),
            initial_sla() + expected_sla_increase_for_issue
        );

        // perform the refund
        execute_refund(VAULT);

        let expected_sla_increase_for_refund = SlaModule::vault_refunded();
        assert_eq!(
            SlaModule::vault_sla(account_of(VAULT)),
            initial_sla() + expected_sla_increase_for_issue + expected_sla_increase_for_refund
        );
    })
}

#[test]
fn test_sla_decrease_for_redeem_failure() {
    test_with(|| {
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());

        let redeem_id = setup_cancelable_redeem(USER, VAULT, 10_000, 1_000);

        cancel_redeem(redeem_id, USER, true);

        // sla should have decreased, but not below 0
        let expected_sla = FixedI128::max(
            FixedI128::zero(),
            initial_sla() + SlaModule::vault_redeem_failure_sla_change(),
        );
        assert_eq!(SlaModule::vault_sla(account_of(VAULT)), expected_sla);
    })
}

#[test]
fn test_sla_remains_unchanged_when_liquidated() {
    test_with(|| {
        let (issue_id, _) = request_issue(1000);

        drop_exchange_rate_and_liquidate(VAULT);

        execute_issue(issue_id);

        // sla remains unchanged if vault has been liquidated
        assert_eq!(SlaModule::vault_sla(account_of(VAULT)), initial_sla());
    })
}

#[test]
fn test_sla_increase_for_underpayed_issue() {
    test_with(|| {
        let (issue_id, issue) = request_issue(4_000);

        // only pay 25%
        ExecuteIssueBuilder::new(issue_id)
            .with_amount((issue.amount + issue.fee) / 4)
            .with_submitter(USER, false)
            .assert_execute();

        // check the sla increase
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(4000, issue.amount + issue.fee).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(VAULT)),
            initial_sla() + expected_sla_increase
        );
    });
}
