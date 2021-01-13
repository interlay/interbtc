mod mock;

use mock::*;
use primitive_types::H256;
use std::convert::TryFrom;

type IssueCall = issue::Call<Runtime>;
type IssueModule = issue::Module<Runtime>;
type IssueEvent = issue::Event<Runtime>;
type IssueError = issue::Error<Runtime>;

type RefundCall = refund::Call<Runtime>;
type RefundModule = refund::Module<Runtime>;
type RefundEvent = refund::Event<Runtime>;
type RefundError = refund::Error<Runtime>;

fn assert_issue_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().find(|record| match record.event {
        Event::issue(IssueEvent::RequestIssue(_, _, _, _, _)) => true,
        _ => false,
    });
    let id = if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _)) = record.unwrap().event {
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
        SecurityModule::set_parachain_status(StatusCode::Shutdown);

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
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = 1005000; // enough for fee + issued amount

        let vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            vault_btc_address.clone()
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
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, issue_id);

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

/// Like integration_test_issue_polka_btc_execute, but here request only half of the amount - we
/// still transfer the same amount of bitcoin though. Check that it acts as if we requested the
/// full amount
#[test]
fn integration_test_issue_overpayment() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 1000000;
        let overpayment_factor = 2;
        let requested_amount_btc = amount_btc / overpayment_factor;
        let griefing_collateral = 100;
        let collateral_vault = 1005000;

        let vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            vault_btc_address.clone()
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

        let fee_amount_btc = FeeModule::get_issue_fee(amount_btc).unwrap();
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) =
            generate_transaction_and_mine(vault_btc_address, total_amount_btc, issue_id);

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
        let user = ALICE;
        let vault = BOB;

        let amount_btc = 1000000;
        let griefing_collateral = 100;
        let collateral_vault = 1005000;
        let overpayment_factor = 2;

        let vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            vault_btc_address.clone()
        ))
        .dispatch(origin_of(account_of(vault))));

        let issuable =
            VaultRegistryModule::get_issuable_tokens_from_vault(account_of(vault)).unwrap();
        println!("{}", issuable);

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            amount_btc,
            account_of(vault),
            griefing_collateral
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let issue_id = assert_issue_request_event();
        let issue_request = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        let fee_amount_btc = issue_request.fee;
        let total_amount_btc = amount_btc + fee_amount_btc;

        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) = generate_transaction_and_mine(
            vault_btc_address,
            overpayment_factor * total_amount_btc,
            issue_id,
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
            generate_transaction_and_mine(refund_address, refund.amount_polka_btc, refund_id);

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

        let vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));

        SystemModule::set_block_number(1);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            vault_btc_address.clone()
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
