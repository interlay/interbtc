extern crate hex;
use crate::{ext, mock::*, sp_api_hidden_includes_decl_storage::hidden_include::StorageMap, Stakes};
use bitcoin::{
    formatter::Formattable,
    types::{H256Le, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput},
};
use btc_relay::{BtcAddress, BtcPublicKey, Error as BtcRelayError};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use redeem::types::{RedeemRequest, RedeemRequestStatus};
use replace::types::{ReplaceRequest, ReplaceRequestStatus};
use sp_core::{H160, H256};
use std::{convert::TryInto, str::FromStr};
use vault_registry::{Vault, VaultStatus, Wallet};
type Event = crate::Event<Test>;

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::staked_relayers($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::staked_relayers($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

/// Mocking functions
fn init_zero_vault(id: AccountId, btc_address: Option<BtcAddress>) -> Vault<AccountId, BlockNumber, u64, u64> {
    let mut vault = Vault::default();
    vault.id = id;
    vault.wallet = Wallet::new(dummy_public_key());
    match btc_address {
        Some(btc_address) => vault.wallet.add_btc_address(btc_address),
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
            StakedRelayers::register_staked_relayer(relayer, amount),
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

        assert_ok!(StakedRelayers::register_staked_relayer(relayer.clone(), amount));
        assert_emitted!(Event::RegisterStakedRelayer(ALICE, amount));

        // re-registration not allowed
        assert_err!(
            StakedRelayers::register_staked_relayer(relayer, amount),
            TestError::AlreadyRegistered,
        );

        assert_eq!(StakedRelayers::stakes(ALICE), amount);
    })
}

#[test]
fn test_deregister_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);

        assert_err!(
            StakedRelayers::deregister_staked_relayer(relayer),
            TestError::NotRegistered,
        );
    })
}

#[test]
fn test_deregister_staked_relayer_succeeds() {
    run_test(|| {
        let relayer = Origin::signed(ALICE);
        let amount: Balance = 3;
        <Stakes<Test>>::insert(&ALICE, amount);

        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(StakedRelayers::deregister_staked_relayer(relayer));

        assert!(!<Stakes<Test>>::contains_key(&BOB));

        assert_emitted!(Event::DeregisterStakedRelayer(ALICE));
    })
}

#[test]
fn test_slash_staked_relayer_fails_with_non_root() {
    run_test(|| {
        assert_err!(
            StakedRelayers::slash_staked_relayer(Origin::signed(ALICE), BOB),
            DispatchError::BadOrigin
        );
    })
}

#[test]
fn test_slash_staked_relayer_fails_with_not_registered() {
    run_test(|| {
        assert_err!(
            StakedRelayers::slash_staked_relayer(Origin::root(), BOB),
            TestError::NotRegistered,
        );
    })
}

#[test]
fn test_slash_staked_relayer_succeeds() {
    run_test(|| {
        let amount: Balance = 5;
        <Stakes<Test>>::insert(&BOB, amount);

        ext::collateral::slash_collateral::<Test>.mock_safe(|sender, receiver, _amount| {
            assert_eq!(sender, BOB);
            assert_eq!(receiver, ext::fee::fee_pool_account_id::<Test>());
            MockResult::Return(Ok(()))
        });

        assert_ok!(StakedRelayers::slash_staked_relayer(Origin::root(), BOB));
        assert!(!<Stakes<Test>>::contains_key(&BOB));
        assert_emitted!(Event::SlashStakedRelayer(BOB));
    })
}

#[test]
fn test_report_vault_passes_with_vault_transaction() {
    run_test(|| {
        let raw_tx = "0100000001c15041a06deb6b3818b022fac558da4ce2097f0860c8f642105bbad9d29be02a010000006c493046022100cfd2a2d332b29adce119c55a9fadd3c073332024b7e272513e51623ca15993480221009b482d7f7b4d479aff62bdcdaea54667737d56f8d4d63dd03ec3ef651ed9a25401210325f8b039a11861659c9bf03f43fc4ea055f3a71cd60c7b1fd474ab578f9977faffffffff0290d94000000000001976a9148ed243a7be26080a1a8cf96b53270665f1b8dd2388ac4083086b000000001976a9147e7d94d0ddc21d83bfbcfc7798e4547edf0832aa88ac00000000";

        let amount = 3;
        <Stakes<Test>>::insert(&ALICE, amount);

        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, Some(btc_address)))));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(StakedRelayers::report_vault_theft(
            Origin::signed(ALICE),
            CAROL,
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
        <Stakes<Test>>::insert(&ALICE, amount);

        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            125, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, Some(btc_address)))));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));

        assert_err!(
            StakedRelayers::report_vault_theft(
                Origin::signed(ALICE),
                CAROL,
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
        <Stakes<Test>>::insert(&ALICE, amount);

        let vault = CAROL;

        let btc_address = BtcAddress::P2WPKHv0(H160::from_slice(&[
            164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34, 105, 85,
        ]));
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, Some(btc_address)))));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(StakedRelayers::report_vault_theft(
            Origin::signed(ALICE),
            CAROL,
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
        <Stakes<Test>>::insert(&ALICE, amount);

        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        StakedRelayers::is_transaction_invalid.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(move |_| MockResult::Return(Ok(())));

        assert_ok!(StakedRelayers::report_vault_theft(
            relayer,
            BOB,
            vec![0u8; 32],
            vec![0u8; 32],
        ));
        // check that the event has been emitted
        assert!(System::events()
            .iter()
            .any(|a| matches!(a.event, TestEvent::staked_relayers(Event::VaultTheft(id, _)) if id == BOB)));
    })
}

#[test]
fn test_is_valid_merge_transaction_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, None))));

        let address1 = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        assert_eq!(
            StakedRelayers::is_valid_merge_transaction(&[(100, address1)], &[], &Wallet::new(dummy_public_key())),
            false,
            "payment to unknown recipient"
        );

        assert_eq!(
            StakedRelayers::is_valid_merge_transaction(
                &[(100, address2)],
                &[(0, vec![])],
                &Wallet::new(dummy_public_key())
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
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, None))));

        let address = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(address);

        assert_eq!(
            StakedRelayers::is_valid_merge_transaction(&[(100, address)], &[], &wallet),
            true
        );
    })
}

#[test]
fn test_is_valid_request_transaction_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault, None))));

        let address1 = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(address2);

        let actual_value: i32 = 50;

        let request_value = 100;
        let request_address = address1;

        assert_eq!(
            StakedRelayers::is_valid_request_transaction(
                request_value,
                request_address,
                &[(actual_value.try_into().unwrap(), address1)],
                &wallet
            ),
            false
        );
    })
}

#[test]
fn test_is_valid_request_transaction_succeeds() {
    run_test(|| {
        let recipient_address = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let vault_address = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let request_value = 100;
        let change_value = 50;

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(vault_address);

        assert_eq!(
            StakedRelayers::is_valid_request_transaction(
                request_value,
                recipient_address,
                &[
                    (request_value.try_into().unwrap(), recipient_address),
                    (change_value.try_into().unwrap(), vault_address)
                ],
                &wallet
            ),
            true
        );
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_merge_transaction() {
    run_test(|| {
        let address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(address);

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                replace_collateral: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: wallet.clone(),
                banned_until: None,
                status: VaultStatus::Active(true),
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
                        193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8, 96,
                        200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                    ]))
                    .with_script(&[
                        73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197, 90, 159, 173, 211,
                        192, 115, 51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60, 161, 89, 147, 72, 2, 33, 0, 155, 72,
                        45, 127, 123, 77, 71, 154, 255, 98, 189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214,
                        61, 208, 62, 195, 239, 101, 30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101,
                        156, 155, 240, 63, 67, 252, 78, 160, 85, 243, 167, 28, 214, 12, 123, 31, 212, 116, 171, 87,
                        143, 153, 119, 250,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &address))
            .build();

        assert_err!(
            StakedRelayers::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidMergeTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_request_or_redeem() {
    run_test(|| {
        let vault_address = BtcAddress::P2WPKHv0(H160::from_slice(&[
            164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34, 105, 85,
        ]));

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(vault_address);

        let recipient_address = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                backing_collateral: 0,
                wallet: wallet.clone(),
                banned_until: None,
                status: VaultStatus::Active(true),
            }))
        });

        ext::redeem::get_open_or_completed_redeem_request_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 0,
                fee: 0,
                amount_btc: 100,
                premium_dot: 0,
                redeemer: ALICE,
                btc_address: recipient_address,
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
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
                        22, 0, 20, 164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34,
                        105, 85,
                    ])
                    .add_witness(&[
                        48, 69, 2, 33, 0, 134, 4, 239, 143, 109, 138, 250, 137, 45, 238, 15, 49, 37, 155, 108, 224, 45,
                        215, 12, 84, 92, 252, 254, 216, 20, 129, 121, 151, 24, 118, 197, 74, 2, 32, 118, 215, 113, 214,
                        233, 27, 237, 33, 39, 131, 201, 176, 110, 13, 230, 0, 250, 178, 213, 24, 250, 214, 241, 90, 43,
                        25, 29, 127, 189, 38, 42, 62, 1,
                    ])
                    .add_witness(&[
                        3, 157, 37, 171, 121, 244, 31, 117, 206, 175, 136, 36, 17, 253, 65, 250, 103, 10, 76, 103, 44,
                        35, 255, 175, 14, 54, 26, 150, 156, 222, 6, 146, 232,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &recipient_address))
            .add_output(TransactionOutput::op_return(0, &H256::from_slice(&[0; 32]).as_bytes()))
            .build();

        assert_err!(
            StakedRelayers::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidRedeemTransaction
        );

        ext::redeem::get_open_or_completed_redeem_request_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Err(RedeemError::RedeemIdNotFound.into())));

        ext::replace::get_open_or_completed_replace_request::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(ReplaceRequest {
                period: 0,
                old_vault: BOB,
                amount: 100,
                griefing_collateral: 0,
                new_vault: ALICE,
                collateral: 0,
                accept_time: 1,
                btc_address: recipient_address,
                btc_height: 0,
                status: ReplaceRequestStatus::Pending,
            }))
        });

        assert_err!(
            StakedRelayers::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidReplaceTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_succeeds() {
    run_test(|| {
        let vault_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));

        let recipient_address = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(BOB, Some(vault_address)))));

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_sequence(4294967295)
                    .with_previous_index(1)
                    .with_previous_hash(H256Le::from_bytes_le(&[
                        193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8, 96,
                        200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                    ]))
                    .with_script(&[
                        73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197, 90, 159, 173, 211,
                        192, 115, 51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60, 161, 89, 147, 72, 2, 33, 0, 155, 72,
                        45, 127, 123, 77, 71, 154, 255, 98, 189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214,
                        61, 208, 62, 195, 239, 101, 30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101,
                        156, 155, 240, 63, 67, 252, 78, 160, 85, 243, 167, 28, 214, 12, 123, 31, 212, 116, 171, 87,
                        143, 153, 119, 250,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(100, &recipient_address))
            .build();

        assert_ok!(StakedRelayers::is_transaction_invalid(&BOB, transaction.format()));
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_merge_testnet_transaction() {
    run_test(|| {
        // bitcoin-cli -testnet getrawtransaction "3453e52ebab8ac96159d6b19114b492a05cce05a8fdfdaf5dea266ac10601ce4" 0
        // "00000000000000398849cc9d67261ec2d5fea07db87ab66a8ea47bc05acfb194"
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

        let mut wallet = Wallet::new(dummy_public_key());
        wallet.add_btc_address(vault_btc_address_0);
        wallet.add_btc_address(vault_btc_address_1);
        wallet.add_btc_address(vault_btc_address_2);

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                replace_collateral: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: wallet.clone(),
                banned_until: None,
                status: VaultStatus::Active(true),
            }))
        });

        assert_err!(
            StakedRelayers::is_transaction_invalid(&BOB, raw_tx),
            TestError::ValidMergeTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_succeeds_with_testnet_transaction() {
    run_test(|| {
        // bitcoin-cli -testnet getrawtransaction "3453e52ebab8ac96159d6b19114b492a05cce05a8fdfdaf5dea266ac10601ce4" 0
        // "00000000000000398849cc9d67261ec2d5fea07db87ab66a8ea47bc05acfb194"
        let raw_tx_hex = "0200000000010108ce8e8943edbbf09d070bb893e09c0de12c0cf3704fe8a9b0f8b8d1a4a7a4760000000017160014473ca3f4d726ce9c21af7cdc3fcc13264f681b04feffffff02b377413f0000000017a914fe5183ccb89d98beaa6908c7cf1bd109029482cf87142e1a00000000001976a914d0a46d39dafa3012c2a7ed4d82d644b428e4586b88ac02473044022069484377c6627ccca566d4c4ac2cb84d1b0662f5ffbd384815c5e98b072759fc022061de3b77b4543ef43bb969d3f97fbbbdcddc008438720e7026181d99c455b2410121034172c29d3da8279f71adda48db8281d65b794e73cf04ea91fac4293030f0fe91a3ee1c00";
        let raw_tx = hex::decode(&raw_tx_hex).unwrap();

        // 2MsqorfMrsvXiVM8pD9bPWxGnccSWsj16XE (P2WPKH-P2SH)
        let btc_address = BtcAddress::P2WPKHv0(H160::from_slice(
            &hex::decode("473ca3f4d726ce9c21af7cdc3fcc13264f681b04").unwrap(),
        ));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(BOB, Some(btc_address)))));

        assert_ok!(StakedRelayers::is_transaction_invalid(&BOB, raw_tx));
    })
}

#[test]
fn test_store_block_header_and_update_sla_succeeds_with_duplicate() {
    run_test(|| {
        ext::btc_relay::store_block_header::<Test>
            .mock_safe(|_, _| MockResult::Return(Err(BtcRelayError::<Test>::DuplicateBlock.into())));

        ext::sla::event_update_relayer_sla::<Test>.mock_safe(|&relayer_id, event| {
            assert_eq!(relayer_id, 0);
            assert_eq!(event, ext::sla::RelayerEvent::DuplicateBlockSubmission);
            MockResult::Return(Ok(()))
        });

        assert_ok!(StakedRelayers::store_block_header_and_update_sla(
            &0,
            RawBlockHeader::default()
        ));
    })
}

#[test]
fn test_store_block_header_and_update_sla_fails_with_invalid() {
    run_test(|| {
        ext::btc_relay::store_block_header::<Test>
            .mock_safe(|_, _| MockResult::Return(Err(BtcRelayError::<Test>::DiffTargetHeader.into())));

        ext::sla::event_update_relayer_sla::<Test>.mock_safe(|_, _| {
            panic!("Should not call sla update for invalid block");
        });

        assert_err!(
            StakedRelayers::store_block_header_and_update_sla(&0, RawBlockHeader::default()),
            BtcRelayError::<Test>::DiffTargetHeader
        );
    })
}

#[test]
fn test_runtime_upgrade() {
    use crate::{sp_api_hidden_includes_decl_storage::hidden_include::StorageHasher, types::StakedRelayer};
    use codec::Encode;
    use frame_support::{storage::migration, Blake2_128Concat};

    run_test(|| {
        let active: StakedRelayer<Balance, BlockNumber> = StakedRelayer {
            height: 51,
            stake: 12u32.into(),
        };
        migration::put_storage_value(
            b"Staking",
            b"ActiveStakedRelayers",
            &Blake2_128Concat::hash(&ALICE.encode()),
            active,
        );

        let inactive: StakedRelayer<Balance, BlockNumber> = StakedRelayer {
            height: 51,
            stake: 13u32.into(),
        };
        migration::put_storage_value(
            b"Staking",
            b"ActiveStakedRelayers",
            &Blake2_128Concat::hash(&BOB.encode()),
            inactive,
        );

        StakedRelayers::_on_runtime_upgrade();
        assert_eq!(crate::Stakes::<Test>::get(ALICE), 12);
        assert_eq!(crate::Stakes::<Test>::get(BOB), 13);
    })
}
