use crate::{assert_eq, *};
use currency::Amount;
use frame_support::transactional;
use redeem::RedeemRequestStatus;
use vault_registry::DefaultVaultId;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const USER_BTC_ADDRESS: BtcAddress = BtcAddress::P2PKH(H160([2u8; 20]));

pub struct ExecuteRedeemBuilder {
    redeem_id: H256,
    redeem: RedeemRequest<AccountId32, u32, u128, CurrencyId>,
    amount: Amount<Runtime>,
    submitter: AccountId32,
    inclusion_fee: Amount<Runtime>,
}

impl ExecuteRedeemBuilder {
    pub fn new(redeem_id: H256) -> Self {
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        Self {
            redeem_id,
            redeem: redeem.clone(),
            amount: redeem.amount_btc(),
            submitter: redeem.redeemer,
            inclusion_fee: wrapped(0),
        }
    }

    pub fn with_amount(&mut self, amount: Amount<Runtime>) -> &mut Self {
        self.amount = amount;
        self
    }

    pub fn with_submitter(&mut self, submitter: [u8; 32]) -> &mut Self {
        self.submitter = account_of(submitter);
        self
    }

    pub fn with_inclusion_fee(&mut self, inclusion_fee: Amount<Runtime>) -> &mut Self {
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

    pub fn assert_noop(&self, error: RedeemError) {
        assert_noop!(self.execute(), error);
    }
}

pub fn setup_cancelable_redeem(user: [u8; 32], vault: DefaultVaultId<Runtime>, issued_tokens: Amount<Runtime>) -> H256 {
    let redeem_id = setup_redeem(issued_tokens, user, vault);

    // expire request without transferring btc
    mine_blocks((RedeemPallet::redeem_period() + 99) / 100 + 1);
    SecurityPallet::set_active_block_number(RedeemPallet::redeem_period() + 1 + 1);

    redeem_id
}

pub fn set_redeem_state(
    currency_id: CurrencyId,
    vault_to_be_redeemed: Amount<Runtime>,
    user_to_redeem: Amount<Runtime>,
    user: [u8; 32],
    vault: [u8; 32],
) -> () {
    let burned_tokens = user_to_redeem - FeePallet::get_redeem_fee(&user_to_redeem).unwrap();
    let vault_issued_tokens = vault_to_be_redeemed + burned_tokens;
    CoreVaultData::force_to(
        vault,
        CoreVaultData {
            issued: vault_issued_tokens,
            to_be_redeemed: vault_to_be_redeemed,
            ..CoreVaultData::get_default(currency_id)
        },
    );
    let mut user_state = UserData::get(user);
    (*user_state.balances.get_mut(&INTERBTC).unwrap()).free = user_to_redeem;

    UserData::force_to(ALICE, user_state);
}

pub fn setup_redeem(issued_tokens: Amount<Runtime>, user: [u8; 32], vault: DefaultVaultId<Runtime>) -> H256 {
    // alice requests to redeem issued_tokens from Bob
    assert_ok!(Call::Redeem(RedeemCall::request_redeem(
        issued_tokens.amount(),
        USER_BTC_ADDRESS,
        vault
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
            Event::Redeem(RedeemEvent::RequestRedeem(id, _, _, _, _, _, _, _)) => Some(id),
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

pub fn assert_redeem_error(
    redeem_id: H256,
    user_btc_address: BtcAddress,
    amount: Amount<Runtime>,
    return_data: H256,
    current_block_number: u32,
    error: BTCRelayError,
) -> u32 {
    // send the btc from the vault to the user
    let (_tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(user_btc_address, amount, Some(return_data));

    SecurityPallet::set_active_block_number(current_block_number + 1 + CONFIRMATIONS);

    assert_noop!(
        Call::Redeem(RedeemCall::execute_redeem(redeem_id, merkle_proof.clone(), raw_tx))
            .dispatch(origin_of(account_of(VAULT))),
        error
    );
    return current_block_number + 1 + CONFIRMATIONS;
}

pub fn check_redeem_status(user: [u8; 32], status: RedeemRequestStatus) {
    let redeems = RedeemPallet::get_redeem_requests_for_account(account_of(user));
    assert_eq!(redeems.len(), 1);
    let (_, redeem) = redeems[0].clone();
    assert_eq!(redeem.status, status);
}
