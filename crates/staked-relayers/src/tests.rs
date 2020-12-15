extern crate hex;
use crate::types::{
    ActiveStakedRelayer, InactiveStakedRelayer, ProposalStatus, StakedRelayerStatus, StatusUpdate,
    Tally,
};
use crate::{ext, mock::*};
use bitcoin::formatter::Formattable;
use bitcoin::types::{H256Le, TransactionBuilder, TransactionInputBuilder, TransactionOutput};
use btc_relay::BtcAddress;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use redeem::types::RedeemRequest;
use replace::types::ReplaceRequest;
use security::types::{ErrorCode, StatusCode};
use sp_core::{H160, H256};
use sp_std::collections::btree_set::BTreeSet;
use std::convert::TryInto;
use std::str::FromStr;
use vault_registry::{Vault, VaultStatus, Wallet};

type Event = crate::Event<Test>;

const DEFAULT_START_HEIGHT: u64 = 0;
const DEFAULT_END_HEIGHT: u64 = 100;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::test_events($event);
        assert_eq!(
            System::events()
                .iter()
                .filter(|a| a.event == test_event)
                .count(),
            $times
        );
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

/// Mocking functions
fn init_zero_vault<Test>(
    id: AccountId,
    btc_address: Option<BtcAddress>,
) -> Vault<AccountId, BlockNumber, u64> {
    let mut vault = Vault::default();
    vault.id = id;
    match btc_address {
        Some(btc_address) => vault.wallet = Wallet::new(btc_address),
        None => {}
    }
    vault
}

/// Tests
#[test]
fn test_register_staked_relayer_fails_with_insufficient_stake() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 0;

        assert_err!(
            Staking::register_staked_relayer(relayer, amount),
            TestError::InsufficientStake,
        );
    })
}

#[test]
fn test_register_staked_relayer_succeeds() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 20;

        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Staking::register_staked_relayer(relayer.clone(), amount));
        assert_emitted!(Event::RegisterStakedRelayer(ALICE, 11, amount));

        // re-registration not allowed
        assert_err!(
            Staking::register_staked_relayer(relayer, amount),
            TestError::AlreadyRegistered,
        );

        assert_err!(
            Staking::get_active_staked_relayer(&ALICE),
            TestError::NotRegistered,
        );

        assert_ok!(
            Staking::get_inactive_staked_relayer(&ALICE),
            InactiveStakedRelayer {
                stake: amount,
                status: StakedRelayerStatus::Bonding(11)
            }
        );

        assert_err!(
            Staking::try_bond_staked_relayer(&ALICE, amount, 0, 11),
            TestError::NotMatured
        );
        assert_err!(
            Staking::get_active_staked_relayer(&ALICE),
            TestError::NotRegistered,
        );

        assert_ok!(Staking::try_bond_staked_relayer(&ALICE, amount, 20, 10));
        assert_ok!(
            Staking::get_active_staked_relayer(&ALICE),
            ActiveStakedRelayer { stake: amount }
        );
    })
}

#[test]
fn test_deregister_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);

        assert_err!(
            Staking::deregister_staked_relayer(relayer),
            TestError::NotRegistered,
        );
    })
}

fn inject_active_staked_relayer(id: &AccountId, amount: Balance) {
    Staking::add_active_staked_relayer(id, amount);
    assert_ok!(
        Staking::get_active_staked_relayer(id),
        ActiveStakedRelayer { stake: amount }
    );
}

fn inject_status_update(proposer: AccountId) -> u64 {
    let mut tally = Tally::default();
    tally.aye.insert(proposer.clone());

    Staking::insert_active_status_update(StatusUpdate {
        new_status_code: StatusCode::Error,
        old_status_code: StatusCode::Running,
        add_error: None,
        remove_error: None,
        start: DEFAULT_START_HEIGHT,
        end: DEFAULT_END_HEIGHT,
        proposal_status: ProposalStatus::Pending,
        btc_block_hash: None,
        proposer: proposer,
        deposit: 10,
        tally: tally,
        message: vec![],
    })
}

#[test]
fn test_deregister_staked_relayer_fails_with_status_update_found() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        let _ = inject_status_update(ALICE);

        assert_err!(
            Staking::deregister_staked_relayer(relayer),
            TestError::StatusUpdateFound,
        );
    })
}

#[test]
fn test_deregister_staked_relayer_succeeds() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);

        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Staking::deregister_staked_relayer(relayer));
        assert_emitted!(Event::DeregisterStakedRelayer(ALICE));
    })
}

#[test]
fn test_activate_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        assert_err!(
            Staking::activate_staked_relayer(Origin::signed(ALICE)),
            TestError::NotRegistered
        );
    })
}

fn inject_inactive_staked_relayer(id: &AccountId, amount: Balance) {
    Staking::add_inactive_staked_relayer(id, amount, StakedRelayerStatus::Idle);
    assert_ok!(
        Staking::get_inactive_staked_relayer(id),
        InactiveStakedRelayer {
            stake: amount,
            status: StakedRelayerStatus::Idle
        }
    );
}

#[test]
fn test_activate_staked_relayer_succeeds() {
    run_test(|| {
        inject_inactive_staked_relayer(&ALICE, 3);
        assert_ok!(Staking::activate_staked_relayer(Origin::signed(ALICE)));
        assert_ok!(
            Staking::get_active_staked_relayer(&ALICE),
            ActiveStakedRelayer { stake: 3 }
        );
    })
}

#[test]
fn test_deactivate_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        assert_err!(
            Staking::deactivate_staked_relayer(Origin::signed(ALICE)),
            TestError::NotRegistered
        );
    })
}

#[test]
fn test_deactivate_staked_relayer_succeeds() {
    run_test(|| {
        inject_active_staked_relayer(&ALICE, 3);
        assert_ok!(Staking::deactivate_staked_relayer(Origin::signed(ALICE)));
        assert_ok!(
            Staking::get_inactive_staked_relayer(&ALICE),
            InactiveStakedRelayer {
                stake: 3,
                status: StakedRelayerStatus::Idle
            }
        );
    })
}

#[test]
fn test_suggest_status_update_fails_with_staked_relayers_only() {
    run_test(|| {
        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                20,
                StatusCode::Error,
                None,
                None,
                None,
                vec![],
            ),
            TestError::StakedRelayersOnly,
        );
    })
}

#[test]
fn test_suggest_status_update_fails_with_governance_only() {
    run_test(|| {
        Staking::only_governance
            .mock_safe(|_| MockResult::Return(Err(TestError::GovernanceOnly.into())));

        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                20,
                StatusCode::Shutdown,
                None,
                None,
                None,
                vec![],
            ),
            TestError::GovernanceOnly,
        );
    })
}

#[test]
fn test_suggest_status_update_fails_with_insufficient_deposit() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        inject_active_staked_relayer(&ALICE, 20);

        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                0,
                StatusCode::Error,
                None,
                None,
                None,
                vec![],
            ),
            TestError::InsufficientDeposit,
        );
    })
}

#[test]
fn test_suggest_status_update_fails_with_message_too_big() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        inject_active_staked_relayer(&ALICE, 20);

        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                20,
                StatusCode::Error,
                None,
                None,
                None,
                Vec::from([0; 64]),
            ),
            TestError::MessageTooBig,
        );
    })
}

#[test]
fn test_suggest_status_update_fails_with_no_block_hash_found() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        ext::btc_relay::block_header_exists::<Test>.mock_safe(move |_| MockResult::Return(false));
        inject_active_staked_relayer(&ALICE, 20);
        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                20,
                StatusCode::Error,
                Some(ErrorCode::InvalidBTCRelay),
                None,
                Some(H256Le::zero()),
                vec![],
            ),
            TestError::BlockNotFound,
        );
    })
}

#[test]
fn test_suggest_suggest_invalid_block_already_reported() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        ext::btc_relay::block_header_exists::<Test>.mock_safe(move |_| MockResult::Return(true));
        inject_active_staked_relayer(&ALICE, 20);

        let mut status_update = StatusUpdate::default();
        status_update.add_error = Some(ErrorCode::InvalidBTCRelay);
        status_update.btc_block_hash = Some(H256Le::zero());
        Staking::insert_active_status_update(status_update);

        assert_err!(
            Staking::suggest_status_update(
                Origin::signed(ALICE),
                20,
                StatusCode::Error,
                Some(ErrorCode::InvalidBTCRelay),
                None,
                Some(H256Le::zero()),
                vec![],
            ),
            TestError::BlockAlreadyReported,
        );
    })
}

#[test]
fn test_suggest_status_update_succeeds() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        inject_active_staked_relayer(&ALICE, 20);

        assert_ok!(Staking::suggest_status_update(
            Origin::signed(ALICE),
            20,
            StatusCode::Error,
            None,
            None,
            None,
            vec![],
        ));
        assert_emitted!(Event::StatusUpdateSuggested(
            1,
            ALICE,
            StatusCode::Error,
            None,
            None,
            None
        ));
    })
}

macro_rules! account_id_set {
    () => { BTreeSet::<AccountId>::new() };
    ($($x:expr),*) => {
        {
            let mut set = BTreeSet::<AccountId>::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}

#[test]
fn test_tally_is_approved_or_rejected() {
    run_test(|| {
        let mut tally = Tally {
            aye: account_id_set!(1, 2, 3),
            nay: account_id_set!(4, 5, 6),
        };

        assert_eq!(tally.is_approved(6, 25), true);
        assert_eq!(tally.is_rejected(6, 25), false);

        assert_eq!(tally.is_approved(6, 50), false);
        assert_eq!(tally.is_rejected(6, 50), false);

        assert_eq!(tally.is_approved(6, 75), false);
        assert_eq!(tally.is_rejected(6, 75), true);

        assert_eq!(tally.is_approved(6, 100), false);
        assert_eq!(tally.is_rejected(6, 100), true);

        tally.nay.clear();
        assert_eq!(tally.is_approved(3, 100), true);
    })
}

#[test]
fn test_tally_vote() {
    run_test(|| {
        let mut tally = Tally {
            aye: account_id_set!(),
            nay: account_id_set!(),
        };

        tally.vote(1, true);
        tally.vote(2, false);

        assert_eq!(
            tally,
            Tally {
                aye: account_id_set!(1),
                nay: account_id_set!(2),
            }
        );
    })
}

#[test]
fn test_vote_on_status_update_fails_with_staked_relayers_only() {
    run_test(|| {
        assert_err!(
            Staking::vote_on_status_update(Origin::signed(ALICE), 0, false),
            TestError::StakedRelayersOnly,
        );
    })
}

#[test]
fn test_vote_on_status_update_succeeds() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        let status_update_id = inject_status_update(ALICE);
        assert_err!(
            Staking::vote_on_status_update(Origin::signed(ALICE), status_update_id, true),
            TestError::VoteAlreadyCast
        );
        Staking::end_block(3);
        assert_not_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            None,
            None,
            None
        ));
        assert_not_emitted!(Event::RejectStatusUpdate(StatusCode::Error, None, None));

        assert_ok!(Staking::vote_on_status_update(
            Origin::signed(CAROL),
            status_update_id,
            true
        ));
        Staking::end_block(3);
        assert_not_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            None,
            None,
            None
        ));
        assert_not_emitted!(Event::RejectStatusUpdate(StatusCode::Error, None, None));

        assert_ok!(Staking::vote_on_status_update(
            Origin::signed(BOB),
            status_update_id,
            true
        ));
        Staking::end_block(3);
        assert_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            None,
            None,
            None
        ));

        let status_update = Staking::inactive_status_update(status_update_id);
        assert_eq!(status_update.proposal_status, ProposalStatus::Accepted);
    })
}
#[test]
fn test_end_block_status_update_expired() {
    run_test(|| {
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        let status_update_id = inject_status_update(ALICE);
        Staking::end_block(DEFAULT_END_HEIGHT + 100);
        let status_update = Staking::inactive_status_update(&status_update_id);
        assert_eq!(status_update.proposal_status, ProposalStatus::Expired);
        assert_not_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            None,
            None,
            None
        ));
        assert_not_emitted!(Event::RejectStatusUpdate(StatusCode::Error, None, None));
        assert_emitted!(Event::ExpireStatusUpdate(status_update_id));
    })
}

#[test]
fn test_vote_on_status_update_fails_with_vote_already_cast() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);

        let status_update_id = inject_status_update(ALICE);

        assert_err!(
            Staking::vote_on_status_update(Origin::signed(ALICE), status_update_id, false),
            TestError::VoteAlreadyCast
        );

        assert_ok!(Staking::vote_on_status_update(
            Origin::signed(BOB),
            status_update_id,
            false
        ));

        assert_err!(
            Staking::vote_on_status_update(Origin::signed(BOB), status_update_id, false),
            TestError::VoteAlreadyCast
        );
    })
}

#[test]
fn test_execute_status_update_fails_with_insufficient_yes_votes() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        let mut status_update = StatusUpdate::default();
        status_update.tally.nay = account_id_set!(ALICE, BOB, CAROL);
        Staking::insert_active_status_update(status_update.clone());

        assert_err!(
            Staking::execute_status_update(&mut status_update),
            TestError::InsufficientYesVotes
        );

        assert_not_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::default(),
            None,
            None,
            None
        ));
    })
}

#[test]
fn test_execute_status_update_fails_with_no_block_hash() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        let mut status_update = StatusUpdate {
            new_status_code: StatusCode::Error,
            old_status_code: StatusCode::Running,
            add_error: Some(ErrorCode::NoDataBTCRelay),
            remove_error: None,
            start: DEFAULT_START_HEIGHT,
            end: DEFAULT_END_HEIGHT,
            proposal_status: ProposalStatus::Pending,
            btc_block_hash: None,
            proposer: ALICE,
            deposit: 10,
            tally: Tally {
                aye: account_id_set!(ALICE, BOB, CAROL),
                nay: account_id_set!(),
            },
            message: vec![],
        };

        Staking::insert_active_status_update(status_update.clone());

        assert_err!(
            Staking::execute_status_update(&mut status_update),
            TestError::ExpectedBlockHash
        );
    })
}

#[test]
fn test_execute_status_update_succeeds() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        let mut status_update = StatusUpdate {
            new_status_code: StatusCode::Error,
            old_status_code: StatusCode::Running,
            add_error: Some(ErrorCode::OracleOffline),
            remove_error: None,
            start: DEFAULT_START_HEIGHT,
            end: DEFAULT_END_HEIGHT,
            proposal_status: ProposalStatus::Pending,
            btc_block_hash: Some(H256Le::zero()),
            proposer: ALICE,
            deposit: 10,
            tally: Tally {
                aye: account_id_set!(ALICE, BOB, CAROL),
                nay: account_id_set!(),
            },
            message: vec![],
        };
        Staking::insert_active_status_update(status_update.clone());

        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_ok!(Staking::execute_status_update(&mut status_update));

        assert_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            Some(ErrorCode::OracleOffline),
            None,
            Some(H256Le::zero())
        ));
    })
}

#[test]
fn test_reject_status_update_fails_with_insufficient_no_votes() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        let mut status_update = StatusUpdate::default();
        status_update.tally.aye = account_id_set!(ALICE, BOB, CAROL);
        Staking::insert_active_status_update(status_update.clone());

        assert_err!(
            Staking::reject_status_update(&mut status_update),
            TestError::InsufficientNoVotes
        );

        assert_not_emitted!(Event::RejectStatusUpdate(StatusCode::default(), None, None));
    })
}

#[test]
fn test_reject_status_update_succeeds() {
    run_test(|| {
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);
        inject_active_staked_relayer(&BOB, amount);
        inject_active_staked_relayer(&CAROL, amount);
        inject_active_staked_relayer(&DAVE, amount);
        inject_active_staked_relayer(&EVE, amount);

        let mut status_update = StatusUpdate::default();
        status_update.tally.nay = account_id_set!(ALICE, BOB, CAROL);
        Staking::insert_active_status_update(status_update.clone());

        assert_ok!(Staking::reject_status_update(&mut status_update));

        assert_emitted!(Event::RejectStatusUpdate(StatusCode::default(), None, None));
    })
}

#[test]
fn test_force_status_update_fails_with_governance_only() {
    run_test(|| {
        Staking::only_governance
            .mock_safe(|_| MockResult::Return(Err(TestError::GovernanceOnly.into())));

        assert_err!(
            Staking::force_status_update(Origin::signed(ALICE), StatusCode::Shutdown, None, None),
            TestError::GovernanceOnly,
        );
    })
}

#[test]
fn test_force_status_update_succeeds() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(Staking::force_status_update(
            Origin::signed(ALICE),
            StatusCode::Shutdown,
            Some(ErrorCode::Liquidation),
            None
        ));

        assert_eq!(
            ext::security::get_parachain_status::<Test>(),
            StatusCode::Shutdown
        );

        assert_emitted!(Event::ForceStatusUpdate(
            StatusCode::Shutdown,
            Some(ErrorCode::Liquidation),
            None
        ));

        let errors = ext::security::get_errors::<Test>();
        assert_eq!(errors.contains(&ErrorCode::Liquidation), true);
    })
}

#[test]
fn test_slash_staked_relayer_fails_with_governance_only() {
    run_test(|| {
        Staking::only_governance
            .mock_safe(|_| MockResult::Return(Err(TestError::GovernanceOnly.into())));

        assert_err!(
            Staking::slash_staked_relayer(Origin::signed(ALICE), BOB),
            TestError::GovernanceOnly,
        );
    })
}

#[test]
fn test_slash_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        assert_err!(
            Staking::slash_staked_relayer(Origin::signed(ALICE), BOB),
            TestError::NotRegistered,
        );
    })
}

#[test]
fn test_slash_staked_relayer_succeeds() {
    run_test(|| {
        Staking::only_governance.mock_safe(|_| MockResult::Return(Ok(())));
        let amount: Balance = 5;
        inject_active_staked_relayer(&BOB, amount);
        ext::collateral::slash_collateral::<Test>.mock_safe(|sender, receiver, amount| {
            assert_eq!(sender, BOB);
            assert_eq!(receiver, ALICE);
            assert_eq!(amount, amount);
            MockResult::Return(Ok(()))
        });

        assert_ok!(Staking::slash_staked_relayer(Origin::signed(ALICE), BOB));
        assert_err!(
            Staking::get_active_staked_relayer(&BOB),
            TestError::NotRegistered
        );
        assert_emitted!(Event::SlashStakedRelayer(BOB));
    })
}

#[test]
fn test_report_vault_theft_fails_with_staked_relayers_only() {
    run_test(|| {
        assert_err!(
            Staking::report_vault_theft(
                Origin::signed(ALICE),
                CAROL,
                H256Le::zero(),
                vec![0u8; 32],
                vec![0u8; 32]
            ),
            TestError::StakedRelayersOnly,
        );
    })
}

#[test]
fn test_report_vault_passes_with_vault_transaction() {
    run_test(|| {
        let raw_tx = "0100000001c15041a06deb6b3818b022fac558da4ce2097f0860c8f642105bbad9d29be02a010000006c493046022100cfd2a2d332b29adce119c55a9fadd3c073332024b7e272513e51623ca15993480221009b482d7f7b4d479aff62bdcdaea54667737d56f8d4d63dd03ec3ef651ed9a25401210325f8b039a11861659c9bf03f43fc4ea055f3a71cd60c7b1fd474ab578f9977faffffffff0290d94000000000001976a9148ed243a7be26080a1a8cf96b53270665f1b8dd2388ac4083086b000000001976a9147e7d94d0ddc21d83bfbcfc7798e4547edf0832aa88ac00000000";

        let amount = 3;
        inject_active_staked_relayer(&ALICE, amount);
        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(
                vault.clone(),
                Some(btc_address),
            )))
        });
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>
            .mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(Staking::report_vault_theft(
            Origin::signed(ALICE),
            CAROL,
            H256Le::zero(),
            vec![0u8; 32],
            hex::decode(&raw_tx).unwrap()
        ),);
    })
}

#[test]
fn test_report_vault_fails_with_non_vault_transaction() {
    run_test(|| {
        let raw_tx = "0100000001c15041a06deb6b3818b022fac558da4ce2097f0860c8f642105bbad9d29be02a010000006c493046022100cfd2a2d332b29adce119c55a9fadd3c073332024b7e272513e51623ca15993480221009b482d7f7b4d479aff62bdcdaea54667737d56f8d4d63dd03ec3ef651ed9a25401210325f8b039a11861659c9bf03f43fc4ea055f3a71cd60c7b1fd474ab578f9977faffffffff0290d94000000000001976a9148ed243a7be26080a1a8cf96b53270665f1b8dd2388ac4083086b000000001976a9147e7d94d0ddc21d83bfbcfc7798e4547edf0832aa88ac00000000";

        let amount = 3;
        inject_active_staked_relayer(&ALICE, amount);
        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            125, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(
                vault.clone(),
                Some(btc_address),
            )))
        });
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_vault::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

        assert_err!(
            Staking::report_vault_theft(
                Origin::signed(ALICE),
                CAROL,
                H256Le::zero(),
                vec![0u8; 32],
                hex::decode(&raw_tx).unwrap()
            ),
            TestError::VaultNoInputToTransaction
        );
    })
}

#[test]
fn test_report_vault_succeeds_with_segwit_transaction() {
    run_test(|| {
        let raw_tx = "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000";

        let amount = 3;
        inject_active_staked_relayer(&ALICE, amount);
        let vault = CAROL;

        let btc_address = BtcAddress::P2WPKHv0(H160::from_slice(&[
            164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34, 105,
            85,
        ]));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(
                vault.clone(),
                Some(btc_address),
            )))
        });
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>
            .mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(Staking::report_vault_theft(
            Origin::signed(ALICE),
            CAROL,
            H256Le::zero(),
            vec![0u8; 32],
            hex::decode(&raw_tx).unwrap()
        ));
    })
}

#[test]
fn test_report_vault_theft_succeeds() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);

        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(move |_, _| MockResult::Return(Ok(())));
        Staking::is_transaction_invalid.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(())));

        assert_ok!(Staking::report_vault_theft(
            relayer,
            BOB,
            H256Le::zero(),
            vec![0u8; 32],
            vec![0u8; 32],
        ));
        assert_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            Some(ErrorCode::Liquidation),
            None,
            Some(H256Le::zero()),
        ));
    })
}

#[test]
fn test_report_vault_under_liquidation_threshold_fails() {
    run_test(|| {
        let relayer = ALICE;
        let vault = BOB;

        Staking::check_relayer_registered.mock_safe(|_| MockResult::Return(true));

        ext::vault_registry::is_vault_below_secure_threshold::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(false)));

        assert_err!(
            Staking::report_vault_under_liquidation_threshold(Origin::signed(relayer), vault),
            TestError::CollateralOk
        );
    })
}

#[test]
fn test_report_vault_under_liquidation_threshold_succeeds() {
    run_test(|| {
        let relayer = ALICE;
        let vault = BOB;

        Staking::check_relayer_registered.mock_safe(|_| MockResult::Return(true));

        ext::vault_registry::is_vault_below_secure_threshold::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(true)));

        ext::vault_registry::liquidate_vault::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(Staking::report_vault_under_liquidation_threshold(
            Origin::signed(relayer),
            vault
        ));

        assert_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            Some(ErrorCode::Liquidation),
            None,
            None,
        ));

        let parachain_status = ext::security::get_parachain_status::<Test>();
        assert_eq!(parachain_status, StatusCode::Error);
    })
}

#[test]
fn test_report_vault_under_liquidation_threshold_fails_with_staked_relayers_only() {
    run_test(|| {
        assert_err!(
            Staking::report_vault_under_liquidation_threshold(Origin::signed(ALICE), CAROL),
            TestError::StakedRelayersOnly,
        );
    })
}

#[test]
fn test_report_oracle_offline_fails_with_staked_relayers_only() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        assert_err!(
            Staking::report_oracle_offline(relayer),
            TestError::StakedRelayersOnly,
        );
    })
}

#[test]
fn test_report_oracle_offline_fails_with_already_reported() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);

        ext::security::get_errors::<Test>
            .mock_safe(|| MockResult::Return([ErrorCode::OracleOffline].iter().cloned().collect()));
        assert_err!(
            Staking::report_oracle_offline(relayer),
            TestError::OracleAlreadyReported,
        );
    })
}

#[test]
fn test_report_oracle_offline_fails_with_oracle_online() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);

        ext::oracle::is_max_delay_passed::<Test>.mock_safe(|| MockResult::Return(false));
        assert_err!(
            Staking::report_oracle_offline(relayer),
            TestError::OracleOnline,
        );
    })
}

#[test]
fn test_report_oracle_offline_succeeds() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        inject_active_staked_relayer(&ALICE, amount);

        ext::oracle::is_max_delay_passed::<Test>.mock_safe(|| MockResult::Return(true));

        assert_ok!(Staking::report_oracle_offline(relayer));
        assert_emitted!(Event::ExecuteStatusUpdate(
            StatusCode::Error,
            Some(ErrorCode::OracleOffline),
            None,
            None,
        ));
    })
}

#[test]
fn test_is_valid_merge_transaction_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(vault.clone(), None)))
        });

        let address1 =
            BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 =
            BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        assert_eq!(
            Staking::is_valid_merge_transaction(
                &vec![(100, address1)],
                &vec![],
                &Wallet::new(address2)
            ),
            false,
            "payment to unknown recipient"
        );

        assert_eq!(
            Staking::is_valid_merge_transaction(
                &vec![(100, address2)],
                &vec![(0, vec![])],
                &Wallet::new(address2)
            ),
            false,
            "migration should not have op_returns"
        );
    })
}

#[test]
fn test_is_valid_merge_transaction_succeeds() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(vault.clone(), None)))
        });

        let address =
            BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        assert_eq!(
            Staking::is_valid_merge_transaction(
                &vec![(100, address.clone())],
                &vec![],
                &Wallet::new(address)
            ),
            true
        );
    })
}

#[test]
fn test_is_valid_request_transaction_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(vault.clone(), None)))
        });

        let address1 =
            BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 =
            BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let actual_value: i32 = 50;

        let request_value = 100;
        let request_address = address1.clone();

        assert_eq!(
            Staking::is_valid_request_transaction(
                request_value,
                request_address,
                &vec![(actual_value.try_into().unwrap(), address1.clone())],
                &Wallet::new(address2)
            ),
            false
        );
    })
}

#[test]
fn test_is_valid_request_transaction_succeeds() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(vault.clone(), None)))
        });

        let address1 =
            BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 =
            BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let request_value = 100;
        let request_address = address1.clone();

        assert_eq!(
            Staking::is_valid_request_transaction(
                request_value,
                request_address,
                &vec![(request_value.try_into().unwrap(), address2)],
                &Wallet::new(address2)
            ),
            true
        );
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_merge_transaction() {
    run_test(|| {
        let address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                wallet: Wallet::new(address),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_sequence(4294967295)
                    .with_previous_index(1)
                    .with_previous_hash(H256Le::from_bytes_le(&[
                        193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76,
                        226, 9, 127, 8, 96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                    ]))
                    .with_script(&[
                        73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197,
                        90, 159, 173, 211, 192, 115, 51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60,
                        161, 89, 147, 72, 2, 33, 0, 155, 72, 45, 127, 123, 77, 71, 154, 255, 98,
                        189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214, 61, 208, 62, 195,
                        239, 101, 30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101,
                        156, 155, 240, 63, 67, 252, 78, 160, 85, 243, 167, 28, 214, 12, 123, 31,
                        212, 116, 171, 87, 143, 153, 119, 250,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &address))
            .build();

        assert_err!(
            Staking::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidMergeTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_request_or_redeem() {
    run_test(|| {
        let address1 = BtcAddress::P2WPKHv0(H160::from_slice(&[
            164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34, 105,
            85,
        ]));

        let address2 =
            BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                wallet: Wallet::new(address1),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });

        ext::redeem::get_redeem_request_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(RedeemRequest {
                vault: BOB,
                opentime: 0,
                amount_polka_btc: 0,
                fee: 0,
                amount_btc: 100,
                amount_dot: 0,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: address2,
                completed: false,
                cancelled: false,
                reimburse: false,
            }))
        });

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_previous_index(1)
                    .with_previous_hash(H256Le::from_hex_le(
                        "40d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f742",
                    ))
                    .with_sequence(4294967295)
                    .with_script(&[
                        22, 0, 20, 164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220,
                        141, 186, 174, 34, 105, 85,
                    ])
                    .add_witness(&[
                        48, 69, 2, 33, 0, 134, 4, 239, 143, 109, 138, 250, 137, 45, 238, 15, 49,
                        37, 155, 108, 224, 45, 215, 12, 84, 92, 252, 254, 216, 20, 129, 121, 151,
                        24, 118, 197, 74, 2, 32, 118, 215, 113, 214, 233, 27, 237, 33, 39, 131,
                        201, 176, 110, 13, 230, 0, 250, 178, 213, 24, 250, 214, 241, 90, 43, 25,
                        29, 127, 189, 38, 42, 62, 1,
                    ])
                    .add_witness(&[
                        3, 157, 37, 171, 121, 244, 31, 117, 206, 175, 136, 36, 17, 253, 65, 250,
                        103, 10, 76, 103, 44, 35, 255, 175, 14, 54, 26, 150, 156, 222, 6, 146, 232,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &address2))
            .add_output(TransactionOutput::op_return(
                0,
                &H256::from_slice(&[0; 32]).as_bytes(),
            ))
            .build();

        assert_err!(
            Staking::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidRedeemTransaction
        );

        ext::redeem::get_redeem_request_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Err(RedeemError::RedeemIdNotFound.into())));

        ext::replace::get_replace_request::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(ReplaceRequest {
                old_vault: BOB,
                open_time: 0,
                amount: 100,
                griefing_collateral: 0,
                new_vault: None,
                collateral: 0,
                accept_time: None,
                btc_address: address2,
                completed: false,
                cancelled: false,
            }))
        });

        assert_err!(
            Staking::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidReplaceTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_succeeds() {
    run_test(|| {
        let vault_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8,
            50, 170,
        ]));

        let address =
            BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(BOB, Some(vault_address))))
        });

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_sequence(4294967295)
                    .with_previous_index(1)
                    .with_previous_hash(H256Le::from_bytes_le(&[
                        193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76,
                        226, 9, 127, 8, 96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                    ]))
                    .with_script(&[
                        73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197,
                        90, 159, 173, 211, 192, 115, 51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60,
                        161, 89, 147, 72, 2, 33, 0, 155, 72, 45, 127, 123, 77, 71, 154, 255, 98,
                        189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214, 61, 208, 62, 195,
                        239, 101, 30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101,
                        156, 155, 240, 63, 67, 252, 78, 160, 85, 243, 167, 28, 214, 12, 123, 31,
                        212, 116, 171, 87, 143, 153, 119, 250,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &address))
            .build();

        assert_ok!(Staking::is_transaction_invalid(&BOB, transaction.format()));
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_merge_testnet_transaction() {
    run_test(|| {
        // bitcoin-cli -testnet getrawtransaction "3453e52ebab8ac96159d6b19114b492a05cce05a8fdfdaf5dea266ac10601ce4" 0 "00000000000000398849cc9d67261ec2d5fea07db87ab66a8ea47bc05acfb194"
        let raw_tx_hex = "0200000000010108ce8e8943edbbf09d070bb893e09c0de12c0cf3704fe8a9b0f8b8d1a4a7a4760000000017160014473ca3f4d726ce9c21af7cdc3fcc13264f681b04feffffff02b377413f0000000017a914fe5183ccb89d98beaa6908c7cf1bd109029482cf87142e1a00000000001976a914d0a46d39dafa3012c2a7ed4d82d644b428e4586b88ac02473044022069484377c6627ccca566d4c4ac2cb84d1b0662f5ffbd384815c5e98b072759fc022061de3b77b4543ef43bb969d3f97fbbbdcddc008438720e7026181d99c455b2410121034172c29d3da8279f71adda48db8281d65b794e73cf04ea91fac4293030f0fe91a3ee1c00";
        let raw_tx = hex::decode(&raw_tx_hex).unwrap();

        // 2MsqorfMrsvXiVM8pD9bPWxGnccSWsj16XE (P2WPKH-P2SH)
        let vault_btc_address_0 = BtcAddress::P2WPKHv0(H160::from_slice(
            &hex::decode("473ca3f4d726ce9c21af7cdc3fcc13264f681b04").unwrap(),
        ));

        // 2NGRwGkzypA4fEz9m4KhA2ZBs7fTg3B7Zjo
        let vault_btc_address_1 = BtcAddress::P2SH(H160::from_slice(
            &hex::decode("fe5183ccb89d98beaa6908c7cf1bd109029482cf").unwrap(),
        ));

        // mzY9pX6NA3cBmiC4umbBzf1NdwrmjS7MS8
        let vault_btc_address_2 = BtcAddress::P2PKH(H160::from_slice(
            &hex::decode("d0a46d39dafa3012c2a7ed4d82d644b428e4586b").unwrap(),
        ));

        let mut wallet = Wallet::new(vault_btc_address_0);
        wallet.add_btc_address(vault_btc_address_1);
        wallet.add_btc_address(vault_btc_address_2);

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                wallet: wallet.clone(),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });

        assert_err!(
            Staking::is_transaction_invalid(&BOB, raw_tx),
            TestError::ValidMergeTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_succeeds_with_testnet_transaction() {
    run_test(|| {
        // bitcoin-cli -testnet getrawtransaction "3453e52ebab8ac96159d6b19114b492a05cce05a8fdfdaf5dea266ac10601ce4" 0 "00000000000000398849cc9d67261ec2d5fea07db87ab66a8ea47bc05acfb194"
        let raw_tx_hex = "0200000000010108ce8e8943edbbf09d070bb893e09c0de12c0cf3704fe8a9b0f8b8d1a4a7a4760000000017160014473ca3f4d726ce9c21af7cdc3fcc13264f681b04feffffff02b377413f0000000017a914fe5183ccb89d98beaa6908c7cf1bd109029482cf87142e1a00000000001976a914d0a46d39dafa3012c2a7ed4d82d644b428e4586b88ac02473044022069484377c6627ccca566d4c4ac2cb84d1b0662f5ffbd384815c5e98b072759fc022061de3b77b4543ef43bb969d3f97fbbbdcddc008438720e7026181d99c455b2410121034172c29d3da8279f71adda48db8281d65b794e73cf04ea91fac4293030f0fe91a3ee1c00";
        let raw_tx = hex::decode(&raw_tx_hex).unwrap();

        // 2MsqorfMrsvXiVM8pD9bPWxGnccSWsj16XE (P2WPKH-P2SH)
        let btc_address = BtcAddress::P2WPKHv0(H160::from_slice(
            &hex::decode("473ca3f4d726ce9c21af7cdc3fcc13264f681b04").unwrap(),
        ));

        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(init_zero_vault::<Test>(BOB, Some(btc_address))))
        });

        assert_ok!(Staking::is_transaction_invalid(&BOB, raw_tx));
    })
}

#[test]
fn test_get_status_counter_success() {
    run_test(|| {
        assert_eq!(Staking::get_status_counter(), 1);
        assert_eq!(Staking::get_status_counter(), 2);
    })
}

#[test]
fn test_remove_active_status_update_only_root() {
    run_test(|| {
        let status_update = StatusUpdate::default();
        let status_update_id = Staking::insert_active_status_update(status_update);

        assert_noop!(
            Staking::remove_active_status_update(Origin::signed(ALICE), status_update_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Staking::remove_active_status_update(
            Origin::root(),
            status_update_id
        ));
    })
}

#[test]
fn test_remove_inactive_status_update_only_root() {
    run_test(|| {
        let status_update_id = 0;
        let status_update = StatusUpdate::default();
        Staking::insert_inactive_status_update(status_update_id, &status_update);

        assert_noop!(
            Staking::remove_inactive_status_update(Origin::signed(ALICE), status_update_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Staking::remove_inactive_status_update(
            Origin::root(),
            status_update_id
        ));
    })
}
