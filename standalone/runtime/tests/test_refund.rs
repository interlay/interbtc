mod mock;

use frame_support::traits::Currency;
use mock::{assert_eq, *};

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies() {
                assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            }
            CoreVaultData::force_to(BOB, default_vault_state(currency_id));
            execute(currency_id)
        });
    };
    test_with(CurrencyId::KSM);
    test_with(CurrencyId::DOT);
}

mod spec_based_tests {
    use super::{assert_eq, *};

    #[test]
    fn execute_refund_should_fail_when_parachain_has_shutdown() {
        // PRECONDITION: The parachain status MUST NOT be shutdown
        test_with(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Refund(RefundCall::execute_refund(H256::zero(), vec![0u8; 32], vec![0u8; 32],))
                    .dispatch(origin_of(account_of(BOB))),
                SecurityError::ParachainShutdown,
            );
        });
    }

    #[test]
    fn execute_refund_should_fail_when_no_request_exists() {
        test_with(|_currency_id| {
            // PRECONDITION: A pending refund MUST exist
            assert_noop!(
                Call::Refund(RefundCall::execute_refund(H256::zero(), vec![0u8; 32], vec![0u8; 32],))
                    .dispatch(origin_of(account_of(BOB))),
                RefundError::RefundIdNotFound,
            );
        });
    }

    #[test]
    fn execute_refund_should_succeed() {
        test_with(|currency_id| {
            let pre_refund_state = ParachainState::get(currency_id);
            let vault_id = vault_id_of(BOB, currency_id);

            let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

            let refund_amount = wrapped(100);
            let refund_id = RefundPallet::request_refund(
                &refund_amount,
                vault_id.clone(),
                account_of(ALICE),
                user_btc_address,
                Default::default(),
            )
            .unwrap()
            .unwrap();

            let (_tx_id, _tx_block_height, merkle_proof, raw_tx) =
                generate_transaction_and_mine(user_btc_address, refund_amount, Some(refund_id));
            SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

            assert_ok!(Call::Refund(RefundCall::execute_refund(
                refund_id,
                merkle_proof.clone(),
                raw_tx.clone()
            ))
            .dispatch(origin_of(vault_id.account_id.clone())));

            let refund_request = RefundPallet::refund_requests(refund_id).unwrap();
            let refund_fee = wrapped(refund_request.fee);
            let total_supply = wrapped(TreasuryPallet::total_issuance());

            // POSTCONDITION: refund.completed MUST be true
            assert!(refund_request.completed);

            // PRECONDITION: refund.completed MUST be false
            assert_noop!(
                Call::Refund(RefundCall::execute_refund(refund_id, merkle_proof, raw_tx))
                    .dispatch(origin_of(vault_id.account_id.clone())),
                RefundError::RefundCompleted,
            );

            // POSTCONDITION: total supply MUST increase by fee
            assert_eq!(total_supply + refund_fee, wrapped(TreasuryPallet::total_issuance()));

            assert_eq!(
                ParachainState::get(currency_id),
                pre_refund_state.with_changes(|_, vault, _, _| {
                    // POSTCONDITION: vault.issued_tokens MUST increase by fee
                    vault.issued += refund_fee;
                    // POSTCONDITION: vault.free_balance MUST increase by fee
                    *vault.free_balance.get_mut(&INTERBTC).unwrap() += refund_fee;
                })
            );
        });
    }
}
