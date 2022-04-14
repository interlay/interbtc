mod mock;

use crate::redeem_testing_utils::{setup_redeem, USER};
use currency::Amount;
use mock::{assert_eq, replace_testing_utils::*, *};
use refund::types::RefundRequestExt;
use sp_core::H256;

pub const RELAYER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            UserData::force_to(USER, default_user_state());
            execute(currency_id)
        })
    };
    test_with(Token(DOT));
    test_with(Token(KSM));
}

fn setup_vault_for_potential_double_spend(
    issued_tokens: Amount<Runtime>,
    stealing_vault: [u8; 32],
    issue_tokens: bool,
) -> (BtcPublicKey, BtcPublicKey) {
    let vault_public_key_one = BtcPublicKey([
        2, 168, 49, 109, 0, 14, 227, 106, 112, 84, 59, 37, 153, 238, 121, 44, 66, 8, 181, 64, 248, 19, 137, 27, 47,
        222, 50, 95, 187, 221, 152, 165, 69,
    ]);

    let vault_public_key_two = BtcPublicKey([
        2, 139, 220, 235, 13, 249, 164, 152, 179, 4, 175, 217, 170, 84, 218, 179, 182, 247, 109, 48, 57, 152, 241, 165,
        225, 26, 242, 187, 160, 225, 248, 195, 250,
    ]);

    let vault_id = VaultId::new(
        account_of(stealing_vault),
        DEFAULT_COLLATERAL_CURRENCY,
        DEFAULT_WRAPPED_CURRENCY,
    );
    register_vault_with_public_key(
        &vault_id,
        Amount::new(INITIAL_BALANCE, vault_id.collateral_currency()),
        vault_public_key_one.clone(),
    );

    if issue_tokens {
        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &default_vault_id_of(stealing_vault),
            &issued_tokens,
        ));
        assert_ok!(VaultRegistryPallet::issue_tokens(
            &default_vault_id_of(stealing_vault),
            &issued_tokens
        ));
    }
    (vault_public_key_one, vault_public_key_two)
}

#[test]
fn integration_test_report_vault_theft() {
    test_with(|currency_id| {
        let user = ALICE;
        let vault = BOB;
        let theft_amount = wrapped(100);
        let collateral_vault = Amount::new(1000000, currency_id);
        let issued_tokens = wrapped(100);
        let vault_id = vault_id_of(vault, currency_id);

        let vault_btc_address = BtcAddress::P2SH(H160([
            215, 255, 109, 96, 235, 244, 10, 155, 24, 134, 172, 206, 6, 101, 59, 162, 34, 77, 143, 234,
        ]));
        let other_btc_address = BtcAddress::P2SH(H160([1; 20]));

        SecurityPallet::set_active_block_number(1);

        register_vault(&vault_id, collateral_vault);

        assert_ok!(VaultRegistryPallet::insert_vault_deposit_address(
            vault_id.clone(),
            vault_btc_address
        ));

        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &vault_id,
            &issued_tokens,
        ));
        assert_ok!(VaultRegistryPallet::issue_tokens(&vault_id, &issued_tokens));

        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_outputs(vec![(other_btc_address, theft_amount)])
            .with_confirmations(7)
            .with_relayer(Some(ALICE))
            .mine();

        SecurityPallet::set_active_block_number(1000);

        let pre_liquidation_state = ParachainState::get(&vault_id);
        let theft_fee = FeePallet::get_theft_fee(&collateral_vault).unwrap();

        assert_ok!(Call::Relay(RelayCall::report_vault_theft {
            vault_id: vault_id.clone(),
            raw_merkle_proof: proof,
            raw_tx: raw_tx
        })
        .dispatch(origin_of(account_of(user))));

        let confiscated_collateral = Amount::new(150, currency_id);
        assert_eq!(
            ParachainState::get(&vault_id),
            pre_liquidation_state.with_changes(|user, vault, liquidation_vault, _fee_pool| {
                let liquidation_vault = liquidation_vault.with_currency(&vault_id.currencies);

                (*user.balances.get_mut(&currency_id).unwrap()).free += theft_fee;

                vault.issued -= issued_tokens;
                vault.backing_collateral -= confiscated_collateral;
                vault.backing_collateral -= theft_fee;

                liquidation_vault.issued += issued_tokens;
                liquidation_vault.collateral += confiscated_collateral;
            })
        );
    });
}

#[test]
fn integration_test_double_spend_redeem() {
    test_with(|_currency_id| {
        let issued_tokens = wrapped(10_000);
        // Register vault with hardcoded public key so it counts as theft
        let stealing_vault = DAVE;
        let (vault_public_key_one, vault_public_key_two) =
            setup_vault_for_potential_double_spend(issued_tokens, stealing_vault, true);

        let redeem_id = setup_redeem(issued_tokens, USER, &default_vault_id_of(stealing_vault));
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));
        let current_block_number = 1;

        // Send the honest redeem transaction
        let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = {
            register_addresses_and_mine_transaction(
                default_vault_id_of(stealing_vault),
                vault_public_key_one,
                vec![],
                vec![(user_btc_address, redeem.amount_btc())],
                vec![redeem_id],
            )
        };

        // Double-spend the redeem, so the redeemer gets twice the BTC
        let (_theft_tx_id, _theft_tx_block_height, theft_merkle_proof, theft_raw_tx, _) =
            register_addresses_and_mine_transaction(
                default_vault_id_of(stealing_vault),
                vault_public_key_two,
                vec![],
                vec![(user_btc_address, redeem.amount_btc())],
                vec![redeem_id],
            );
        SecurityPallet::set_active_block_number(current_block_number + 1 + CONFIRMATIONS);

        assert_ok!(Call::Redeem(RedeemCall::execute_redeem {
            redeem_id: redeem_id,
            merkle_proof: merkle_proof.clone(),
            raw_tx: raw_tx.clone()
        })
        .dispatch(origin_of(account_of(stealing_vault))));

        // Executing the theft tx should fail
        assert_err!(
            Call::Redeem(RedeemCall::execute_redeem {
                redeem_id: redeem_id,
                merkle_proof: theft_merkle_proof.clone(),
                raw_tx: theft_raw_tx.clone()
            })
            .dispatch(origin_of(account_of(stealing_vault))),
            RedeemError::RedeemCompleted
        );

        // Reporting the double-spend transaction as theft should work
        assert_ok!(Call::Relay(RelayCall::report_vault_double_payment {
            vault_id: default_vault_id_of(stealing_vault),
            raw_merkle_proofs: (merkle_proof, theft_merkle_proof),
            raw_txs: (raw_tx, theft_raw_tx),
        })
        .dispatch(origin_of(account_of(USER))));
    });
}

fn redeem_with_extra_utxo(use_unregistered_btc_address: bool) -> DispatchResultWithPostInfo {
    let issued_tokens = wrapped(10_000);
    let vault = DAVE;
    let (vault_public_key_one, vault_public_key_two) =
        setup_vault_for_potential_double_spend(issued_tokens, vault, true);
    let second_vault_btc_address = if use_unregistered_btc_address {
        BtcAddress::P2PKH(H160([8; 20]))
    } else {
        let btc_address = BtcAddress::P2PKH(vault_public_key_two.to_hash());
        assert_ok!(VaultRegistryPallet::insert_vault_deposit_address(
            default_vault_id_of(vault),
            btc_address
        ));
        btc_address
    };

    let redeem_id = setup_redeem(issued_tokens, USER, &default_vault_id_of(vault));
    let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
    let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));
    let current_block_number = 1;

    let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = {
        register_addresses_and_mine_transaction(
            default_vault_id_of(vault),
            vault_public_key_one,
            vec![],
            vec![
                (user_btc_address, redeem.amount_btc()),
                (second_vault_btc_address, redeem.amount_btc()),
            ],
            vec![redeem_id],
        )
    };

    SecurityPallet::set_active_block_number(current_block_number + 1 + CONFIRMATIONS);

    assert_ok!(Call::Redeem(RedeemCall::execute_redeem {
        redeem_id: redeem_id,
        merkle_proof: merkle_proof.clone(),
        raw_tx: raw_tx.clone()
    })
    .dispatch(origin_of(account_of(vault))));

    Call::Relay(RelayCall::report_vault_theft {
        vault_id: default_vault_id_of(vault),
        raw_merkle_proof: merkle_proof,
        raw_tx: raw_tx,
    })
    .dispatch(origin_of(account_of(USER)))
}

#[test]
fn integration_test_redeem_valid_change_utxo() {
    test_with(|_currency_id| {
        // Reporting as theft should fail, because the additional UTXO was a change (leftover) tx
        assert_err!(redeem_with_extra_utxo(false), RelayError::ValidRedeemTransaction);
    });
}

#[test]
fn integration_test_redeem_utxo_to_foreign_address() {
    test_with(|_currency_id| {
        // Reporting as theft should work, because the additional UTXO was sent to an
        // address that wasn't the redeemer not the vault's
        assert_ok!(redeem_with_extra_utxo(true));
    });
}

#[test]
fn integration_test_merge_tx() {
    test_with(|_currency_id| {
        let vault = BOB;
        let transfer_amount_raw = 100;
        let transfer_amount = wrapped(transfer_amount_raw);

        let (vault_public_key_one, vault_public_key_two) =
            setup_vault_for_potential_double_spend(transfer_amount, vault, false);

        let vault_public_key_three = BtcPublicKey([1u8; 33]);
        let vault_public_key_four = BtcPublicKey([2u8; 33]);

        let vault_first_address = BtcAddress::P2PKH(vault_public_key_one.to_hash());
        let vault_second_address = BtcAddress::P2PKH(vault_public_key_two.to_hash());
        let vault_third_address = BtcAddress::P2PKH(vault_public_key_three.to_hash());
        let vault_fourth_address = BtcAddress::P2PKH(vault_public_key_four.to_hash());

        // The first public key isn't added to the wallet automatically as a P2PKH address.
        // Need to explicitly add it, otherwise the tx won't be considered a "merge"
        register_vault_address(default_vault_id_of(vault), vault_public_key_one.clone());
        register_vault_address(default_vault_id_of(vault), vault_public_key_two.clone());
        register_vault_address(default_vault_id_of(vault), vault_public_key_three.clone());
        register_vault_address(default_vault_id_of(vault), vault_public_key_four.clone());

        let (_, _, _, _, tx) = generate_transaction_and_mine(
            vault_public_key_one.clone(),
            vec![],
            vec![
                (vault_first_address, transfer_amount),
                (vault_second_address, transfer_amount),
                (vault_third_address, transfer_amount),
                (vault_fourth_address, transfer_amount),
            ],
            vec![],
        );

        let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = generate_transaction_and_mine(
            vault_public_key_one.clone(),
            vec![
                (tx.clone(), 0, Some(vault_public_key_one)),
                (tx.clone(), 1, Some(vault_public_key_two)),
                (tx.clone(), 2, Some(vault_public_key_three)),
                (tx, 3, Some(vault_public_key_four)),
            ],
            vec![(vault_first_address, wrapped(4 * transfer_amount_raw))],
            vec![],
        );

        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

        // Reporting as theft should fail, because the transaction merged
        // UTXOs sent to registered vault addresses
        assert_err!(
            Call::Relay(RelayCall::report_vault_theft {
                vault_id: default_vault_id_of(vault),
                raw_merkle_proof: merkle_proof,
                raw_tx: raw_tx
            })
            .dispatch(origin_of(account_of(USER))),
            RelayError::ValidMergeTransaction
        );
    });
}

#[test]
fn integration_test_double_spend_refund() {
    test_with(|_currency_id| {
        let issued_tokens = wrapped(10_000);
        let stealing_vault = DAVE;
        let (vault_public_key_one, vault_public_key_two) =
            setup_vault_for_potential_double_spend(issued_tokens, stealing_vault, true);

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));
        let refund_amount = wrapped(10_000);
        let refund_id = RefundPallet::request_refund(
            &refund_amount,
            default_vault_id_of(stealing_vault),
            account_of(ALICE),
            user_btc_address,
            Default::default(),
        )
        .unwrap()
        .unwrap();
        let refund_request = RefundPallet::refund_requests(refund_id).unwrap();

        let current_block_number = 1;

        // Send the honest refund transaction
        let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = {
            register_addresses_and_mine_transaction(
                default_vault_id_of(stealing_vault),
                vault_public_key_one,
                vec![],
                vec![(user_btc_address, refund_request.amount_btc())],
                vec![refund_id],
            )
        };

        // Double-spend the refund, so the payee gets twice the BTC
        let (_theft_tx_id, _theft_tx_block_height, theft_merkle_proof, theft_raw_tx, _) =
            register_addresses_and_mine_transaction(
                default_vault_id_of(stealing_vault),
                vault_public_key_two,
                vec![],
                vec![(user_btc_address, refund_request.amount_btc())],
                vec![refund_id],
            );
        SecurityPallet::set_active_block_number(current_block_number + 1 + CONFIRMATIONS);

        assert_ok!(Call::Refund(RefundCall::execute_refund {
            refund_id: refund_id,
            merkle_proof: merkle_proof.clone(),
            raw_tx: raw_tx.clone()
        })
        .dispatch(origin_of(account_of(stealing_vault))));

        // Executing the theft tx should fail
        assert_err!(
            Call::Refund(RefundCall::execute_refund {
                refund_id: refund_id,
                merkle_proof: theft_merkle_proof.clone(),
                raw_tx: theft_raw_tx.clone()
            })
            .dispatch(origin_of(account_of(stealing_vault))),
            RefundError::RefundCompleted
        );

        // Reporting the double-spend transaction as theft should work
        assert_ok!(Call::Relay(RelayCall::report_vault_double_payment {
            vault_id: default_vault_id_of(stealing_vault),
            raw_merkle_proofs: (merkle_proof, theft_merkle_proof),
            raw_txs: (raw_tx, theft_raw_tx),
        })
        .dispatch(origin_of(account_of(USER))));
    });
}

#[test]
fn integration_test_double_spend_replace() {
    test_with(|_currency_id| {
        let issued_tokens = wrapped(1000);
        let stealing_vault = BOB;
        let stealing_vault_id = default_vault_id_of(stealing_vault);
        let new_vault = CAROL;
        let new_vault_id = default_vault_id_of(new_vault);
        let replace_amount = wrapped(100);

        let (vault_public_key_one, vault_public_key_two) =
            setup_vault_for_potential_double_spend(issued_tokens, stealing_vault, false);

        CoreVaultData::force_to(&stealing_vault_id, default_vault_state(&stealing_vault_id));
        CoreVaultData::force_to(&new_vault_id, default_vault_state(&new_vault_id));

        request_replace(&stealing_vault_id, issued_tokens);
        let (replace, replace_id) = setup_replace(&stealing_vault_id, &new_vault_id, replace_amount);
        let current_block_number = 1;

        // Send the honest replace transaction
        let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = {
            register_addresses_and_mine_transaction(
                stealing_vault_id.clone(),
                vault_public_key_one,
                vec![],
                vec![(replace.btc_address, replace_amount)],
                vec![replace_id],
            )
        };

        // Double-spend the replace, so the payee gets twice the BTC
        let (_theft_tx_id, _theft_tx_block_height, theft_merkle_proof, theft_raw_tx, _) =
            register_addresses_and_mine_transaction(
                stealing_vault_id.clone(),
                vault_public_key_two,
                vec![],
                vec![(replace.btc_address, replace_amount)],
                vec![replace_id],
            );

        SecurityPallet::set_active_block_number(current_block_number + 1 + CONFIRMATIONS);
        assert_ok!(Call::Replace(ReplaceCall::execute_replace {
            replace_id: replace_id,
            merkle_proof: merkle_proof.clone(),
            raw_tx: raw_tx.clone()
        })
        .dispatch(origin_of(account_of(stealing_vault))));

        // Executing the theft tx should fail
        assert_err!(
            Call::Replace(ReplaceCall::execute_replace {
                replace_id: replace_id,
                merkle_proof: theft_merkle_proof.clone(),
                raw_tx: theft_raw_tx.clone()
            })
            .dispatch(origin_of(account_of(stealing_vault))),
            ReplaceError::ReplaceCompleted
        );

        // Reporting the double-spend transaction as theft should work
        assert_ok!(Call::Relay(RelayCall::report_vault_double_payment {
            vault_id: stealing_vault_id,
            raw_merkle_proofs: (merkle_proof, theft_merkle_proof),
            raw_txs: (raw_tx, theft_raw_tx),
        })
        .dispatch(origin_of(account_of(USER))));
    });
}

#[test]
fn integration_test_relay_parachain_status_check_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Relay(RelayCall::initialize {
                raw_block_header: Default::default(),
                block_height: 0
            })
            .dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::Relay(RelayCall::store_block_header {
                raw_block_header: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered
        );
        assert_noop!(
            Call::Relay(RelayCall::report_vault_theft {
                vault_id: vault_id_of(ALICE, Token(DOT)),
                raw_merkle_proof: Default::default(),
                raw_tx: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered
        );
    })
}
