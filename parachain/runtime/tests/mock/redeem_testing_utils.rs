use crate::*;
use frame_support::transactional;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const USER_BTC_ADDRESS: BtcAddress = BtcAddress::P2PKH(H160([2u8; 20]));

pub struct ExecuteRedeemBuilder {
    redeem_id: H256,
    redeem: RedeemRequest<AccountId32, u32, u128, u128>,
    amount: u128,
    submitter: AccountId32,
}

impl ExecuteRedeemBuilder {
    pub fn new(redeem_id: H256) -> Self {
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
        Self {
            redeem_id,
            redeem: redeem.clone(),
            amount: redeem.fee + redeem.amount_btc,
            submitter: redeem.redeemer,
        }
    }

    pub fn with_amount(&mut self, amount: u128) -> &mut Self {
        self.amount = amount;
        self
    }

    pub fn with_submitter(&mut self, submitter: [u8; 32]) -> &mut Self {
        self.submitter = account_of(submitter);
        self
    }

    #[transactional]
    pub fn execute(&self) -> DispatchResultWithPostInfo {
        // send the btc from the user to the vault
        let (tx_id, _height, proof, raw_tx) = TransactionGenerator::new()
            .with_address(self.redeem.btc_address.clone())
            .with_amount(self.amount)
            .with_op_return(Some(self.redeem_id))
            .mine();

        SecurityModule::set_active_block_number(SecurityModule::active_block_number() + CONFIRMATIONS);

        // alice executes the redeemrequest by confirming the btc transaction
        Call::Redeem(RedeemCall::execute_redeem(self.redeem_id, tx_id, proof, raw_tx))
            .dispatch(origin_of(self.submitter.clone()))
    }

    pub fn assert_execute(&self) {
        assert_ok!(self.execute());
    }
}

pub fn setup_cancelable_redeem(user: [u8; 32], vault: [u8; 32], collateral: u128, polka_btc: u128) -> H256 {
    let redeem_id = setup_redeem(polka_btc, user, vault, collateral);

    // expire request without transferring btc
    SecurityModule::set_active_block_number(RedeemModule::redeem_period() + 1 + 1);

    // bob cannot execute past expiry
    assert_noop!(
        Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            H256Le::from_bytes_le(&[0; 32]),
            vec![],
            vec![],
        ))
        .dispatch(origin_of(account_of(vault))),
        RedeemError::CommitPeriodExpired,
    );

    redeem_id
}

pub fn setup_redeem(polka_btc: u128, user: [u8; 32], vault: [u8; 32], _collateral: u128) -> H256 {
    // alice requests to redeem polka_btc from Bob
    assert_ok!(Call::Redeem(RedeemCall::request_redeem(
        polka_btc,
        USER_BTC_ADDRESS,
        account_of(vault)
    ))
    .dispatch(origin_of(account_of(user))));

    // assert that request happened and extract the id
    assert_redeem_request_event()
}

// asserts redeem event happen and extracts its id for further testing
pub fn assert_redeem_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

pub fn execute_redeem(redeem_id: H256) {
    ExecuteRedeemBuilder::new(redeem_id).assert_execute();
}

pub fn cancel_redeem(redeem_id: H256, redeemer: [u8; 32], reimburse: bool) {
    assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, reimburse)).dispatch(origin_of(account_of(redeemer))));
}
