use crate::{ext, mock::*};

use crate::types::{RedeemRequest, RedeemRequestStatus};
use bitcoin::types::{MerkleProof, Transaction};
use btc_relay::BtcAddress;
use currency::Amount;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use mocktopus::mocking::*;
use security::Pallet as Security;
use sp_core::{H160, H256};
use vault_registry::{DefaultVault, VaultStatus, Wallet};

type Event = crate::Event<Test>;

fn collateral(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_COLLATERAL_CURRENCY)
}
fn griefing(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_NATIVE_CURRENCY)
}
fn wrapped(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_WRAPPED_CURRENCY)
}

macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::Redeem($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::Redeem($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

fn dummy_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: Default::default(),
        transactions_count: 0,
        flag_bits: vec![],
        hashes: vec![],
    }
}

fn inject_redeem_request(key: H256, value: RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>) {
    Redeem::insert_redeem_request(&key, &value)
}

fn default_vault() -> DefaultVault<Test> {
    vault_registry::Vault {
        id: VAULT,
        to_be_replaced_tokens: 0,
        to_be_issued_tokens: 0,
        issued_tokens: 10,
        replace_collateral: 0,
        to_be_redeemed_tokens: 0,
        active_replace_collateral: 0,
        wallet: Wallet::new(),
        banned_until: None,
        status: VaultStatus::Active(true),
        liquidated_collateral: 0,
    }
}

#[test]
fn test_request_redeem_fails_with_amount_exceeds_user_balance() {
    run_test(|| {
        let amount = Amount::<Test>::new(2, <Test as currency::Config>::GetWrappedCurrencyId::get());
        amount.mint_to(&USER).unwrap();
        let amount = 10_000_000;
        assert_err!(
            Redeem::request_redeem(Origin::signed(USER), amount, BtcAddress::default(), VAULT),
            TestError::AmountExceedsUserBalance
        );
    })
}

#[test]
fn test_request_redeem_fails_with_amount_below_minimum() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                replace_collateral: 0,
                to_be_redeemed_tokens: 0,
                active_replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: None,
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );

        let redeemer = USER;
        let amount = 9;

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(move |vault_id, amount_btc| {
            assert_eq!(vault_id, &VAULT);
            assert_eq!(amount_btc, &wrapped(amount));

            MockResult::Return(Ok(()))
        });

        assert_err!(
            Redeem::request_redeem(Origin::signed(redeemer), 1, BtcAddress::random(), VAULT),
            TestError::AmountBelowDustAmount
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_not_found() {
    run_test(|| {
        assert_err!(
            Redeem::request_redeem(Origin::signed(USER), 1500, BtcAddress::default(), VAULT),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_banned() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>
            .mock_safe(|_| MockResult::Return(Err(VaultRegistryError::VaultBanned.into())));

        assert_err!(
            Redeem::request_redeem(Origin::signed(USER), 1500, BtcAddress::default(), VAULT),
            VaultRegistryError::VaultBanned
        );
    })
}

#[test]
fn test_request_redeem_fails_with_vault_liquidated() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_| MockResult::Return(Ok(())));
        assert_err!(
            Redeem::request_redeem(Origin::signed(USER), 3000, BtcAddress::random(), VAULT),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_redeem_succeeds_with_normal_redeem() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                replace_collateral: 0,
                active_replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: None,
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );

        let redeemer = USER;
        let amount = 90;
        let redeem_fee = 5;

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(move |vault_id, amount_btc| {
            assert_eq!(vault_id, &VAULT);
            assert_eq!(amount_btc, &wrapped(amount - redeem_fee));

            MockResult::Return(Ok(()))
        });

        Amount::<Test>::lock_on.mock_safe(move |amount_wrapped, account| {
            assert_eq!(account, &redeemer);
            assert_eq!(amount_wrapped, &wrapped(amount));

            MockResult::Return(Ok(()))
        });

        ext::security::get_secure_id::<Test>.mock_safe(move |_| MockResult::Return(H256([0; 32])));
        ext::vault_registry::is_vault_below_premium_threshold::<Test>.mock_safe(move |_| MockResult::Return(Ok(false)));
        ext::fee::get_redeem_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(wrapped(redeem_fee))));
        let btc_fee = Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY).unwrap();

        assert_ok!(Redeem::request_redeem(
            Origin::signed(redeemer),
            amount,
            BtcAddress::P2PKH(H160::zero()),
            VAULT
        ));

        assert_emitted!(Event::RequestRedeem {
            redeem_id: H256([0; 32]),
            redeemer,
            amount: amount - redeem_fee - btc_fee.amount(),
            fee: redeem_fee,
            premium: 0,
            vault_id: VAULT,
            btc_address: BtcAddress::P2PKH(H160::zero()),
            transfer_fee: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                .unwrap()
                .amount()
        });
        assert_ok!(
            Redeem::get_open_redeem_request_from_id(&H256([0; 32])),
            RedeemRequest {
                period: Redeem::redeem_period(),
                vault: VAULT,
                opentime: 1,
                fee: redeem_fee,
                amount_btc: amount - redeem_fee - btc_fee.amount(),
                premium: 0,
                redeemer,
                btc_address: BtcAddress::P2PKH(H160::zero()),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            }
        );
    })
}

#[test]
fn test_request_redeem_succeeds_with_self_redeem() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                active_replace_collateral: 0,
                replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: None,
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );

        let redeemer = VAULT.account_id;
        let amount = 90;

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>.mock_safe(move |vault_id, amount_btc| {
            assert_eq!(vault_id, &VAULT);
            assert_eq!(amount_btc, &wrapped(amount));

            MockResult::Return(Ok(()))
        });

        Amount::<Test>::lock_on.mock_safe(move |amount_wrapped, account| {
            assert_eq!(account, &redeemer);
            assert_eq!(amount_wrapped, &wrapped(amount));

            MockResult::Return(Ok(()))
        });

        ext::security::get_secure_id::<Test>.mock_safe(move |_| MockResult::Return(H256::zero()));
        ext::vault_registry::is_vault_below_premium_threshold::<Test>.mock_safe(move |_| MockResult::Return(Ok(false)));
        let btc_fee = Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY).unwrap();

        assert_ok!(Redeem::request_redeem(
            Origin::signed(redeemer),
            amount,
            BtcAddress::P2PKH(H160::zero()),
            VAULT
        ));

        assert_emitted!(Event::RequestRedeem {
            redeem_id: H256::zero(),
            redeemer,
            amount: amount - btc_fee.amount(),
            fee: 0,
            premium: 0,
            vault_id: VAULT,
            btc_address: BtcAddress::P2PKH(H160::zero()),
            transfer_fee: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                .unwrap()
                .amount()
        });
        assert_ok!(
            Redeem::get_open_redeem_request_from_id(&H256::zero()),
            RedeemRequest {
                period: Redeem::redeem_period(),
                vault: VAULT,
                opentime: 1,
                fee: 0,
                amount_btc: amount - btc_fee.amount(),
                premium: 0,
                redeemer,
                btc_address: BtcAddress::P2PKH(H160::zero()),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            }
        );
    })
}

#[test]
fn test_liquidation_redeem_succeeds() {
    run_test(|| {
        let total_amount = 10 * 100_000_000;

        ext::treasury::get_balance::<Test>.mock_safe(move |_, _| MockResult::Return(wrapped(total_amount)));

        Amount::<Test>::lock_on.mock_safe(move |_, _| MockResult::Return(Ok(())));
        Amount::<Test>::burn_from.mock_safe(move |amount, redeemer_id| {
            assert_eq!(redeemer_id, &USER);
            assert_eq!(amount, &wrapped(total_amount));

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens_liquidation::<Test>.mock_safe(move |_, redeemer_id, amount| {
            assert_eq!(redeemer_id, &USER);
            assert_eq!(amount, &wrapped(total_amount));

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::liquidation_redeem(
            Origin::signed(USER),
            DEFAULT_CURRENCY_PAIR,
            total_amount,
        ));
    })
}

#[test]
fn test_execute_redeem_fails_with_redeem_id_not_found() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        assert_err!(
            Redeem::execute_redeem(
                Origin::signed(VAULT.account_id),
                H256([0u8; 32]),
                Vec::default(),
                Vec::default()
            ),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_execute_redeem_succeeds_with_another_account() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                replace_collateral: 0,
                active_replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: None,
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

        let btc_fee = Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY).unwrap();

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: btc_fee.amount(),
            },
        );

        Amount::<Test>::burn_from.mock_safe(move |amount_wrapped, redeemer| {
            assert_eq!(redeemer, &USER);
            assert_eq!(amount_wrapped, &(wrapped(100) + btc_fee));

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_wrapped, premium, _| {
            assert_eq!(vault, &VAULT);
            assert_eq!(amount_wrapped, &(wrapped(100) + btc_fee));
            assert_eq!(premium, &collateral(0));

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(USER),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem {
            redeem_id: H256([0; 32]),
            redeemer: USER,
            vault_id: VAULT,
            amount: 100,
            fee: 0,
            transfer_fee: btc_fee.amount(),
        });
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCompleted,
        );
    })
}

#[test]
fn test_execute_redeem_succeeds() {
    run_test(|| {
        convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
        Security::<Test>::set_active_block_number(40);
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 200,
                to_be_redeemed_tokens: 200,
                replace_collateral: 0,
                active_replace_collateral: 0,
                wallet: Wallet::new(),
                banned_until: None,
                status: VaultStatus::Active(true),
                liquidated_collateral: 0,
            },
        );
        ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
        ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
        ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
            .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

        let btc_fee = Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY).unwrap();

        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: btc_fee.amount(),
            },
        );

        Amount::<Test>::burn_from.mock_safe(move |amount_wrapped, redeemer| {
            assert_eq!(redeemer, &USER);
            assert_eq!(amount_wrapped, &(wrapped(100) + btc_fee));

            MockResult::Return(Ok(()))
        });

        ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_wrapped, premium, _| {
            assert_eq!(vault, &VAULT);
            assert_eq!(amount_wrapped, &(wrapped(100) + btc_fee));
            assert_eq!(premium, &collateral(0));

            MockResult::Return(Ok(()))
        });

        assert_ok!(Redeem::execute_redeem(
            Origin::signed(VAULT.account_id),
            H256([0u8; 32]),
            Vec::default(),
            Vec::default()
        ));
        assert_emitted!(Event::ExecuteRedeem {
            redeem_id: H256([0; 32]),
            redeemer: USER,
            vault_id: VAULT,
            amount: 100,
            fee: 0,
            transfer_fee: btc_fee.amount(),
        });
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCompleted,
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_redeem_id_not_found() {
    run_test(|| {
        assert_err!(
            Redeem::cancel_redeem(Origin::signed(USER), H256([0u8; 32]), false),
            TestError::RedeemIdNotFound
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_time_not_expired() {
    run_test(|| {
        Security::<Test>::set_active_block_number(10);

        Redeem::get_open_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 0,
                fee: 0,
                amount_btc: 0,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            }))
        });

        assert_err!(
            Redeem::cancel_redeem(Origin::signed(USER), H256([0u8; 32]), false),
            TestError::TimeNotExpired
        );
    })
}

#[test]
fn test_cancel_redeem_fails_with_unauthorized_caller() {
    run_test(|| {
        Security::<Test>::set_active_block_number(20);

        Redeem::get_open_redeem_request_from_id.mock_safe(|_| {
            MockResult::Return(Ok(RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 0,
                fee: 0,
                amount_btc: 0,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            }))
        });

        assert_noop!(
            Redeem::cancel_redeem(Origin::signed(CAROL), H256([0u8; 32]), true),
            TestError::UnauthorizedRedeemer
        );
    })
}

#[test]
fn test_cancel_redeem_succeeds() {
    run_test(|| {
        inject_redeem_request(
            H256([0u8; 32]),
            RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 10,
                fee: 0,
                amount_btc: 10,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            },
        );

        ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(true)));

        ext::vault_registry::ban_vault::<Test>.mock_safe(move |vault| {
            assert_eq!(vault, &VAULT);
            MockResult::Return(Ok(()))
        });
        Amount::<Test>::unlock_on.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::transfer_funds_saturated::<Test>
            .mock_safe(move |_, _, amount| MockResult::Return(Ok(amount.clone())));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(vault_registry::types::Vault {
                status: VaultStatus::Active(true),
                ..vault_registry::types::Vault::new(VAULT)
            }))
        });
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_ok!(Redeem::cancel_redeem(Origin::signed(USER), H256([0u8; 32]), false));
        assert_err!(
            Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
            TestError::RedeemCancelled,
        );
        assert_emitted!(Event::CancelRedeem {
            redeem_id: H256([0; 32]),
            redeemer: USER,
            vault_id: VAULT,
            slashed_amount: 1,
            status: RedeemRequestStatus::Retried
        });
    })
}

#[test]
fn test_mint_tokens_for_reimbursed_redeem() {
    // PRECONDITION: The vault MUST NOT be banned.
    // POSTCONDITION: `tryIncreaseToBeIssuedTokens` and `issueTokens` MUST be called,
    // both with the vault and `redeem.amountBtc + redeem.transferFeeBtc` as arguments.
    run_test(|| {
        let redeem_request = RedeemRequest {
            period: 0,
            vault: VAULT,
            opentime: 40,
            fee: 0,
            amount_btc: 100,
            premium: 0,
            redeemer: USER,
            btc_address: BtcAddress::random(),
            btc_height: 0,
            status: RedeemRequestStatus::Reimbursed(false),
            transfer_fee_btc: 1,
        };
        let redeem_request_clone = redeem_request.clone();
        inject_redeem_request(H256([0u8; 32]), redeem_request.clone());
        <vault_registry::Pallet<Test>>::insert_vault(
            &VAULT,
            vault_registry::Vault {
                id: VAULT,
                banned_until: Some(100),
                status: VaultStatus::Active(true),
                ..default_vault()
            },
        );
        Security::<Test>::set_active_block_number(100);
        assert_noop!(
            Redeem::mint_tokens_for_reimbursed_redeem(
                Origin::signed(VAULT.account_id),
                VAULT.currencies.clone(),
                H256([0u8; 32])
            ),
            VaultRegistryError::ExceedingVaultLimit
        );
        Security::<Test>::set_active_block_number(101);
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(move |vault_id, amount| {
            assert_eq!(vault_id, &VAULT);
            assert_eq!(
                amount,
                &wrapped(redeem_request.amount_btc + redeem_request.transfer_fee_btc)
            );
            MockResult::Return(Ok(()))
        });
        ext::vault_registry::issue_tokens::<Test>.mock_safe(move |vault_id, amount| {
            assert_eq!(vault_id, &VAULT);
            assert_eq!(
                amount,
                &wrapped(redeem_request_clone.amount_btc + redeem_request_clone.transfer_fee_btc)
            );
            MockResult::Return(Ok(()))
        });
        assert_ok!(Redeem::mint_tokens_for_reimbursed_redeem(
            Origin::signed(VAULT.account_id),
            VAULT.currencies.clone(),
            H256([0u8; 32])
        ));
    });
}

#[test]
fn test_set_redeem_period_only_root() {
    run_test(|| {
        assert_noop!(
            Redeem::set_redeem_period(Origin::signed(USER), 1),
            DispatchError::BadOrigin
        );
        assert_ok!(Redeem::set_redeem_period(Origin::root(), 1));
    })
}

mod spec_based_tests {
    use super::*;

    #[test]
    fn test_request_reduces_to_be_replaced() {
        // POSTCONDITION: `decreaseToBeReplacedTokens` MUST be called, supplying `vault` and `burnedTokens`.
        // The returned `replaceCollateral` MUST be released by this function.
        run_test(|| {
            let amount_to_redeem = 100;
            let replace_collateral = 100;
            let amount = Amount::<Test>::new(
                amount_to_redeem,
                <Test as currency::Config>::GetWrappedCurrencyId::get(),
            );
            amount.mint_to(&USER).unwrap();
            ext::vault_registry::ensure_not_banned::<Test>.mock_safe(move |_vault_id| MockResult::Return(Ok(())));
            ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
                .mock_safe(move |_vault_id, _amount| MockResult::Return(Ok(())));
            ext::vault_registry::is_vault_below_premium_threshold::<Test>
                .mock_safe(move |_vault_id| MockResult::Return(Ok(false)));
            let redeem_fee = Fee::get_redeem_fee(&wrapped(amount_to_redeem)).unwrap();
            let burned_tokens = wrapped(amount_to_redeem) - redeem_fee;

            ext::vault_registry::decrease_to_be_replaced_tokens::<Test>.mock_safe(move |vault_id, tokens| {
                assert_eq!(vault_id, &VAULT);
                assert_eq!(tokens, &burned_tokens);
                MockResult::Return(Ok((wrapped(0), griefing(0))))
            });

            // The returned `replaceCollateral` MUST be released
            currency::Amount::unlock_on.mock_safe(move |collateral_amount, vault_id| {
                assert_eq!(vault_id, &VAULT.account_id);
                assert_eq!(collateral_amount, &collateral(replace_collateral));
                MockResult::Return(Ok(()))
            });

            assert_ok!(Redeem::request_redeem(
                Origin::signed(USER),
                amount_to_redeem,
                BtcAddress::random(),
                VAULT
            ));
        })
    }

    #[test]
    fn test_liquidation_redeem_succeeds() {
        // POSTCONDITION: `redeemTokensLiquidation` MUST be called with `redeemer`
        // and `amountWrapped` as arguments.
        run_test(|| {
            let total_amount = 10 * 100_000_000;

            ext::treasury::get_balance::<Test>.mock_safe(move |_, _| MockResult::Return(wrapped(total_amount)));

            Amount::<Test>::lock_on.mock_safe(move |_, _| MockResult::Return(Ok(())));
            Amount::<Test>::burn_from.mock_safe(move |amount, redeemer_id| {
                assert_eq!(redeemer_id, &USER);
                assert_eq!(amount, &wrapped(total_amount));

                MockResult::Return(Ok(()))
            });

            ext::vault_registry::redeem_tokens_liquidation::<Test>.mock_safe(move |_, redeemer_id, amount| {
                assert_eq!(redeemer_id, &USER);
                assert_eq!(amount, &wrapped(total_amount));

                MockResult::Return(Ok(()))
            });

            assert_ok!(Redeem::liquidation_redeem(
                Origin::signed(USER),
                DEFAULT_CURRENCY_PAIR,
                total_amount.into(),
            ));
        })
    }

    #[test]
    fn test_execute_redeem_succeeds_with_another_account() {
        // POSTCONDITION: `redeemTokens` MUST be called, supplying `redeemRequest.vault`,
        // `redeemRequest.amountBtc + redeemRequest.transferFeeBtc`, `redeemRequest.premium` and
        // `redeemRequest.redeemer` as arguments.
        run_test(|| {
            convert_to.mock_safe(|_, x| MockResult::Return(Ok(x)));
            Security::<Test>::set_active_block_number(40);
            <vault_registry::Pallet<Test>>::insert_vault(
                &VAULT,
                vault_registry::Vault {
                    id: VAULT,
                    to_be_replaced_tokens: 0,
                    to_be_issued_tokens: 0,
                    issued_tokens: 200,
                    to_be_redeemed_tokens: 200,
                    replace_collateral: 0,
                    wallet: Wallet::new(),
                    banned_until: None,
                    status: VaultStatus::Active(true),
                    ..default_vault()
                },
            );
            ext::btc_relay::parse_merkle_proof::<Test>.mock_safe(|_| MockResult::Return(Ok(dummy_merkle_proof())));
            ext::btc_relay::parse_transaction::<Test>.mock_safe(|_| MockResult::Return(Ok(Transaction::default())));
            ext::btc_relay::verify_and_validate_op_return_transaction::<Test, Balance>
                .mock_safe(|_, _, _, _, _| MockResult::Return(Ok(())));

            let btc_fee = Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY).unwrap();
            let redeem_request = RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 40,
                fee: 0,
                amount_btc: 100,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: btc_fee.amount(),
            };
            inject_redeem_request(H256([0u8; 32]), redeem_request.clone());

            Amount::<Test>::burn_from.mock_safe(move |_, _| MockResult::Return(Ok(())));

            ext::vault_registry::redeem_tokens::<Test>.mock_safe(move |vault, amount_wrapped, premium, redeemer| {
                assert_eq!(vault, &redeem_request.vault);
                assert_eq!(
                    amount_wrapped,
                    &wrapped(redeem_request.amount_btc + redeem_request.transfer_fee_btc)
                );
                assert_eq!(premium, &collateral(redeem_request.premium));
                assert_eq!(redeemer, &redeem_request.redeemer);

                MockResult::Return(Ok(()))
            });

            assert_ok!(Redeem::execute_redeem(
                Origin::signed(USER),
                H256([0u8; 32]),
                Vec::default(),
                Vec::default()
            ));
            assert_emitted!(Event::ExecuteRedeem {
                redeem_id: H256([0; 32]),
                redeemer: USER,
                vault_id: VAULT,
                amount: 100,
                fee: 0,
                transfer_fee: btc_fee.amount(),
            });
            assert_err!(
                Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
                TestError::RedeemCompleted,
            );
        })
    }

    #[test]
    fn test_cancel_redeem_above_secure_threshold_succeeds() {
        // POSTCONDITIONS:
        // - If reimburse is true:
        //   - If after the loss of collateral the vault remains above the `SecureCollateralThreshold`:
        //       - `decreaseToBeRedeemedTokens` MUST be called, supplying the vault and amountIncludingParachainFee as
        //         arguments.
        run_test(|| {
            let redeem_request = RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 10,
                fee: 0,
                amount_btc: 10,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            };
            inject_redeem_request(H256([0u8; 32]), redeem_request.clone());

            ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(true)));
            ext::vault_registry::is_vault_below_secure_threshold::<Test>.mock_safe(|_| MockResult::Return(Ok(false)));
            ext::vault_registry::ban_vault::<Test>.mock_safe(move |vault| {
                assert_eq!(vault, &VAULT);
                MockResult::Return(Ok(()))
            });
            Amount::<Test>::unlock_on.mock_safe(|_, _| MockResult::Return(Ok(())));
            Amount::<Test>::transfer.mock_safe(|_, _, _| MockResult::Return(Ok(())));
            ext::vault_registry::transfer_funds_saturated::<Test>
                .mock_safe(move |_, _, amount| MockResult::Return(Ok(amount.clone())));
            ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
                MockResult::Return(Ok(vault_registry::types::Vault {
                    status: VaultStatus::Active(true),
                    ..default_vault()
                }))
            });
            ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>.mock_safe(move |vault, amount| {
                assert_eq!(vault, &VAULT);
                assert_eq!(
                    amount,
                    &wrapped(redeem_request.amount_btc + redeem_request.transfer_fee_btc)
                );
                MockResult::Return(Ok(()))
            });
            assert_ok!(Redeem::cancel_redeem(Origin::signed(USER), H256([0u8; 32]), true));
            assert_err!(
                Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
                TestError::RedeemCancelled,
            );
            assert_emitted!(Event::CancelRedeem {
                redeem_id: H256([0; 32]),
                redeemer: USER,
                vault_id: VAULT,
                slashed_amount: 14,
                status: RedeemRequestStatus::Reimbursed(true)
            });
        })
    }

    #[test]
    fn test_cancel_redeem_below_secure_threshold_succeeds() {
        // POSTCONDITIONS:
        // - If reimburse is true:
        //   - If after the loss of collateral the vault is below the `SecureCollateralThreshold`:
        //       - `decreaseTokens` MUST be called, supplying the vault, the user, and amountIncludingParachainFee as
        //         arguments.
        run_test(|| {
            let redeem_request = RedeemRequest {
                period: 0,
                vault: VAULT,
                opentime: 10,
                fee: 0,
                amount_btc: 10,
                premium: 0,
                redeemer: USER,
                btc_address: BtcAddress::random(),
                btc_height: 0,
                status: RedeemRequestStatus::Pending,
                transfer_fee_btc: Redeem::get_current_inclusion_fee(DEFAULT_WRAPPED_CURRENCY)
                    .unwrap()
                    .amount(),
            };
            inject_redeem_request(H256([0u8; 32]), redeem_request.clone());

            ext::btc_relay::has_request_expired::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(true)));
            ext::vault_registry::is_vault_below_secure_threshold::<Test>.mock_safe(|_| MockResult::Return(Ok(true)));
            ext::vault_registry::ban_vault::<Test>.mock_safe(move |vault| {
                assert_eq!(vault, &VAULT);
                MockResult::Return(Ok(()))
            });
            Amount::<Test>::unlock_on.mock_safe(|_, _| MockResult::Return(Ok(())));
            Amount::<Test>::burn_from.mock_safe(|_, _| MockResult::Return(Ok(())));
            ext::vault_registry::transfer_funds_saturated::<Test>
                .mock_safe(move |_, _, amount| MockResult::Return(Ok(amount.clone())));
            ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
                MockResult::Return(Ok(vault_registry::types::Vault {
                    status: VaultStatus::Active(true),
                    ..default_vault()
                }))
            });
            ext::vault_registry::decrease_tokens::<Test>.mock_safe(move |vault, user, amount| {
                assert_eq!(vault, &VAULT);
                assert_eq!(user, &USER);
                assert_eq!(
                    amount,
                    &wrapped(redeem_request.amount_btc + redeem_request.transfer_fee_btc)
                );
                MockResult::Return(Ok(()))
            });
            assert_ok!(Redeem::cancel_redeem(Origin::signed(USER), H256([0u8; 32]), true));
            assert_err!(
                Redeem::get_open_redeem_request_from_id(&H256([0u8; 32])),
                TestError::RedeemCancelled,
            );
            assert_emitted!(Event::CancelRedeem {
                redeem_id: H256([0; 32]),
                redeemer: USER,
                vault_id: VAULT,
                slashed_amount: 14,
                status: RedeemRequestStatus::Reimbursed(false)
            });
        })
    }
}
