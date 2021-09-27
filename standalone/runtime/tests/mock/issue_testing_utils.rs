use crate::{assert_eq, *};
use currency::Amount;
use frame_support::transactional;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const PROOF_SUBMITTER: [u8; 32] = CAROL;

pub const DEFAULT_GRIEFING_COLLATERAL: Amount<Runtime> = griefing(5_000);
pub const DEFAULT_COLLATERAL: u128 = 1_000_000;
pub fn request_issue(
    currency_id: CurrencyId,
    amount_btc: Amount<Runtime>,
) -> (H256, IssueRequest<AccountId32, u32, u128>) {
    RequestIssueBuilder::new(currency_id, amount_btc).request()
}

pub struct RequestIssueBuilder {
    amount_btc: u128,
    vault: [u8; 32],
    user: [u8; 32],
    griefing_collateral: u128,
    currency_id: CurrencyId,
}

impl RequestIssueBuilder {
    pub fn new(currency_id: CurrencyId, amount_btc: Amount<Runtime>) -> Self {
        Self {
            amount_btc: amount_btc.amount(),
            currency_id,
            vault: VAULT,
            user: USER,
            griefing_collateral: DEFAULT_COLLATERAL,
        }
    }

    pub fn with_vault(&mut self, vault: [u8; 32]) -> &mut Self {
        self.vault = vault;
        self
    }

    pub fn with_collateral(&mut self, collateral: Amount<Runtime>) -> &mut Self {
        self.griefing_collateral = collateral.amount();
        self
    }

    pub fn with_user(&mut self, user: [u8; 32]) -> &mut Self {
        self.user = user;
        self
    }

    pub fn request(&self) -> (H256, IssueRequest<AccountId32, u32, u128>) {
        try_register_vault(Amount::new(DEFAULT_COLLATERAL, self.currency_id), self.vault);

        // alice requests wrapped by locking btc with bob
        assert_ok!(Call::Issue(IssueCall::request_issue(
            self.amount_btc,
            account_of(self.vault),
            self.griefing_collateral
        ))
        .dispatch(origin_of(account_of(self.user))));

        let issue_id = assert_issue_request_event();
        let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

        (issue_id, issue)
    }
}

pub struct ExecuteIssueBuilder {
    issue_id: H256,
    issue: IssueRequest<AccountId32, u32, u128>,
    amount: Amount<Runtime>,
    submitter: [u8; 32],
    register_vault_with_currency_id: Option<CurrencyId>,
    relayer: Option<[u8; 32]>,
    execution_tx: Option<(Vec<u8>, Vec<u8>)>,
}

impl ExecuteIssueBuilder {
    pub fn new(issue_id: H256) -> Self {
        let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();
        Self {
            issue_id,
            issue: issue.clone(),
            amount: issue.fee() + issue.amount(),
            submitter: PROOF_SUBMITTER,
            register_vault_with_currency_id: None,
            relayer: None,
            execution_tx: None,
        }
    }

    pub fn with_amount(&mut self, amount: Amount<Runtime>) -> &mut Self {
        self.amount = amount;
        self
    }

    pub fn with_submitter(
        &mut self,
        submitter: [u8; 32],
        register_vault_with_currency_id: Option<CurrencyId>,
    ) -> &mut Self {
        self.submitter = submitter;
        self.register_vault_with_currency_id = register_vault_with_currency_id;
        self
    }

    pub fn with_issue_id(&mut self, id: H256) -> &mut Self {
        self.issue_id = id;
        self
    }

    pub fn with_relayer(&mut self, relayer: Option<[u8; 32]>) -> &mut Self {
        self.relayer = relayer;
        self
    }

    #[transactional]
    pub fn execute(&mut self) -> DispatchResultWithPostInfo {
        self.prepare_for_execution().execute_prepared()
    }

    pub fn execute_prepared(&self) -> DispatchResultWithPostInfo {
        if let Some((proof, raw_tx)) = &self.execution_tx {
            // alice executes the issuerequest by confirming the btc transaction
            Call::Issue(IssueCall::execute_issue(self.issue_id, proof.to_vec(), raw_tx.to_vec()))
                .dispatch(origin_of(account_of(self.submitter)))
        } else {
            panic!("Backing transaction was not prepared prior to execution!");
        }
    }

    pub fn prepare_for_execution(&mut self) -> &mut Self {
        // send the btc from the user to the vault
        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_address(self.issue.btc_address)
            .with_amount(self.amount)
            .with_op_return(None)
            .with_relayer(self.relayer)
            .mine();

        SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

        if let Some(currency_id) = self.register_vault_with_currency_id {
            try_register_vault(Amount::new(DEFAULT_COLLATERAL, currency_id), self.submitter);
        }

        self.execution_tx = Some((proof, raw_tx));
        self
    }

    pub fn assert_execute(&mut self) {
        assert_ok!(self.execute());
    }
}

pub fn execute_issue(issue_id: H256) {
    ExecuteIssueBuilder::new(issue_id).assert_execute()
}

pub fn assert_issue_amount_change_event(
    issue_id: H256,
    amount: Amount<Runtime>,
    fee: Amount<Runtime>,
    confiscated_collateral: Amount<Runtime>,
) {
    let expected_event =
        IssueEvent::IssueAmountChange(issue_id, amount.amount(), fee.amount(), confiscated_collateral.amount());
    let events = SystemModule::events();
    let records: Vec<_> = events
        .iter()
        .rev()
        .filter(|record| matches!(&record.event, Event::Issue(x) if x == &expected_event))
        .collect();
    assert_eq!(records.len(), 1);
}

pub fn assert_issue_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().rev().find(|record| {
        matches!(
            record.event,
            Event::Issue(IssueEvent::RequestIssue(_, _, _, _, _, _, _, _))
        )
    });
    if let Event::Issue(IssueEvent::RequestIssue(id, _, _, _, _, _, _, _)) = record.unwrap().event {
        id
    } else {
        panic!("request issue event not found")
    }
}

pub fn assert_refund_request_event() -> H256 {
    SystemModule::events()
        .iter()
        .find_map(|record| match record.event {
            Event::Refund(RefundEvent::RequestRefund(id, _, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .expect("request refund event not found")
}

pub fn execute_refund(vault_id: [u8; 32]) -> (H256, RefundRequest<AccountId, u128>) {
    let refund_id = assert_refund_request_event();
    let refund = RefundPallet::get_open_refund_request_from_id(&refund_id).unwrap();
    assert_ok!(execute_refund_with_amount(vault_id, wrapped(refund.amount_btc)));
    (refund_id, refund)
}

pub fn execute_refund_with_amount(vault_id: [u8; 32], amount: Amount<Runtime>) -> DispatchResultWithPostInfo {
    let refund_address_script = bitcoin::Script::try_from("a914d7ff6d60ebf40a9b1886acce06653ba2224d8fea87").unwrap();
    let refund_address = BtcAddress::from_script_pub_key(&refund_address_script).unwrap();

    let refund_id = assert_refund_request_event();

    let (_tx_id, _height, proof, raw_tx) = generate_transaction_and_mine(refund_address, amount, Some(refund_id), None);

    SecurityPallet::set_active_block_number((1 + CONFIRMATIONS) * 2);

    Call::Refund(RefundCall::execute_refund(refund_id, proof, raw_tx)).dispatch(origin_of(account_of(vault_id)))
}

pub fn cancel_issue(issue_id: H256, vault: [u8; 32]) {
    // expire request without transferring btc
    SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);

    // cancel issue request
    assert_ok!(Call::Issue(IssueCall::cancel_issue(issue_id)).dispatch(origin_of(account_of(vault))));
}
