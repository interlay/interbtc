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
    inclusion_fee: u128,
}

impl ExecuteRedeemBuilder {
    pub fn new(redeem_id: H256) -> Self {
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        Self {
            redeem_id,
            redeem: redeem.clone(),
            amount: redeem.amount_btc,
            submitter: redeem.redeemer,
            inclusion_fee: 0,
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

    pub fn with_inclusion_fee(&mut self, inclusion_fee: u128) -> &mut Self {
        self.inclusion_fee = inclusion_fee;
        self
    }

    #[transactional]
    pub fn execute(&self) -> DispatchResultWithPostInfo {
        // send the btc from the user to the vault
        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_address(self.redeem.btc_address)
            .with_amount(self.amount)
            .with_op_return(Some(self.redeem_id))
            .mine();

        SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

        // alice executes the redeemrequest by confirming the btc transaction
        Call::Redeem(RedeemCall::execute_redeem(self.redeem_id, proof, raw_tx))
            .dispatch(origin_of(self.submitter.clone()))
    }

    pub fn assert_execute(&self) {
        assert_ok!(self.execute());
    }
}

pub fn setup_cancelable_redeem(user: [u8; 32], vault: [u8; 32], collateral: u128, polka_btc: u128) -> H256 {
    let redeem_id = setup_redeem(polka_btc, user, vault, collateral);

    // expire request without transferring btc
    SecurityPallet::set_active_block_number(RedeemPallet::redeem_period() + 1 + 1);

    // bob cannot execute past expiry
    assert_noop!(
        Call::Redeem(RedeemCall::execute_redeem(redeem_id, vec![], vec![],)).dispatch(origin_of(account_of(vault))),
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
            Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0]
}

pub fn execute_redeem(redeem_id: H256) {
    ExecuteRedeemBuilder::new(redeem_id).assert_execute();
}

pub fn cancel_redeem(redeem_id: H256, redeemer: [u8; 32], reimburse: bool) {
    assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, reimburse)).dispatch(origin_of(account_of(redeemer))));
}
