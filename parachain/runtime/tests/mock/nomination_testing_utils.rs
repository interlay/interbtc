use crate::*;

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

pub const DEFAULT_GRIEFING_COLLATERAL: u128 = 5_000;
pub const DEFAULT_BACKING_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_NOMINATION: u128 = 20_000;

pub const DEFAULT_OPERATOR_UNBONDING_PERIOD: u32 = 100;
pub const DEFAULT_NOMINATOR_UNBONDING_PERIOD: u32 = 50;

pub fn enable_nomination() {
    assert_ok!(Call::Nomination(NominationCall::set_nomination_enabled(true))
        .dispatch(<Runtime as frame_system::Config>::Origin::root()));
}

pub fn disable_nomination() {
    assert_ok!(Call::Nomination(NominationCall::set_nomination_enabled(false))
        .dispatch(<Runtime as frame_system::Config>::Origin::root()));
}

pub fn register_vault(vault: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::VaultRegistry(VaultRegistryCall::register_vault(
        DEFAULT_BACKING_COLLATERAL,
        dummy_public_key(),
    ))
    .dispatch(origin_of(account_of(vault)))
}

pub fn assert_register_vault(vault: [u8; 32]) {
    assert_ok!(register_vault(vault));
}

pub fn register_operator(vault: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_in_to_nomination()).dispatch(origin_of(account_of(vault)))
}

pub fn assert_register_operator(vault: [u8; 32]) {
    assert_ok!(register_operator(vault));
}

pub fn deregister_operator(vault: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_out_of_nomination()).dispatch(origin_of(account_of(vault)))
}

pub fn nominate_collateral(
    nominator: [u8; 32],
    operator: [u8; 32],
    amount_collateral: u128,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::deposit_nominated_collateral(
        account_of(operator),
        amount_collateral,
    ))
    .dispatch(origin_of(account_of(nominator)))
}

pub fn assert_nominate_collateral(nominator: [u8; 32], operator: [u8; 32], amount_collateral: u128) {
    assert_ok!(nominate_collateral(nominator, operator, amount_collateral));
}

pub fn request_operator_collateral_withdrawal(
    operator: [u8; 32],
    amount_collateral: u128,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::request_collateral_withdrawal(
        account_of(operator),
        amount_collateral,
    ))
    .dispatch(origin_of(account_of(operator)))
}

pub fn execute_operator_collateral_withdrawal(operator: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::execute_collateral_withdrawal(account_of(operator)))
        .dispatch(origin_of(account_of(operator)))
}

pub fn assert_request_operator_collateral_withdrawal(operator: [u8; 32], amount_dot: u128) {
    assert_ok!(request_operator_collateral_withdrawal(operator, amount_dot));
}

pub fn cancel_operator_collateral_withdrawal(operator: [u8; 32], request_id: H256) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::cancel_collateral_withdrawal(
        account_of(operator),
        request_id,
    ))
    .dispatch(origin_of(account_of(operator)))
}

pub fn assert_operator_withdrawal_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().rev().find(|record| {
        matches!(
            record.event,
            Event::nomination(NominationEvent::RequestOperatorCollateralWithdrawal(_, _, _, _))
        )
    });
    if let Event::nomination(NominationEvent::RequestOperatorCollateralWithdrawal(id, _, _, _)) = record.unwrap().event
    {
        id
    } else {
        panic!("request issue event not found")
    }
}

pub fn request_nominator_collateral_withdrawal(
    nominator: [u8; 32],
    operator: [u8; 32],
    amount_collateral: u128,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::request_collateral_withdrawal(
        account_of(operator),
        amount_collateral,
    ))
    .dispatch(origin_of(account_of(nominator)))
}

pub fn execute_nominator_collateral_withdrawal(nominator: [u8; 32], operator: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::execute_collateral_withdrawal(account_of(operator)))
        .dispatch(origin_of(account_of(nominator)))
}

pub fn assert_request_nominator_collateral_withdrawal(nominator: [u8; 32], operator: [u8; 32], amount_dot: u128) {
    assert_ok!(request_nominator_collateral_withdrawal(nominator, operator, amount_dot));
}

pub fn cancel_nominator_collateral_withdrawal(
    nominator: [u8; 32],
    operator: [u8; 32],
    request_id: H256,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::cancel_collateral_withdrawal(
        account_of(operator),
        request_id,
    ))
    .dispatch(origin_of(account_of(nominator)))
}

pub fn assert_nominator_withdrawal_request_event() -> H256 {
    let events = SystemModule::events();
    let record = events.iter().rev().find(|record| {
        matches!(
            record.event,
            Event::nomination(NominationEvent::RequestNominatorCollateralWithdrawal(_, _, _, _, _))
        )
    });
    if let Event::nomination(NominationEvent::RequestNominatorCollateralWithdrawal(id, _, _, _, _)) =
        record.unwrap().event
    {
        id
    } else {
        panic!("request issue event not found")
    }
}

pub fn assert_total_nominated_collateral_is(operator: [u8; 32], amount_collateral: u128) {
    let nominated_collateral = NominationPallet::get_total_nominated_collateral(&account_of(operator)).unwrap();
    assert_eq!(nominated_collateral, amount_collateral);
}

pub fn get_nominator_collateral(nominator: [u8; 32], operator: [u8; 32]) -> u128 {
    NominationPallet::get_nominator_collateral(&account_of(nominator), &account_of(operator)).unwrap()
}
