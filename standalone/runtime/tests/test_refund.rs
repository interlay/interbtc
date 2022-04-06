mod mock;

use currency::Amount;
use mock::{assert_eq, *};
use refund::types::RefundRequestExt;

fn test_with<R>(execute: impl Fn(VaultId) -> R) {
    let test_with = |collateral_currency, wrapped_currency| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies() {
                assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            }
            if wrapped_currency != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_currency, FixedU128::one()));
            }
            let vault_id = VaultId::new(account_of(BOB), collateral_currency, wrapped_currency);
            CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));
            execute(vault_id)
        });
    };
    test_with(Token(KSM), Token(IBTC));
    test_with(Token(DOT), Token(IBTC));
    test_with(Token(DOT), Token(KBTC));
}

mod spec_based_tests {
    use super::{assert_eq, *};

    #[test]
    fn execute_refund_should_fail_when_parachain_has_shutdown() {
        // PRECONDITION: The parachain status MUST NOT be shutdown
        test_with(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Refund(RefundCall::execute_refund {
                    refund_id: H256::zero(),
                    merkle_proof: vec![0u8; 32],
                    raw_tx: vec![0u8; 32],
                })
                .dispatch(origin_of(account_of(BOB))),
                SystemError::CallFiltered,
            );
        });
    }

    #[test]
    fn execute_refund_should_fail_when_no_request_exists() {
        test_with(|_currency_id| {
            // PRECONDITION: A pending refund MUST exist
            assert_noop!(
                Call::Refund(RefundCall::execute_refund {
                    refund_id: H256::zero(),
                    merkle_proof: vec![0u8; 32],
                    raw_tx: vec![0u8; 32],
                })
                .dispatch(origin_of(account_of(BOB))),
                RefundError::RefundIdNotFound,
            );
        });
    }

    #[test]
    fn execute_refund_should_succeed() {
        test_with(|vault_id| {
            let pre_refund_state = ParachainState::get(&vault_id);

            let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

            let refund_amount = vault_id.wrapped(10000);
            let refund_id = RefundPallet::request_refund(
                &refund_amount,
                vault_id.clone(),
                account_of(ALICE),
                user_btc_address,
                Default::default(),
            )
            .unwrap()
            .unwrap();

            let refund_request = RefundPallet::refund_requests(refund_id).unwrap();
            let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = generate_transaction_and_mine(
                Default::default(),
                vec![],
                vec![(user_btc_address, refund_request.amount_btc())],
                vec![refund_id],
            );
            SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

            let refund_fee = vault_id.wrapped(refund_request.fee);
            let total_supply = vault_id.wrapped(<orml_tokens::Pallet<Runtime>>::total_issuance(
                vault_id.wrapped_currency(),
            ));

            assert_ok!(Call::Refund(RefundCall::execute_refund {
                refund_id: refund_id,
                merkle_proof: merkle_proof.clone(),
                raw_tx: raw_tx.clone()
            })
            .dispatch(origin_of(vault_id.account_id.clone())));

            let refund_request = RefundPallet::refund_requests(refund_id).unwrap();

            // POSTCONDITION: refund.completed MUST be true
            assert!(refund_request.completed);

            // PRECONDITION: refund.completed MUST be false
            assert_noop!(
                Call::Refund(RefundCall::execute_refund {
                    refund_id: refund_id,
                    merkle_proof: merkle_proof,
                    raw_tx: raw_tx
                })
                .dispatch(origin_of(vault_id.account_id.clone())),
                RefundError::RefundCompleted,
            );

            // POSTCONDITION: total supply MUST increase by fee
            assert_eq!(
                total_supply + refund_fee,
                vault_id.wrapped(<orml_tokens::Pallet<Runtime>>::total_issuance(
                    vault_id.wrapped_currency()
                ))
            );

            assert_eq!(
                ParachainState::get(&vault_id),
                pre_refund_state.with_changes(|_, vault, _, _| {
                    // POSTCONDITION: vault.issued_tokens MUST increase by fee
                    vault.issued += refund_fee;
                    // POSTCONDITION: vault.free_balance MUST increase by fee
                    *vault.free_balance.get_mut(&vault_id.wrapped_currency()).unwrap() += refund_fee;
                })
            );
        });
    }

    #[test]
    fn execute_refund_invalid_payment_should_fail() {
        test_with(|vault_id| {
            let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

            let raw_refund_amount = 10000;
            let refund_amount = vault_id.wrapped(raw_refund_amount);
            let refund_id = RefundPallet::request_refund(
                &refund_amount,
                vault_id,
                account_of(ALICE),
                user_btc_address,
                Default::default(),
            )
            .unwrap()
            .unwrap();

            fn refund_invalid_amount(
                user_btc_address: BtcAddress,
                refund_id: H256,
                invalid_refund_amount: Amount<Runtime>,
            ) -> DispatchResultWithPostInfo {
                let (_tx_id, _tx_block_height, merkle_proof, raw_tx, _) = generate_transaction_and_mine(
                    Default::default(),
                    vec![],
                    vec![(user_btc_address, invalid_refund_amount)],
                    vec![refund_id],
                );
                SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + 1 + CONFIRMATIONS);

                Call::Refund(RefundCall::execute_refund {
                    refund_id: refund_id,
                    merkle_proof: merkle_proof.clone(),
                    raw_tx: raw_tx.clone(),
                })
                .dispatch(origin_of(account_of(BOB)))
            }
            let underpayment_refund_amount = wrapped(raw_refund_amount - 1);
            assert_err!(
                refund_invalid_amount(user_btc_address, refund_id, underpayment_refund_amount),
                BTCRelayError::InvalidPaymentAmount
            );

            let overpayment_refund_amount = wrapped(raw_refund_amount + 1);
            assert_err!(
                refund_invalid_amount(user_btc_address, refund_id, overpayment_refund_amount),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}
