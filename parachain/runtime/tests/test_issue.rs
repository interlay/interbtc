mod mock;

use issue::IssueRequest;
use mock::*;
use primitive_types::H256;
use sp_runtime::AccountId32;
use std::convert::TryFrom;

type IssueCall = issue::Call<Runtime>;
type IssueModule = issue::Module<Runtime>;
type IssueEvent = issue::Event<Runtime>;
type IssueError = issue::Error<Runtime>;

type RefundCall = refund::Call<Runtime>;
type RefundModule = refund::Module<Runtime>;
type RefundEvent = refund::Event<Runtime>;

const USER: [u8; 32] = ALICE;
const VAULT: [u8; 32] = BOB;
const PROOF_SUBMITTER: [u8; 32] = CAROL;
pub const DEFAULT_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_GRIEFING_COLLATERAL: u128 = 5_000;

pub const DEFAULT_USER_FREE_BALANCE: u128 = 1_000_000;
pub const DEFAULT_USER_LOCKED_BALANCE: u128 = 100_000;
pub const DEFAULT_USER_FREE_TOKENS: u128 = 1000;
pub const DEFAULT_USER_LOCKED_TOKENS: u128 = 1000;
fn assert_issue_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().find(|record| match record.event {
        Event::issue(IssueEvent::RequestIssue(_, _, _, _, _, _, _, _)) => true,
        _ => false,
    });
    let id = if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _, _, _, _)) =
        record.unwrap().event
    {
        id
    } else {
        panic!("request issue event not found")
    };
    id
}

fn assert_refund_request_event() -> H256 {
    SystemModule::events()
        .iter()
        .find_map(|record| match record.event {
            Event::refund(RefundEvent::RequestRefund(id, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .expect("request refund event not found")
}

#[test]
fn integration_test_issue_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, account_of(BOB), 0))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );

        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                H256([0; 32]),
                H256Le::zero(),
                vec![0u8; 32],
                vec![0u8; 32]
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let user = ALICE;
        let vault = BOB;
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault_proof_submitter))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(
            Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(vault_proof_submitter)))
        );

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // check the sla increase
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(amount_btc, total_amount_btc).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            expected_sla_increase
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral reimbursed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc minted
        assert_eq!(final_btc_balance, initial_btc_balance + amount_btc);

        // vault should have 0 to-be-issued tokens
        assert_eq!(
            VaultRegistryModule::get_vault_from_id(&account_of(vault))
                .unwrap()
                .to_be_issued_tokens,
            0
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(vault))
        ))
        .dispatch(origin_of(account_of(vault))));
    });
}

#[test]
fn integration_test_withdraw_after_request_issue() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let vault = BOB;
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault_proof_submitter))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        // Should not be possible to request more, using the same collateral
        assert!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE)))
        .is_err());

        // should not be possible to withdraw the collateral now
        assert!(
            Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(collateral_vault))
                .dispatch(origin_of(account_of(vault)))
                .is_err()
        );
    });
}

/// Like integration_test_issue_polka_btc_execute, but here request only half of the amount - we
/// still transfer the same amount of bitcoin though. Check that it acts as if we requested the
/// full amount
#[test]
fn integration_test_issue_overpayment() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let user = ALICE;
        let vault = BOB;

        let amount_btc = 1000000;
        let overpayment_factor = 2;
        let requested_amount_btc = amount_btc / overpayment_factor;
        let griefing_collateral = 100;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            requested_amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;

        let fee_amount_btc = FeeModule::get_issue_fee(amount_btc).unwrap();
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, None);

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(
            Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(user)))
        );

        // check the sla increase
        let expected_sla_increase = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(amount_btc, total_amount_btc).unwrap();
        assert_eq!(
            SlaModule::vault_sla(account_of(vault)),
            expected_sla_increase
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral reimbursed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc minted
        assert_eq!(final_btc_balance, initial_btc_balance + amount_btc);

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(vault))
        ))
        .dispatch(origin_of(account_of(vault))));
    });
}

#[test]
fn integration_test_issue_refund() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let user = ALICE;
        let vault = BOB;
        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let overpayment_factor = 2;
        let collateral_vault = required_collateral_for_issue(amount_btc);

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let vault_btc_address = issue_request.btc_address;
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) = generate_transaction_and_mine(
            vault_btc_address,
            overpayment_factor * total_amount_btc,
            None,
        );

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        // alice executes the issue by confirming the btc transaction
        assert_ok!(
            Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(user)))
        );

        // check the sla increase
        let expected_sla = SlaModule::vault_executed_issue_max_sla_change()
            * FixedI128::checked_from_rational(amount_btc, total_amount_btc).unwrap();
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral reimbursed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc minted
        assert_eq!(final_btc_balance, initial_btc_balance + amount_btc);

        let refund_address_script =
            bitcoin::Script::try_from("a914d7ff6d60ebf40a9b1886acce06653ba2224d8fea87").unwrap();
        let refund_address = BtcAddress::from_script(&refund_address_script).unwrap();

        let refund_id = assert_refund_request_event();
        let refund = RefundModule::get_open_refund_request_from_id(&refund_id).unwrap();

        // We have overpaid by 100%, and refund_fee = issue_fee, so fees should be equal
        assert_eq!(refund.fee, issue_request.fee);
        assert_eq!(refund.amount_polka_btc, issue_request.amount);

        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(refund_address, refund.amount_polka_btc, Some(refund_id));

        SystemModule::set_block_number((1 + CONFIRMATIONS) * 2);

        assert_ok!(
            Call::Refund(RefundCall::execute_refund(refund_id, tx_id, proof, raw_tx))
                .dispatch(origin_of(account_of(vault)))
        );

        // check that the ExecuteRefund event has been deposited
        let (id, issuer, refunder, amount) = SystemModule::events()
            .iter()
            .find_map(|record| match record.event {
                Event::refund(RefundEvent::ExecuteRefund(a, ref b, ref c, d)) => {
                    Some((a, b.clone(), c.clone(), d))
                }
                _ => None,
            })
            .expect("execute refund event not found");
        assert_eq!(id, refund_id);
        assert_eq!(issuer, account_of(user));
        assert_eq!(refunder, account_of(vault));
        assert_eq!(amount, refund.amount_polka_btc);

        // check the sla increase
        let expected_sla = SlaModule::vault_refunded() + expected_sla;
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);

        // check that fee was minted
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(vault)),
            refund.fee
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_cancel() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 100000;
        let griefing_collateral = 100;
        let collateral_vault = 1000000;

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();

        // expire request without transferring btc
        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(vault))),
            IssueError::CommitPeriodExpired
        );

        // bob cancels issue request
        assert_ok!(
            Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault)))
        );

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral slashed
        assert_eq!(final_dot_balance, initial_dot_balance - griefing_collateral);

        // no polka_btc for alice
        assert_eq!(final_btc_balance, initial_btc_balance);
    });
}

#[test]
fn integration_test_issue_polka_btc_cancel_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 100000;
        let griefing_collateral = 100;
        let collateral_vault = 1000000;

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue = IssueModule::get_issue_request_from_id(&issue_id).unwrap();

        drop_exchange_rate_and_liquidate(vault);

        assert_eq!(
            VaultRegistryModule::get_liquidation_vault().to_be_issued_tokens,
            issue.amount + issue.fee
        );

        // expire request without transferring btc
        SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

        // alice cannot execute past expiry
        assert_noop!(
            Call::Issue(IssueCall::execute_issue(
                issue_id,
                H256Le::from_bytes_le(&[0; 32]),
                vec![],
                vec![]
            ))
            .dispatch(origin_of(account_of(vault))),
            IssueError::CommitPeriodExpired
        );

        // bob cancels issue request
        assert_ok!(
            Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault)))
        );

        assert_eq!(
            VaultRegistryModule::get_liquidation_vault().to_be_issued_tokens,
            0
        );

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        // griefing collateral is NOT slashed
        assert_eq!(final_dot_balance, initial_dot_balance);

        // no polka_btc for alice
        assert_eq!(final_btc_balance, initial_btc_balance);
    });
}

fn request_issue(amount_btc: u128) -> (H256, IssueRequest<AccountId32, u32, u128, u128>) {
    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::one()
    ));

    SystemModule::set_block_number(1);

    assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
        DEFAULT_COLLATERAL,
        dummy_public_key()
    ))
    .dispatch(origin_of(account_of(VAULT))));

    // alice requests polka_btc by locking btc with bob
    assert_ok!(Call::Issue(IssueCall::request_issue(
        amount_btc,
        account_of(VAULT),
        DEFAULT_GRIEFING_COLLATERAL
    ))
    .dispatch(origin_of(account_of(USER))));

    CollateralModule::transfer(
        account_of(VAULT),
        account_of(FAUCET),
        CollateralModule::get_balance_from_account(&account_of(VAULT)),
    )
    .unwrap();

    assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
        DEFAULT_COLLATERAL,
        dummy_public_key()
    ))
    .dispatch(origin_of(account_of(PROOF_SUBMITTER))));

    let issue_id = assert_issue_request_event();
    let issue = IssueModule::get_issue_request_from_id(&issue_id).unwrap();

    (issue_id, issue)
}

#[test]
fn integration_test_issue_polka_btc_execute_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let vault_proof_submitter = CAROL;

        let amount_btc = 1000;

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS,
            },
        );

        let (issue_id, issue) = request_issue(amount_btc);

        let fee_amount_btc = issue.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_issued: total_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);
        execute_issue(issue_id, &issue, total_amount_btc);

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // check that sla is zero for being liquidated
        assert_eq!(SlaModule::vault_sla(account_of(VAULT)), FixedI128::zero());

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        // vault should be empty
        assert_eq!(CoreVaultData::vault(VAULT), CoreVaultData::default());
        // liquidation vault took everything from vault
        assert_eq!(
            CoreVaultData::liquidation_vault(),
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                issued: amount_btc + fee_amount_btc,
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            }
        );
        // net effect is that user received free_tokens
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS + amount_btc,
            },
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT))
        ))
        .dispatch(origin_of(account_of(VAULT))));
        // should not have received fee
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(VAULT)),
            0
        );
    });
}

#[test]
fn integration_test_issue_polka_btc_execute_not_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let vault_proof_submitter = CAROL;

        let amount_btc = 10_000;

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS,
            },
        );

        let (issue_id, issue) = request_issue(amount_btc);

        let fee_amount_btc = issue.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_issued: total_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );

        execute_issue(issue_id, &issue, total_amount_btc);

        // check that the vault who submitted the proof is rewarded with increased SLA score
        assert_eq!(
            SlaModule::vault_sla(account_of(vault_proof_submitter)),
            SlaModule::vault_submitted_issue_proof()
        );

        // fee should be added to epoch rewards
        assert_eq!(FeeModule::epoch_rewards_polka_btc(), fee_amount_btc);

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: amount_btc + fee_amount_btc,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            },
        );
        // net effect is that user received free_tokens
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
                free_tokens: DEFAULT_USER_FREE_TOKENS + amount_btc,
            },
        );

        // force issue rewards and withdraw
        assert_ok!(FeeModule::update_rewards_for_epoch());
        assert_ok!(Call::Fee(FeeCall::withdraw_polka_btc(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT))
        ))
        .dispatch(origin_of(account_of(VAULT))));
        // check that a fee has been withdrawn
        assert!(TreasuryModule::get_balance_from_account(account_of(VAULT)) > 0);
    });
}

fn execute_issue(issue_id: H256, issue: &IssueRequest<AccountId32, u32, u128, u128>, amount: u128) {
    // send the btc from the user to the vault
    let (tx_id, _height, proof, raw_tx) =
        generate_transaction_and_mine(issue.btc_address.clone(), amount, None);

    SystemModule::set_block_number(1 + CONFIRMATIONS);

    // alice executes the issuerequest by confirming the btc transaction
    assert_ok!(
        Call::Issue(IssueCall::execute_issue(issue_id, tx_id, proof, raw_tx))
            .dispatch(origin_of(account_of(PROOF_SUBMITTER)))
    );
}
