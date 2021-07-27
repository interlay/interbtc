mod mock;

use mock::{redeem_testing_utils::*, *};

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(VAULT, default_vault_state());
        // additional vault in order to prevent the edge case where the fee pool does not
        // get additional funds because there are no non-liquidated vaults left
        CoreVaultData::force_to(CAROL, default_vault_state());
        execute()
    })
}

/// to-be-replaced & replace_collateral are decreased in request_redeem
fn consume_to_be_replaced(vault: &mut CoreVaultData, amount_btc: u128) {
    let to_be_replaced_decrease = amount_btc.min(vault.to_be_replaced);
    let released_replace_collateral = (vault.replace_collateral * to_be_replaced_decrease) / vault.to_be_replaced;

    vault.replace_collateral -= released_replace_collateral;
    vault.griefing_collateral -= released_replace_collateral;
    vault.free_balance += released_replace_collateral;

    vault.to_be_replaced -= to_be_replaced_decrease;
}

mod spec_based_tests {
    use super::*;
    #[test]
    fn integration_test_redeem_with_parachain_shutdown_status_fails() {
        // Checked PRECONDITION: The BTC Parachain status in the Security component
        test_with(|| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Redeem(RedeemCall::request_redeem(
                    1500,
                    BtcAddress::P2PKH(H160([0u8; 20])),
                    account_of(BOB),
                ))
                .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::execute_redeem(
                    Default::default(),
                    Default::default(),
                    Default::default()
                ))
                .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainShutdown,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::cancel_redeem(Default::default(), false))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );
            assert_noop!(
                Call::Redeem(RedeemCall::cancel_redeem(Default::default(), true))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::liquidation_redeem(1000)).dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainShutdown,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::mint_tokens_for_reimbursed_redeem(Default::default()))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );
        });
    }

    #[test]
    fn integration_test_redeem_with_parachain_error_status_fails() {
        // Checked PRECONDITION: The BTC Parachain status in the Security component
        test_with(|| {
            // `liquidation_redeem` and `execute_redeem` are not tested here
            // because they only require the parachain status not to be `Shutdown`
            SecurityPallet::set_status(StatusCode::Error);

            assert_noop!(
                Call::Redeem(RedeemCall::request_redeem(
                    1500,
                    BtcAddress::P2PKH(H160([0u8; 20])),
                    account_of(BOB),
                ))
                .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::cancel_redeem(Default::default(), false))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );
            assert_noop!(
                Call::Redeem(RedeemCall::cancel_redeem(Default::default(), true))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );

            assert_noop!(
                Call::Redeem(RedeemCall::mint_tokens_for_reimbursed_redeem(Default::default()))
                    .dispatch(origin_of(account_of(ALICE))),
                SecurityError::ParachainNotRunning,
            );
        });
    }

    mod request_redeem {
        use frame_support::assert_ok;
        use sp_runtime::FixedU128;

        use super::*;

        fn calculate_vault_capacity() -> u128 {
            let redeemable_tokens = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED;

            // we are able to redeem `redeemable_tokens`. However, when requesting a redeem,
            // the fee is subtracted for this amount. As such, a user is able to request more
            // than `redeemable_tokens`. A first approximation of the limit is redeemable_tokens+fee,
            // however, this slightly underestimates it. Since the actual fee rate is not exposed,
            // use an iterative process to find the maximum redeem request amount.
            let mut ret = redeemable_tokens + FeePallet::get_redeem_fee(redeemable_tokens).unwrap();

            loop {
                let actually_redeemed_tokens = ret - FeePallet::get_redeem_fee(ret).unwrap();
                if actually_redeemed_tokens > redeemable_tokens {
                    return ret - 1;
                }
                ret += 1;
            }
        }

        #[test]
        fn integration_test_request_redeem_at_capacity_succeeds() {
            // Checked PRECONDITION: The vault’s `issuedTokens` MUST be at least `vault.toBeRedeemedTokens +
            // burnedTokens`
            test_with(|| {
                let amount = calculate_vault_capacity();
                assert_ok!(Call::Redeem(RedeemCall::request_redeem(
                    amount,
                    BtcAddress::default(),
                    account_of(VAULT)
                ))
                .dispatch(origin_of(account_of(USER))));

                let redeem_id = assert_redeem_request_event();
                let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

                assert_eq!(amount, redeem.fee + redeem.amount_btc + redeem.transfer_fee_btc);
                assert_eq!(redeem.vault, account_of(VAULT));

                assert_eq!(
                    ParachainState::get(),
                    ParachainState::default().with_changes(|user, vault, _, _| {
                        vault.to_be_redeemed += redeem.amount_btc + redeem.transfer_fee_btc;
                        user.free_tokens -= redeem.amount_btc + redeem.transfer_fee_btc + redeem.fee;
                        user.locked_tokens += redeem.amount_btc + redeem.transfer_fee_btc + redeem.fee;
                        consume_to_be_replaced(vault, redeem.amount_btc);
                    })
                );
            });
        }

        #[test]
        fn integration_test_request_redeem_above_capacity_fails() {
            // Checked PRECONDITION: The vault’s `issuedTokens` MUST be at least `vault.toBeRedeemedTokens +
            // burnedTokens`
            test_with(|| {
                let amount = calculate_vault_capacity() + 1;
                assert_noop!(
                    Call::Redeem(RedeemCall::request_redeem(
                        amount,
                        BtcAddress::default(),
                        account_of(VAULT)
                    ))
                    .dispatch(origin_of(account_of(USER))),
                    VaultRegistryError::InsufficientTokensCommitted
                );
            });
        }

        #[test]
        fn integration_test_redeem_cannot_request_from_liquidated_vault() {
            // Checked PRECONDITION: The selected vault MUST NOT be liquidated.
            test_with(|| {
                drop_exchange_rate_and_liquidate(VAULT);
                assert_noop!(
                    Call::Redeem(RedeemCall::request_redeem(
                        1500,
                        BtcAddress::P2PKH(H160([0u8; 20])),
                        account_of(VAULT),
                    ))
                    .dispatch(origin_of(account_of(ALICE))),
                    VaultRegistryError::VaultNotFound,
                );
            });
        }

        #[test]
        fn integration_test_redeem_redeemer_free_tokens() {
            // Checked PRECONDITION: The redeemer MUST have at least `amountWrapped` free tokens.
            test_with(|| {
                let free_tokens_to_redeem = 1500;
                UserData::force_to(
                    ALICE,
                    UserData {
                        free_tokens: free_tokens_to_redeem,
                        ..UserData::get(USER)
                    },
                );
                assert_ok!(Call::Redeem(RedeemCall::request_redeem(
                    free_tokens_to_redeem,
                    BtcAddress::P2PKH(H160([0u8; 20])),
                    account_of(VAULT),
                ))
                .dispatch(origin_of(account_of(ALICE))));
                UserData::force_to(
                    ALICE,
                    UserData {
                        free_tokens: free_tokens_to_redeem - 1,
                        ..UserData::get(USER)
                    },
                );
                assert_noop!(
                    Call::Redeem(RedeemCall::request_redeem(
                        free_tokens_to_redeem,
                        BtcAddress::P2PKH(H160([0u8; 20])),
                        account_of(VAULT),
                    ))
                    .dispatch(origin_of(account_of(ALICE))),
                    RedeemError::AmountExceedsUserBalance,
                );
            });
        }

        #[test]
        fn integration_test_redeem_vault_capacity_sufficient() {
            // Checked PRECONDITION: The vault’s `issuedTokens` MUST be at least `vault.toBeRedeemedTokens +
            // burnedTokens`.
            // Checked POSTCONDITIONS:
            //  - The vault’s `toBeRedeemedTokens` MUST increase by `burnedTokens`.
            //  - `amountWrapped` of the redeemer’s tokens MUST be locked by this transaction.
            //  - If the vault’s collateralization rate is above the PremiumRedeemThreshold, then `redeem.premium` MUST
            //    be 0
            test_with(|| {
                let vault_to_be_redeemed = 1500;
                let user_to_redeem = 1500;
                set_redeem_state(vault_to_be_redeemed, user_to_redeem, USER, VAULT);
                let redeem_fee = FeePallet::get_redeem_fee(user_to_redeem).unwrap();
                let burned_tokens = user_to_redeem - redeem_fee;

                CoreVaultData::force_to(
                    VAULT,
                    CoreVaultData {
                        backing_collateral: DEFAULT_VAULT_BACKING_COLLATERAL,
                        ..CoreVaultData::vault(VAULT)
                    },
                );
                let parachain_state_before_request = ParachainState::get();
                let redeem_id = setup_redeem(user_to_redeem, USER, VAULT, 1_000_000);
                let actual_redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
                assert_eq!(actual_redeem, default_redeem_request(user_to_redeem, VAULT, USER));
                assert_eq!(
                    ParachainState::get(),
                    parachain_state_before_request.with_changes(|user, vault, _liquidation_vault, _fee_pool| {
                        user.locked_tokens += user_to_redeem;
                        user.free_tokens -= user_to_redeem;
                        vault.to_be_redeemed += burned_tokens;
                    })
                );
            });
        }

        #[test]
        fn integration_test_redeem_with_premium() {
            // Checked PRECONDITION: The vault’s `issuedTokens` MUST be at least `vault.toBeRedeemedTokens +
            // burnedTokens`.
            // Checked POSTCONDITIONS:
            //  - The vault’s `toBeRedeemedTokens` MUST increase by `burnedTokens`.
            //  - `amountWrapped` of the redeemer’s tokens MUST be locked by this transaction.
            //  - If the vault’s collateralization rate is below the PremiumRedeemThreshold, then `redeem.premium` MUST
            //    be
            // PremiumRedeemFee multiplied by the worth of `redeem.amountBtc`
            test_with(|| {
                let vault_to_be_redeemed = 1500;
                let user_to_redeem = 1500;
                set_redeem_state(vault_to_be_redeemed, user_to_redeem, USER, VAULT);
                setup_redeem(user_to_redeem, USER, VAULT, 1_000_000);
                let redeem_id = assert_redeem_request_event();
                let actual_redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
                assert_eq!(actual_redeem, premium_redeem_request(user_to_redeem, VAULT, USER));
            });
        }

        #[test]
        fn integration_test_redeem_vault_capacity_insufficient() {
            // Checked PRECONDITION: The vault’s `issuedTokens` MUST be at least `vault.toBeRedeemedTokens +
            // burnedTokens`.
            test_with(|| {
                let vault_to_be_redeemed = 1500;
                let user_to_redeem = 1500;
                set_redeem_state(vault_to_be_redeemed, user_to_redeem, USER, VAULT);
                let core_vault = CoreVaultData::vault(VAULT);
                CoreVaultData::force_to(
                    VAULT,
                    CoreVaultData {
                        issued: core_vault.issued - 1,
                        ..core_vault
                    },
                );
                assert_noop!(
                    Call::Redeem(RedeemCall::request_redeem(
                        user_to_redeem,
                        BtcAddress::P2PKH(H160([0u8; 20])),
                        account_of(VAULT),
                    ))
                    .dispatch(origin_of(account_of(ALICE))),
                    VaultRegistryError::InsufficientTokensCommitted
                );
            });
        }

        #[test]
        fn integration_test_redeem_dust_value() {
            // Checked PRECONDITION: `burnedTokens` minus the inclusion fee MUST be above the RedeemBtcDustValue,
            // where the inclusion fee is the multiplication of RedeemTransactionSize and the fee rate estimate
            // reported by the oracle.

            test_with(|| {
                // The formula for finding the threshold `to_redeem` for the dust amount error is
                // `(redeem_dust_value + inclusion_fee) / (1 - redeem_fee_rate)`
                let redeem_dust_value = RedeemPallet::redeem_btc_dust_value();
                let inclusion_fee = RedeemPallet::get_current_inclusion_fee().unwrap();
                let redeem_fee_rate = FeePallet::redeem_fee();
                let denominator = FixedU128::one() - redeem_fee_rate;
                let numerator = FixedU128::from_inner(redeem_dust_value + inclusion_fee);
                let to_redeem = (numerator / denominator).into_inner();
                assert_noop!(
                    Call::Redeem(RedeemCall::request_redeem(
                        to_redeem - 1,
                        BtcAddress::P2PKH(H160([0u8; 20])),
                        account_of(VAULT),
                    ))
                    .dispatch(origin_of(account_of(ALICE))),
                    RedeemError::AmountBelowDustAmount
                );
                assert_ok!(Call::Redeem(RedeemCall::request_redeem(
                    to_redeem,
                    BtcAddress::P2PKH(H160([0u8; 20])),
                    account_of(VAULT),
                ))
                .dispatch(origin_of(account_of(ALICE))));
            });
        }
    }
}

mod expiry_test {
    use super::*;

    fn set_redeem_period(period: u32) {
        assert_ok!(Call::Redeem(RedeemCall::set_redeem_period(period)).dispatch(root()));
    }

    fn request_redeem() -> H256 {
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            4_000,
            BtcAddress::default(),
            account_of(VAULT)
        ))
        .dispatch(origin_of(account_of(USER))));
        // get the redeem id
        assert_redeem_request_event()
    }

    fn execute_redeem(redeem_id: H256) -> DispatchResultWithPostInfo {
        ExecuteRedeemBuilder::new(redeem_id).execute()
    }

    fn cancel_redeem(redeem_id: H256) -> DispatchResultWithPostInfo {
        Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER)))
    }

    #[test]
    fn integration_test_redeem_expiry_only_parachain_blocks_expired() {
        test_with(|| {
            set_redeem_period(1000);
            let redeem_id = request_redeem();
            mine_blocks(1);
            SecurityPallet::set_active_block_number(10000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_noop!(cancel_redeem(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_redeem(redeem_id));
        });
    }

    #[test]
    fn integration_test_redeem_expiry_only_bitcoin_blocks_expired() {
        test_with(|| {
            set_redeem_period(1000);
            let redeem_id = request_redeem();
            SecurityPallet::set_active_block_number(100);
            mine_blocks(20);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_noop!(cancel_redeem(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_redeem(redeem_id));
        });
    }

    #[test]
    fn integration_test_redeem_expiry_no_period_change_pre_expiry() {
        test_with(|| {
            set_redeem_period(1000);
            let redeem_id = request_redeem();
            SecurityPallet::set_active_block_number(750);
            mine_blocks(1);

            assert_noop!(cancel_redeem(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_redeem(redeem_id));
        });
    }

    #[test]
    fn integration_test_redeem_expiry_no_period_change_post_expiry() {
        // can still execute after expiry
        test_with(|| {
            set_redeem_period(1000);
            let redeem_id = request_redeem();
            mine_blocks(12);
            SecurityPallet::set_active_block_number(1100);
            assert_ok!(execute_redeem(redeem_id));
        });

        // .. but user can also cancel. Whoever is first wins
        test_with(|| {
            set_redeem_period(1000);
            let redeem_id = request_redeem();
            mine_blocks(12);
            SecurityPallet::set_active_block_number(1100);
            assert_ok!(cancel_redeem(redeem_id));
        });
    }

    #[test]
    fn integration_test_redeem_expiry_with_period_decrease() {
        test_with(|| {
            set_redeem_period(2000);
            let redeem_id = request_redeem();
            SecurityPallet::set_active_block_number(1100);
            mine_blocks(12);
            set_redeem_period(1000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_noop!(cancel_redeem(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_redeem(redeem_id));
        });
    }

    #[test]
    fn integration_test_redeem_expiry_with_period_increase() {
        test_with(|| {
            set_redeem_period(100);
            let redeem_id = request_redeem();
            SecurityPallet::set_active_block_number(110);
            mine_blocks(12);
            set_redeem_period(200);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_noop!(cancel_redeem(redeem_id), RedeemError::TimeNotExpired);
            assert_ok!(execute_redeem(redeem_id));
        });
    }
}

#[test]
fn integration_test_redeem_parachain_status_shutdown_fails() {
    test_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(0, account_of(BOB), 0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );

        assert_noop!(
            Call::Issue(IssueCall::cancel_issue(H256([0; 32]),)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );

        assert_noop!(
            Call::Issue(IssueCall::execute_issue(H256([0; 32]), vec![0u8; 32], vec![0u8; 32]))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown,
        );
    });
}

mod execute_redeem_payment_limits {
    use super::*;

    #[test]
    fn integration_test_redeem_polka_btc_execute_underpayment_fails() {
        test_with(|| {
            let redeem_id = setup_redeem(10_000, USER, VAULT, 1_000_000);
            let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_noop!(
                ExecuteRedeemBuilder::new(redeem_id)
                    .with_amount(redeem.amount_btc - 1)
                    .execute(),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }

    #[test]
    fn integration_test_redeem_polka_btc_execute_with_exact_amount_succeeds() {
        test_with(|| {
            let redeem_id = setup_redeem(10_000, USER, VAULT, 1_000_000);
            let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            ExecuteRedeemBuilder::new(redeem_id)
                .with_amount(redeem.amount_btc)
                .assert_execute();
        });
    }

    #[test]
    fn integration_test_redeem_polka_btc_execute_overpayment_fails() {
        test_with(|| {
            let redeem_id = setup_redeem(10_000, USER, VAULT, 1_000_000);
            let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

            assert_noop!(
                ExecuteRedeemBuilder::new(redeem_id)
                    .with_amount(redeem.amount_btc + 1)
                    .execute(),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

#[test]
fn integration_test_redeem_wrapped_execute() {
    test_with(|| {
        let issued_tokens = 10_000;
        let collateral_vault = 1_000_000;

        let redeem_id = setup_redeem(issued_tokens, USER, VAULT, collateral_vault);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

        execute_redeem(redeem_id);

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                vault.issued -= redeem.amount_btc + redeem.transfer_fee_btc;
                user.free_tokens -= issued_tokens;
                fee_pool.vault_rewards += redeem.fee;
                consume_to_be_replaced(vault, redeem.amount_btc + redeem.transfer_fee_btc);
            })
        );
    });
}

#[test]
fn integration_test_premium_redeem_wrapped_execute() {
    test_with(|| {
        let issued_tokens = 10_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // make vault undercollateralized. Note that we place it under the liquidation threshold
        // as well, but as long as we don't call liquidate that's ok
        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::from(100)));

        // alice requests to redeem issued_tokens from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            issued_tokens,
            user_btc_address,
            account_of(VAULT)
        ))
        .dispatch(origin_of(account_of(USER))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // send the btc from the vault to the user
        let (_tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(user_btc_address, redeem.amount_btc, Some(redeem_id));

        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

        assert_ok!(
            Call::Redeem(RedeemCall::execute_redeem(redeem_id, merkle_proof, raw_tx))
                .dispatch(origin_of(account_of(VAULT)))
        );

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                // fee moves from user to fee_pool
                user.free_tokens -= redeem.fee;
                fee_pool.vault_rewards += redeem.fee;
                // amount_btc is burned from user and decreased on vault
                let burned_amount = redeem.amount_btc + redeem.transfer_fee_btc;
                vault.issued -= burned_amount;
                user.free_tokens -= burned_amount;
                // premium is moved from vault to user
                vault.backing_collateral -= redeem.premium;
                user.free_balance += redeem.premium;
                consume_to_be_replaced(vault, burned_amount);
            })
        );

        assert!(redeem.premium > 0); // sanity check that our test is useful
    });
}

#[test]
fn integration_test_redeem_wrapped_liquidation_redeem() {
    test_with(|| {
        let issued = 400;
        let to_be_issued = 100;
        let to_be_redeemed = 50;
        let liquidation_redeem_amount = 325;

        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued,
                to_be_issued,
                to_be_redeemed,
                backing_collateral: 10_000,
                ..Default::default()
            },
        );

        // create tokens for the vault and user
        drop_exchange_rate_and_liquidate(VAULT);

        let post_liquidation_state = ParachainState::get();

        assert_noop!(
            Call::Redeem(RedeemCall::liquidation_redeem(351)).dispatch(origin_of(account_of(USER))),
            VaultRegistryError::InsufficientTokensCommitted
        );

        assert_ok!(Call::Redeem(RedeemCall::liquidation_redeem(liquidation_redeem_amount))
            .dispatch(origin_of(account_of(USER))));

        // NOTE: changes are relative the the post liquidation state
        assert_eq!(
            ParachainState::get(),
            post_liquidation_state.with_changes(|user, _vault, liquidation_vault, _fee_pool| {
                let reward = (liquidation_vault.backing_collateral * liquidation_redeem_amount)
                    / (liquidation_vault.issued + liquidation_vault.to_be_issued);

                user.free_tokens -= liquidation_redeem_amount;
                user.free_balance += reward;

                liquidation_vault.issued -= liquidation_redeem_amount;
                liquidation_vault.backing_collateral -= reward;
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_cancel_reimburse_sufficient_collateral_for_wrapped() {
    test_with(|| {
        let amount_btc = 10_000;

        let redeem_id = setup_cancelable_redeem(USER, VAULT, 100000000, amount_btc);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_collateral =
            ExchangeRateOraclePallet::wrapped_to_collateral(redeem.amount_btc + redeem.transfer_fee_btc).unwrap();

        let punishment_fee = FeePallet::get_punishment_fee(amount_without_fee_collateral).unwrap();
        assert!(punishment_fee > 0);

        // alice cancels redeem request and chooses to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER))));

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, fee_pool| {
                // vault gets slashed for 110% to user
                vault.backing_collateral -= amount_without_fee_collateral + punishment_fee;
                vault.free_tokens += redeem.amount_btc + redeem.transfer_fee_btc;

                user.free_balance += amount_without_fee_collateral + punishment_fee;
                user.free_tokens -= amount_btc;

                fee_pool.vault_rewards += redeem.fee;

                consume_to_be_replaced(vault, redeem.amount_btc + redeem.transfer_fee_btc);
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_cancel_reimburse_insufficient_collateral_for_wrapped() {
    test_with(|| {
        let amount_btc = 10_000;

        // set collateral to the minimum amount required, such that the vault can not afford to both
        // reimburse and keep collateral his current tokens
        let required_collateral =
            VaultRegistryPallet::get_required_collateral_for_wrapped(DEFAULT_VAULT_ISSUED).unwrap();
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: required_collateral,
                ..CoreVaultData::vault(VAULT)
            },
        );
        let initial_state = ParachainState::get();

        let redeem_id = setup_cancelable_redeem(USER, VAULT, 100000000, amount_btc);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_as_collateral =
            ExchangeRateOraclePallet::wrapped_to_collateral(redeem.amount_btc + redeem.transfer_fee_btc).unwrap();

        let punishment_fee = FeePallet::get_punishment_fee(amount_without_fee_as_collateral).unwrap();
        assert!(punishment_fee > 0);

        // alice cancels redeem request and chooses to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER))));

        assert_eq!(
            ParachainState::get(),
            initial_state.with_changes(|user, vault, _, fee_pool| {
                // vault gets slashed for 110% to user
                vault.backing_collateral -= amount_without_fee_as_collateral + punishment_fee;
                // vault free tokens does not change, and issued tokens is reduced
                vault.issued -= redeem.amount_btc + redeem.transfer_fee_btc;

                user.free_balance += amount_without_fee_as_collateral + punishment_fee;
                user.free_tokens -= amount_btc;

                fee_pool.vault_rewards += redeem.fee;

                consume_to_be_replaced(vault, redeem.amount_btc + redeem.transfer_fee_btc);
            })
        );

        SecurityPallet::set_active_block_number(100000000);
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: required_collateral + amount_btc * 2,
                ..CoreVaultData::vault(VAULT)
            },
        );
        let pre_minting_state = ParachainState::get();

        assert_ok!(Call::Redeem(RedeemCall::mint_tokens_for_reimbursed_redeem(redeem_id))
            .dispatch(origin_of(account_of(VAULT))));
        assert_eq!(
            ParachainState::get(),
            pre_minting_state.with_changes(|_user, vault, _, _fee_pool| {
                vault.issued += redeem.amount_btc + redeem.transfer_fee_btc;
                vault.free_tokens += redeem.amount_btc + redeem.transfer_fee_btc;
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_cancel_no_reimburse() {
    test_with(|| {
        let amount_btc = 10_000;

        let redeem_id = setup_cancelable_redeem(USER, VAULT, 100000000, amount_btc);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_collateral =
            ExchangeRateOraclePallet::wrapped_to_collateral(redeem.amount_btc + redeem.transfer_fee_btc).unwrap();

        let punishment_fee = FeePallet::get_punishment_fee(amount_without_fee_collateral).unwrap();
        assert!(punishment_fee > 0);

        // alice cancels redeem request and chooses not to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, false)).dispatch(origin_of(account_of(USER))));

        assert_eq!(
            ParachainState::get(),
            ParachainState::default().with_changes(|user, vault, _, _| {
                // vault is slashed a punishment fee of 10%

                user.free_balance += punishment_fee;

                vault.backing_collateral -= punishment_fee;

                consume_to_be_replaced(vault, redeem.amount_btc + redeem.transfer_fee_btc);
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_cancel_liquidated_no_reimburse() {
    test_with(|| {
        let issued_tokens = 10_000;
        let collateral_vault = 1_000_000;
        let redeem_id = setup_cancelable_redeem(USER, VAULT, collateral_vault, issued_tokens);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // setup vault state such that 1/4th of its collateral is freed after successful redeem
        let consumed_issued_tokens = redeem.amount_btc + redeem.transfer_fee_btc;
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: consumed_issued_tokens * 4,
                to_be_issued: 0,
                to_be_redeemed: consumed_issued_tokens * 4,
                backing_collateral: collateral_vault,
                to_be_replaced: 0,
                replace_collateral: 0,
                ..default_vault_state()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);

        let post_liquidation_state = ParachainState::get();

        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, false)).dispatch(origin_of(account_of(USER))));

        // NOTE: changes are relative the the post liquidation state
        assert_eq!(
            ParachainState::get(),
            post_liquidation_state.with_changes(|user, vault, liquidation_vault, _fee_pool| {
                // to-be-redeemed decreased, forwarding to liquidation vault
                vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;
                liquidation_vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;

                // the collateral that remained with the vault to back this redeem is now transferred to the liquidation
                // vault
                let collateral_for_this_redeem = collateral_vault / 4;
                vault.liquidated_collateral -= collateral_for_this_redeem;
                liquidation_vault.backing_collateral += collateral_for_this_redeem;

                // user's tokens get unlocked
                user.locked_tokens -= redeem.amount_btc + redeem.fee + redeem.transfer_fee_btc;
                user.free_tokens += redeem.amount_btc + redeem.fee + redeem.transfer_fee_btc;

                // Note that no punishment is taken from vault, because it's already liquidated
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_cancel_liquidated_reimburse() {
    test_with(|| {
        let issued_tokens = 10_000;
        let collateral_vault = 1_000_000;
        let redeem_id = setup_cancelable_redeem(USER, VAULT, collateral_vault, issued_tokens);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // setup vault state such that 1/4th of its collateral is freed after successful redeem
        let consumed_issued_tokens = redeem.amount_btc + redeem.transfer_fee_btc;
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: consumed_issued_tokens * 4,
                to_be_issued: 0,
                to_be_redeemed: consumed_issued_tokens * 4,
                backing_collateral: collateral_vault,
                to_be_replaced: 0,
                replace_collateral: 0,
                ..default_vault_state()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);

        let post_liquidation_state = ParachainState::get();

        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER))));

        // NOTE: changes are relative the the post liquidation state
        assert_eq!(
            ParachainState::get(),
            post_liquidation_state.with_changes(|user, vault, liquidation_vault, fee_pool| {
                // to-be-redeemed decreased, forwarding to liquidation vault
                vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;
                liquidation_vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;

                // tokens are given to the vault, minus a fee that is given to the fee pool
                vault.free_tokens += redeem.amount_btc + redeem.transfer_fee_btc;
                fee_pool.vault_rewards += redeem.fee;

                // the collateral that remained with the vault to back this redeem is transferred to the user
                let collateral_for_this_redeem = collateral_vault / 4;
                vault.liquidated_collateral -= collateral_for_this_redeem;
                user.free_balance += collateral_for_this_redeem;

                // user's tokens get burned
                user.locked_tokens -= issued_tokens;

                // Note that no punishment is taken from vault, because it's already liquidated
            })
        );
    });
}

#[test]
fn integration_test_redeem_wrapped_execute_liquidated() {
    test_with(|| {
        let issued_tokens = 10_000;
        let collateral_vault = 1_000_000;

        let redeem_id = setup_redeem(issued_tokens, USER, VAULT, collateral_vault);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // setup vault state such that 1/4th of its collateral is freed after successful redeem
        let consumed_issued_tokens = redeem.amount_btc + redeem.transfer_fee_btc;
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: consumed_issued_tokens * 4,
                to_be_issued: 0,
                to_be_redeemed: consumed_issued_tokens * 4,
                backing_collateral: collateral_vault,
                to_be_replaced: 0,
                replace_collateral: 0,
                ..default_vault_state()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);

        let post_liquidation_state = ParachainState::get();

        execute_redeem(redeem_id);

        // NOTE: changes are relative the the post liquidation state
        assert_eq!(
            ParachainState::get(),
            post_liquidation_state.with_changes(|user, vault, liquidation_vault, fee_pool| {
                // fee given to fee pool
                fee_pool.vault_rewards += redeem.fee;

                // wrapped burned from user
                user.locked_tokens -= issued_tokens;

                // to-be-redeemed & issued decreased, forwarding to liquidation vault
                vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;
                liquidation_vault.to_be_redeemed -= redeem.amount_btc + redeem.transfer_fee_btc;
                liquidation_vault.issued -= redeem.amount_btc + redeem.transfer_fee_btc;

                // collateral released
                let released_collateral = vault.liquidated_collateral / 4;
                vault.liquidated_collateral -= released_collateral;
                vault.backing_collateral += released_collateral;
            })
        );
    });
}

mod mint_tokens_for_reimbursed_redeem_equivalence_test {
    use super::*;

    fn setup_cancelable_redeem_with_insufficient_collateral_for_reimburse() -> H256 {
        let amount_btc = 10_000;

        // set collateral to the minimum amount required, such that the vault can not afford to both
        // reimburse and keep collateral his current tokens
        let required_collateral =
            VaultRegistryPallet::get_required_collateral_for_wrapped(DEFAULT_VAULT_ISSUED).unwrap();
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                backing_collateral: required_collateral,
                ..CoreVaultData::vault(VAULT)
            },
        );
        let redeem_id = setup_cancelable_redeem(USER, VAULT, 100000000, amount_btc);
        let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_as_collateral =
            ExchangeRateOraclePallet::wrapped_to_collateral(redeem.amount_btc + redeem.transfer_fee_btc).unwrap();

        let punishment_fee = FeePallet::get_punishment_fee(amount_without_fee_as_collateral).unwrap();
        assert!(punishment_fee > 0);

        redeem_id
    }

    fn get_additional_collateral() {
        assert_ok!(VaultRegistryPallet::transfer_funds(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Collateral(account_of(VAULT)),
            100_000_000_000,
        ));
    }

    #[test]
    fn integration_test_mint_tokens_for_reimbursed_redeem_equivalence_to_succesful_cancel() {
        // scenario 1: sufficient collateral
        let result1 = test_with(|| {
            let redeem_id = setup_cancelable_redeem_with_insufficient_collateral_for_reimburse();
            get_additional_collateral();
            assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER))));
            ParachainState::get()
        });
        // scenario 2: insufficient collateral
        let result2 = test_with(|| {
            let redeem_id = setup_cancelable_redeem_with_insufficient_collateral_for_reimburse();
            assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true)).dispatch(origin_of(account_of(USER))));
            get_additional_collateral();
            SecurityPallet::set_active_block_number(100000000);
            assert_ok!(Call::Redeem(RedeemCall::mint_tokens_for_reimbursed_redeem(redeem_id))
                .dispatch(origin_of(account_of(VAULT))));
            ParachainState::get()
        });
        // the states should be identical
        assert_eq!(result1, result2);
    }
}
