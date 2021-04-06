use crate::*;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const PROOF_SUBMITTER: [u8; 32] = CAROL;

pub const DEFAULT_GRIEFING_COLLATERAL: u128 = 5_000;
pub const DEFAULT_COLLATERAL: u128 = 1_000_000;

pub fn request_issue(amount_btc: u128) -> (H256, IssueRequest<AccountId32, u32, u128, u128>) {
    RequestIssueBuilder::new(amount_btc).request()
}

pub struct RequestIssueBuilder {
    amount_btc: u128,
    vault: [u8; 32],
    user: [u8; 32],
}

impl RequestIssueBuilder {
    pub fn new(amount_btc: u128) -> Self {
        Self {
            amount_btc,
            vault: VAULT,
            user: USER,
        }
    }

    pub fn with_vault(&mut self, vault: [u8; 32]) -> &mut Self {
        self.vault = vault;
        self
    }

    pub fn with_user(&mut self, user: [u8; 32]) -> &mut Self {
        self.user = user;
        self
    }

    pub fn request(&self) -> (H256, IssueRequest<AccountId32, u32, u128, u128>) {
        try_register_vault(DEFAULT_COLLATERAL, self.vault);

        // alice requests polka_btc by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            self.amount_btc,
            account_of(self.vault),
            DEFAULT_GRIEFING_COLLATERAL
        ))
        .dispatch(origin_of(account_of(self.user))));

        let issue_id = assert_issue_request_event();
        let issue = IssueModule::get_issue_request_from_id(&issue_id).unwrap();

        (issue_id, issue)
    }
}

pub struct ExecuteIssueBuilder {
    issue_id: H256,
    issue: IssueRequest<AccountId32, u32, u128, u128>,
    amount: u128,
    submitter: [u8; 32],
    register_submitter_as_vault: bool,
    relayer: Option<[u8; 32]>,
}

impl ExecuteIssueBuilder {
    pub fn new(issue_id: H256) -> Self {
        let issue = IssueModule::get_issue_request_from_id(&issue_id).unwrap();
        Self {
            issue_id,
            issue: issue.clone(),
            amount: issue.fee + issue.amount,
            submitter: PROOF_SUBMITTER,
            register_submitter_as_vault: true,
            relayer: None,
        }
    }

    pub fn with_amount(&mut self, amount: u128) -> &mut Self {
        self.amount = amount;
        self
    }

    pub fn with_submitter(&mut self, submitter: [u8; 32], register_as_vault: bool) -> &mut Self {
        self.submitter = submitter;
        self.register_submitter_as_vault = register_as_vault;
        self
    }

    pub fn with_relayer(&mut self, relayer: Option<[u8; 32]>) -> &mut Self {
        self.relayer = relayer;
        self
    }

    pub fn execute(&self) -> DispatchResultWithPostInfo {
        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) = TransactionGenerator::new()
            .with_address(self.issue.btc_address.clone())
            .with_amount(self.amount)
            .with_op_return(None)
            .with_relayer(self.relayer)
            .mine();

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        if self.register_submitter_as_vault {
            try_register_vault(DEFAULT_COLLATERAL, self.submitter);
        }

        // alice executes the issuerequest by confirming the btc transaction
        Call::Issue(IssueCall::execute_issue(self.issue_id, tx_id, proof, raw_tx))
            .dispatch(origin_of(account_of(self.submitter)))
    }
    pub fn assert_execute(&self) {
        assert_ok!(self.execute());
    }
}

pub fn execute_issue(issue_id: H256) {
    ExecuteIssueBuilder::new(issue_id).assert_execute()
}

pub fn assert_issue_amount_change_event(issue_id: H256, amount: u128, fee: u128, confiscated_collateral: u128) {
    let expected_event = IssueEvent::IssueAmountChange(issue_id, amount, fee, confiscated_collateral);
    let events = SystemModule::events();
    let records: Vec<_> = events
        .iter()
        .rev()
        .filter(|record| matches!(&record.event, Event::issue(x) if x == &expected_event))
        .collect();
    assert_eq!(records.len(), 1);
}

pub fn assert_issue_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().rev().find(|record| match record.event {
        Event::issue(IssueEvent::RequestIssue(_, _, _, _, _, _, _, _)) => true,
        _ => false,
    });
    let id = if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _, _, _, _)) = record.unwrap().event {
        id
    } else {
        panic!("request issue event not found")
    };
    id
}

pub fn assert_refund_request_event() -> H256 {
    SystemModule::events()
        .iter()
        .find_map(|record| match record.event {
            Event::refund(RefundEvent::RequestRefund(id, _, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .expect("request refund event not found")
}

pub fn execute_refund(vault_id: [u8; 32]) -> (H256, RefundRequest<AccountId, u128>) {
    let refund_address_script = bitcoin::Script::try_from("a914d7ff6d60ebf40a9b1886acce06653ba2224d8fea87").unwrap();
    let refund_address = BtcAddress::from_script(&refund_address_script).unwrap();

    let refund_id = assert_refund_request_event();
    let refund = RefundModule::get_open_refund_request_from_id(&refund_id).unwrap();

    let (tx_id, _height, proof, raw_tx) =
        generate_transaction_and_mine(refund_address, refund.amount_polka_btc, Some(refund_id));

    SystemModule::set_block_number((1 + CONFIRMATIONS) * 2);

    assert_ok!(
        Call::Refund(RefundCall::execute_refund(refund_id, tx_id, proof, raw_tx))
            .dispatch(origin_of(account_of(vault_id)))
    );

    (refund_id, refund)
}

pub fn cancel_issue(issue_id: H256, vault: [u8; 32]) {
    // expire request without transferring btc
    SystemModule::set_block_number(IssueModule::issue_period() + 1 + 1);

    // cancel issue request
    assert_ok!(Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault))));
}
