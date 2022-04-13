extern crate hex;
use crate::{ext, mock::*};
use bitcoin::{
    formatter::Formattable,
    types::{
        H256Le, MerkleProof, Transaction, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
        TransactionOutput,
    },
};
use btc_relay::{BtcAddress, BtcPublicKey, OpReturnPaymentData};
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use redeem::types::{RedeemRequest, RedeemRequestStatus};
use replace::types::{ReplaceRequest, ReplaceRequestStatus};
use sp_core::{H160, H256};
use sp_std::convert::TryFrom;
use std::{convert::TryInto, str::FromStr};
use vault_registry::{types::DefaultVault, DefaultVaultId, Vault, Wallet};

type Event = crate::Event<Test>;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

/// Mocking functions
fn init_zero_vault(id: DefaultVaultId<Test>, btc_addresses: Vec<BtcAddress>) -> DefaultVault<Test> {
    let mut vault = Vault {
        wallet: Wallet::new(),
        ..Vault::new(id)
    };

    for btc_address in btc_addresses.iter() {
        vault.wallet.add_btc_address(*btc_address)
    }
    vault
}

#[test]
fn test_report_vault_passes_with_vault_transaction() {
    run_test(|| {
        let raw_tx = "0100000001c15041a06deb6b3818b022fac558da4ce2097f0860c8f642105bbad9d29be02a010000006c493046022100cfd2a2d332b29adce119c55a9fadd3c073332024b7e272513e51623ca15993480221009b482d7f7b4d479aff62bdcdaea54667737d56f8d4d63dd03ec3ef651ed9a25401210325f8b039a11861659c9bf03f43fc4ea055f3a71cd60c7b1fd474ab578f9977faffffffff0290d94000000000001976a9148ed243a7be26080a1a8cf96b53270665f1b8dd2388ac4083086b000000001976a9147e7d94d0ddc21d83bfbcfc7798e4547edf0832aa88ac00000000";

        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![btc_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Relay::report_vault_theft(
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

        let vault = CAROL;

        let btc_address = BtcAddress::P2PKH(H160::from_slice(&[
            125, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![btc_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));

        assert_err!(
            Relay::report_vault_theft(
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

        let vault = CAROL;

        let btc_address = BtcAddress::P2SH(H160::from_str(&"2928f43af18d2d60e8a843540d8086b305341339").unwrap());
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![btc_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Relay::report_vault_theft(
            Origin::signed(ALICE),
            CAROL,
            vec![0u8; 32],
            hex::decode(&raw_tx).unwrap()
        ));
    })
}

#[test]
fn test_report_vault_succeeds_with_p2sh_segwit_transaction() {
    run_test(|| {
        // source: https://blockstream.info/tx/0a0d7b9ab879fbd7a096e856fa5461dbae959ac86d51451c211a65fb8e95f54b?expand
        let raw_tx = "02000000000101a1dcf3ca033463e346339642dd7305e33de4ce5ab179d1e19b1eb146534421660000000017160014a97a9058829417d4c581ad5004b6e46cc680063dfdffffff01b9010000000000001600143b05c08e224ddec538ac7aa2e3b6583b983807a302473044022051480b10ef40d12bce982d1d08176a403f176dd3e51189c07a0a9584ddb8e91602204a02134b2b892904a3519da0044e97da9ae78232f8f7678fea0b6531bf3104130121039dcac4d315739516bf5cea98bc6a9cfb49cb6269beb67c520bc5ecacc3c7d47206c70900";

        let vault = CAROL;
        // 35PLQyoXs2sk9QDqMv7bBGowxP1pjwXAMe
        let btc_address = BtcAddress::P2SH(H160::from_str(&"288873634ae24a3c9b6792cc7e2a084ec79ef68b").unwrap());
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![btc_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Relay::report_vault_theft(
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

        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        Relay::_is_parsed_transaction_invalid.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));

        let raw_proof = hex::decode("00000020ecf348128755dbeea5deb8eddf64566d9d4e59bc65d485000000000000000000901f0d92a66ee7dcefd02fa282ca63ce85288bab628253da31ef259b24abe8a0470a385a45960018e8d672f8a90a00000d0bdabada1fb6e3cef7f5c6e234621e3230a2f54efc1cba0b16375d9980ecbc023cbef3ba8d8632ea220927ec8f95190b30769eb35d87618f210382c9445f192504074f56951b772efa43b89320d9c430b0d156b93b7a1ff316471e715151a0619a39392657f25289eb713168818bd5b37476f1bc59b166deaa736d8a58756f9d7ce2aef46d8004c5fe3293d883838f87b5f1da03839878895b71530e9ff89338bb6d4578b3c3135ff3e8671f9a64d43b22e14c2893e8271cecd420f11d2359307403bb1f3128885b3912336045269ef909d64576b93e816fa522c8c027fe408700dd4bdee0254c069ccb728d3516fe1e27578b31d70695e3e35483da448f3a951273e018de7f2a8f657064b013c6ede75c74bbd7f98fdae1c2ac6789ee7b21a791aa29d60e89fff2d1d2b1ada50aa9f59f403823c8c58bb092dc58dc09b28158ca15447da9c3bedb0b160f3fe1668d5a27716e27661bcb75ddbf3468f5c76b7bed1004c6b4df4da2ce80b831a7c260b515e6355e1c306373d2233e8de6fda3674ed95d17a01a1f64b27ba88c3676024fbf8d5dd962ffc4d5e9f3b1700763ab88047f7d0000").unwrap();
        let tx_bytes = hex::decode("0100000001c8cc2b56525e734ff63a13bc6ad06a9e5664df8c67632253a8e36017aee3ee40000000009000483045022100ad0851c69dd756b45190b5a8e97cb4ac3c2b0fa2f2aae23aed6ca97ab33bf88302200b248593abc1259512793e7dea61036c601775ebb23640a0120b0dba2c34b79001455141042f90074d7a5bf30c72cf3a8dfd1381bdbd30407010e878f3a11269d5f74a58788505cdca22ea6eab7cfb40dc0e07aba200424ab0d79122a653ad0c7ec9896bdf51aefeffffff0120f40e00000000001976a9141d30342095961d951d306845ef98ac08474b36a088aca7270400").unwrap();

        assert_ok!(Relay::report_vault_theft(relayer, BOB, raw_proof, tx_bytes,));
        // check that the event has been emitted
        assert!(System::events()
            .iter()
            .any(|a| matches!(a.event, TestEvent::Relay(Event::VaultTheft{ref vault_id,..}) if vault_id == &BOB)));
    })
}

fn build_dummy_transaction_from_input_with_output_and_op_return(
    prev_hash: H256Le,
    public_key: &BtcPublicKey,
    output: BtcAddress,
    op_return: &[u8],
) -> Transaction {
    let mut builder = TransactionBuilder::new();
    builder.with_version(1).add_input(
        TransactionInputBuilder::new()
            .with_source(TransactionInputSource::FromOutput(prev_hash, 1))
            .with_sequence(4294967295)
            .with_p2pkh(public_key, vec![1u8; 32])
            .build(),
    );
    builder.add_output(TransactionOutput::payment(100, &output));
    builder.add_output(TransactionOutput::op_return(0, &op_return));
    builder.build()
}

fn build_dummy_transaction_with_output(addresses: Vec<BtcAddress>) -> Transaction {
    let mut builder = TransactionBuilder::new();
    builder.with_version(1).add_input(
        TransactionInputBuilder::new()
            .with_sequence(4294967295)
            .with_source(TransactionInputSource::FromOutput(
                H256Le::from_bytes_le(&[
                    193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8, 96, 200,
                    246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                ]),
                1,
            ))
            .with_script(&[
                73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197, 90, 159, 173, 211, 192, 115,
                51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60, 161, 89, 147, 72, 2, 33, 0, 155, 72, 45, 127, 123, 77,
                71, 154, 255, 98, 189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214, 61, 208, 62, 195, 239, 101,
                30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101, 156, 155, 240, 63, 67, 252, 78, 160,
                85, 243, 167, 28, 214, 12, 123, 31, 212, 116, 171, 87, 143, 153, 119, 250,
            ])
            .build(),
    );
    for address in addresses {
        builder.add_output(TransactionOutput::payment(100, &address));
    }
    builder.build()
}

#[test]
fn test_is_valid_merge_transaction_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![]))));

        let address1 = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let transaction1 = build_dummy_transaction_with_output(vec![address1]);
        assert_eq!(
            Relay::is_valid_merge_transaction(&transaction1, &Wallet::new()),
            false,
            "payment to unknown recipient"
        );

        let address2 = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());
        let transaction2 = build_dummy_transaction_with_output(vec![address2]);
        assert_eq!(
            Relay::is_valid_merge_transaction(&transaction2, &Wallet::new()),
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
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![]))));

        let address = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let transaction = build_dummy_transaction_with_output(vec![address]);

        let mut wallet = Wallet::new();
        wallet.add_btc_address(address);

        assert_eq!(Relay::is_valid_merge_transaction(&transaction, &wallet), true);
    })
}

fn build_dummy2_transaction_with(output_addresses: Vec<(i64, BtcAddress)>, op_return: H256) -> Transaction {
    let mut builder = TransactionBuilder::new();
    builder.with_version(1).add_input(
        TransactionInputBuilder::new()
            .with_sequence(4294967295)
            .with_source(TransactionInputSource::FromOutput(
                H256Le::from_bytes_le(&[
                    193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8, 96, 200,
                    246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                ]),
                1,
            ))
            .with_script(&[
                73, 48, 70, 2, 33, 0, 207, 210, 162, 211, 50, 178, 154, 220, 225, 25, 197, 90, 159, 173, 211, 192, 115,
                51, 32, 36, 183, 226, 114, 81, 62, 81, 98, 60, 161, 89, 147, 72, 2, 33, 0, 155, 72, 45, 127, 123, 77,
                71, 154, 255, 98, 189, 205, 174, 165, 70, 103, 115, 125, 86, 248, 212, 214, 61, 208, 62, 195, 239, 101,
                30, 217, 162, 84, 1, 33, 3, 37, 248, 176, 57, 161, 24, 97, 101, 156, 155, 240, 63, 67, 252, 78, 160,
                85, 243, 167, 28, 214, 12, 123, 31, 212, 116, 171, 87, 143, 153, 119, 250,
            ])
            .build(),
    );
    for (amount, address) in output_addresses {
        builder.add_output(TransactionOutput::payment(amount, &address));
    }
    builder.add_output(TransactionOutput::op_return(0, op_return.as_bytes()));
    builder.build()
}

#[test]
fn test_is_valid_request_transaction_overpayment_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![]))));

        let address1 = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let mut wallet = Wallet::new();
        wallet.add_btc_address(address2);

        let actual_value = 101;

        let request_value = 100;
        let request_address = address1;

        let transaction = build_dummy2_transaction_with(vec![(actual_value, address1)], H256::zero());
        let payment_data = OpReturnPaymentData::try_from(transaction).unwrap();

        assert_eq!(
            Relay::is_valid_request_transaction(request_value, request_address, &payment_data, &wallet),
            false
        );
    })
}
#[test]
fn test_is_valid_request_transaction_underpayment_fails() {
    run_test(|| {
        let vault = BOB;
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(vault.clone(), vec![]))));

        let address1 = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let address2 = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        let mut wallet = Wallet::new();
        wallet.add_btc_address(address2);

        let actual_value = 99;

        let request_value = 100;
        let request_address = address1;

        let transaction = build_dummy2_transaction_with(vec![(actual_value, address1)], H256::zero());
        let payment_data = OpReturnPaymentData::try_from(transaction).unwrap();

        assert_eq!(
            Relay::is_valid_request_transaction(request_value, request_address, &payment_data, &wallet),
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

        let mut wallet = Wallet::new();
        wallet.add_btc_address(vault_address);

        let transaction = build_dummy2_transaction_with(
            vec![(request_value, recipient_address), (change_value, vault_address)],
            H256::zero(),
        );
        let payment_data = OpReturnPaymentData::try_from(transaction).unwrap();

        assert_eq!(
            Relay::is_valid_request_transaction(
                request_value.try_into().unwrap(),
                recipient_address,
                &payment_data,
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

        let mut wallet = Wallet::new();
        wallet.add_btc_address(address);

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                wallet: wallet.clone(),
                ..Vault::new(BOB)
            }))
        });

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_sequence(4294967295)
                    .with_source(TransactionInputSource::FromOutput(
                        H256Le::from_bytes_le(&[
                            193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8,
                            96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                        ]),
                        1,
                    ))
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
            Relay::is_transaction_invalid(&BOB, transaction.format()),
            TestError::ValidMergeTransaction
        );
    })
}

#[test]
fn test_is_transaction_invalid_fails_with_valid_request_or_redeem() {
    run_test(|| {
        let vault_address = BtcAddress::P2SH(H160::from_slice(&[
            41, 40, 244, 58, 241, 141, 45, 96, 232, 168, 67, 84, 13, 128, 134, 179, 5, 52, 19, 57,
        ]));

        let mut wallet = Wallet::new();
        wallet.add_btc_address(vault_address);

        let recipient_address = BtcAddress::P2PKH(H160::from_str(&"5f69790b72c98041330644bbd50f2ebb5d073c36").unwrap());

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                wallet: wallet.clone(),
                ..Vault::new(BOB)
            }))
        });

        ext::redeem::get_open_or_completed_redeem_request_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: BOB,
                opentime: 0,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: ALICE,
                btc_address: recipient_address,
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: 0,
            }))
        });

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_source(TransactionInputSource::FromOutput(
                        H256Le::from_hex_le("40d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f742"),
                        1,
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
            Relay::is_transaction_invalid(&BOB, transaction.format()),
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
                new_vault: CAROL,
                collateral: 0,
                accept_time: 1,
                btc_address: recipient_address,
                btc_height: 0,
                status: ReplaceRequestStatus::Pending,
            }))
        });

        assert_err!(
            Relay::is_transaction_invalid(&BOB, transaction.format()),
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
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(BOB.clone(), vec![vault_address]))));

        let transaction = TransactionBuilder::new()
            .with_version(1)
            .add_input(
                TransactionInputBuilder::new()
                    .with_sequence(4294967295)
                    .with_source(TransactionInputSource::FromOutput(
                        H256Le::from_bytes_le(&[
                            193, 80, 65, 160, 109, 235, 107, 56, 24, 176, 34, 250, 197, 88, 218, 76, 226, 9, 127, 8,
                            96, 200, 246, 66, 16, 91, 186, 217, 210, 155, 224, 42,
                        ]),
                        1,
                    ))
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

        assert_ok!(Relay::is_transaction_invalid(&BOB, transaction.format()));
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
        let vault_btc_address_0 = BtcAddress::P2SH(H160::from_slice(
            &hex::decode("068a6a2ec6be7d6e7aac1657445154c52db0cef8").unwrap(),
        ));

        // 2NGRwGkzypA4fEz9m4KhA2ZBs7fTg3B7Zjo
        let vault_btc_address_1 = BtcAddress::P2SH(H160::from_slice(
            &hex::decode("fe5183ccb89d98beaa6908c7cf1bd109029482cf").unwrap(),
        ));

        // mzY9pX6NA3cBmiC4umbBzf1NdwrmjS7MS8
        let vault_btc_address_2 = BtcAddress::P2PKH(H160::from_slice(
            &hex::decode("d0a46d39dafa3012c2a7ed4d82d644b428e4586b").unwrap(),
        ));

        let mut wallet = Wallet::new();
        wallet.add_btc_address(vault_btc_address_0);
        wallet.add_btc_address(vault_btc_address_1);
        wallet.add_btc_address(vault_btc_address_2);

        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(move |_| {
            MockResult::Return(Ok(Vault {
                wallet: wallet.clone(),
                ..Vault::new(BOB)
            }))
        });

        assert_err!(
            Relay::is_transaction_invalid(&BOB, raw_tx),
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
        let btc_address = BtcAddress::P2SH(H160::from_slice(
            &hex::decode("068a6a2ec6be7d6e7aac1657445154c52db0cef8").unwrap(),
        ));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(BOB.clone(), vec![btc_address]))));

        assert_ok!(Relay::is_transaction_invalid(&BOB, raw_tx));
    })
}

#[test]
fn should_not_report_double_payment_with_identical_tx() {
    run_test(|| {
        assert_err!(
            Relay::report_vault_double_payment(
                Origin::signed(ALICE),
                CAROL,
                (vec![0u8; 32], vec![0u8; 32]),
                (vec![0u8; 32], vec![0u8; 32]),
            ),
            TestError::DuplicateTransaction
        );
    })
}

#[test]
fn should_report_double_payment() {
    run_test(|| {
        let public_key = dummy_public_key();
        let input_address = BtcAddress::P2PKH(public_key.to_hash());
        let output_address = BtcAddress::P2PKH(H160::random());
        let left_tx = build_dummy_transaction_from_input_with_output_and_op_return(
            H256Le::from_bytes_le(&vec![1u8; 32]),
            &public_key,
            output_address,
            &[1; 32],
        );
        let right_tx = build_dummy_transaction_from_input_with_output_and_op_return(
            H256Le::from_bytes_le(&vec![2u8; 32]),
            &public_key,
            output_address,
            &[1; 32],
        );

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(CAROL.clone(), vec![input_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_ok!(Relay::report_vault_double_payment(
            Origin::signed(ALICE),
            CAROL,
            (vec![0u8; 32], vec![1u8; 32]),
            (left_tx.format(), right_tx.format()),
        ));
    })
}

#[test]
fn should_not_report_double_payment_with_vault_no_input() {
    run_test(|| {
        let public_key = dummy_public_key();
        let input_address = BtcAddress::P2PKH(H160::random());
        let output_address = BtcAddress::P2PKH(H160::random());
        let left_tx = build_dummy_transaction_from_input_with_output_and_op_return(
            H256Le::from_bytes_le(&vec![1u8; 32]),
            &public_key,
            output_address,
            &[1; 32],
        );
        let right_tx = build_dummy_transaction_from_input_with_output_and_op_return(
            H256Le::from_bytes_le(&vec![2u8; 32]),
            &public_key,
            output_address,
            &[1; 32],
        );

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(init_zero_vault(CAROL.clone(), vec![input_address]))));
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::verify_transaction_inclusion::<Test>.mock_safe(move |_, _| MockResult::Return(Ok(())));
        ext::vault_registry::liquidate_theft_vault::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_err!(
            Relay::report_vault_double_payment(
                Origin::signed(ALICE),
                CAROL,
                (vec![0u8; 32], vec![1u8; 32]),
                (left_tx.format(), right_tx.format()),
            ),
            TestError::VaultNoInputToTransaction
        );
    })
}
