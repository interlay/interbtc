use crate::{assert_eq, *};
use currency::Amount;
use frame_support::transactional;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const PROOF_SUBMITTER: [u8; 32] = CAROL;

pub const DEFAULT_COLLATERAL: Balance = 1_000_000;

pub fn request_issue(
    vault_id: &VaultId,
    amount_btc: Amount<Runtime>,
) -> (H256, IssueRequest<AccountId32, BlockNumber, Balance, CurrencyId>) {
    RequestIssueBuilder::new(vault_id, amount_btc).request()
}

pub struct RequestIssueBuilder {
    amount_btc: Balance,
    vault_id: VaultId,
    user: [u8; 32],
}

impl RequestIssueBuilder {
    pub fn new(vault_id: &VaultId, amount_btc: Amount<Runtime>) -> Self {
        Self {
            amount_btc: amount_btc.amount(),
            vault_id: vault_id.clone(),
            user: USER,
        }
    }

    pub fn with_vault(&mut self, vault: VaultId) -> &mut Self {
        self.vault_id = vault;
        self
    }

    pub fn with_user(&mut self, user: [u8; 32]) -> &mut Self {
        self.user = user;
        self
    }

    pub fn request(&self) -> (H256, IssueRequest<AccountId32, BlockNumber, Balance, CurrencyId>) {
        try_register_vault(
            Amount::new(DEFAULT_COLLATERAL, self.vault_id.collateral_currency()),
            &self.vault_id,
        );
        // alice requests wrapped by locking btc with bob
        assert_ok!(RuntimeCall::Issue(IssueCall::request_issue {
            amount: self.amount_btc,
            vault_id: self.vault_id.clone(),
        })
        .dispatch(origin_of(account_of(self.user))));

        let issue_id = assert_issue_request_event();
        let issue = IssuePallet::get_issue_request_from_id(&issue_id).unwrap();

        (issue_id, issue)
    }
}

pub struct ExecuteIssueBuilder {
    issue_id: H256,
    issue: IssueRequest<AccountId32, BlockNumber, Balance, CurrencyId>,
    amount: Amount<Runtime>,
    submitter: AccountId,
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
            submitter: account_of(PROOF_SUBMITTER),
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
        submitter: AccountId,
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
        VaultRegistryPallet::collateral_integrity_check();

        if let Some((proof, raw_tx)) = &self.execution_tx {
            // alice executes the issuerequest by confirming the btc transaction
            let ret = RuntimeCall::Issue(IssueCall::execute_issue {
                issue_id: self.issue_id,
                merkle_proof: proof.to_vec(),
                raw_tx: raw_tx.to_vec(),
            })
            .dispatch(origin_of(self.submitter.clone()));
            VaultRegistryPallet::collateral_integrity_check();
            ret
        } else {
            panic!("Backing transaction was not prepared prior to execution!");
        }
    }

    pub fn prepare_for_execution(&mut self) -> &mut Self {
        // send the btc from the user to the vault
        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_outputs(vec![(self.issue.btc_address, self.amount)])
            .with_relayer(self.relayer)
            .mine();

        SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

        if let Some(currency_id) = self.register_vault_with_currency_id {
            try_register_vault(
                Amount::new(DEFAULT_COLLATERAL, currency_id),
                &PrimitiveVaultId::new(self.submitter.clone(), currency_id, DEFAULT_WRAPPED_CURRENCY),
            );
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
    let expected_event = IssueEvent::IssueAmountChange {
        issue_id,
        amount: amount.amount(),
        fee: fee.amount(),
        confiscated_griefing_collateral: confiscated_collateral.amount(),
    };
    let events = SystemPallet::events();
    let records: Vec<_> = events
        .iter()
        .rev()
        .filter(|record| matches!(&record.event, RuntimeEvent::Issue(x) if x == &expected_event))
        .collect();
    assert_eq!(records.len(), 1);
}

pub fn assert_issue_request_event() -> H256 {
    let events = SystemPallet::events();
    let record = events
        .iter()
        .rev()
        .find(|record| matches!(record.event, RuntimeEvent::Issue(IssueEvent::RequestIssue { .. })));
    if let RuntimeEvent::Issue(IssueEvent::RequestIssue { issue_id, .. }) = record.unwrap().event {
        issue_id
    } else {
        panic!("request issue event not found")
    }
}

pub fn cancel_issue(issue_id: H256, vault: [u8; 32]) {
    // expire request without transferring btc
    SecurityPallet::set_active_block_number(IssuePallet::issue_period() + 1 + 1);

    // cancel issue request
    assert_ok!(
        RuntimeCall::Issue(IssueCall::cancel_issue { issue_id: issue_id }).dispatch(origin_of(account_of(vault)))
    );
}
