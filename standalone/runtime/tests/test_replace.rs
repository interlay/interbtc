mod mock;

use currency::Amount;
use mock::{assert_eq, *};

use sp_core::H256;

type IssueCall = issue::Call<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;
pub type ReplaceError = replace::Error<Runtime>;

const USER: [u8; 32] = ALICE;
const OLD_VAULT: [u8; 32] = BOB;
const NEW_VAULT: [u8; 32] = CAROL;
pub const DEFAULT_GRIEFING_COLLATERAL: Amount<Runtime> = griefing(5_000);

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            set_default_thresholds();
            UserData::force_to(USER, default_user_state());
            CoreVaultData::force_to(OLD_VAULT, default_vault_state(currency_id));
            CoreVaultData::force_to(NEW_VAULT, default_vault_state(currency_id));
            LiquidationVaultData::force_to(default_liquidation_vault_state(currency_id));
            execute(currency_id)
        })
    };
    test_with(CurrencyId::DOT);
    test_with(CurrencyId::KSM);
}

fn test_without_initialization<R>(execute: impl Fn(CurrencyId) -> R) {
    ExtBuilder::build().execute_with(|| execute(CurrencyId::DOT));
    ExtBuilder::build().execute_with(|| execute(CurrencyId::KSM));
}

fn assert_request_event() {
    let events = SystemModule::events();
    let ids = events.iter().filter_map(|r| match r.event {
        Event::Replace(ReplaceEvent::RequestReplace(_, _, _)) => Some(()),
        _ => None,
    });
    assert_eq!(ids.count(), 1);
}

pub fn assert_accept_event() -> H256 {
    SystemModule::events()
        .iter()
        .rev()
        .find_map(|record| match record.event {
            Event::Replace(ReplaceEvent::AcceptReplace(id, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .unwrap()
}

fn accept_replace(
    amount_btc: Amount<Runtime>,
    griefing_collateral: Amount<Runtime>,
) -> (H256, ReplaceRequest<AccountId32, u32, u128>) {
    assert_ok!(Call::Replace(ReplaceCall::accept_replace(
        account_of(OLD_VAULT),
        amount_btc.amount(),
        griefing_collateral.amount(),
        BtcAddress::P2PKH(H160([1; 20]))
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    let replace_id = assert_accept_event();
    let replace = ReplacePallet::get_open_replace_request(&replace_id).unwrap();
    (replace_id, replace)
}

#[cfg(test)]
mod accept_replace_tests {
    use super::{assert_eq, *};

    fn assert_state_after_accept_replace_correct(
        currency_id: CurrencyId,
        replace: &ReplaceRequest<AccountId32, u32, u128>,
    ) {
        assert_eq!(
            ParachainTwoVaultState::get(),
            ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, new_vault, _| {
                *new_vault.free_balance.get_mut(&currency_id).unwrap() -= replace.collateral().unwrap();
                new_vault.backing_collateral += replace.collateral().unwrap();

                old_vault.replace_collateral -= griefing(
                    (old_vault.replace_collateral.amount() * replace.amount) / old_vault.to_be_replaced.amount(),
                );
                old_vault.to_be_replaced -= replace.amount();

                old_vault.to_be_redeemed += replace.amount();
                new_vault.to_be_issued += replace.amount();
            })
        );
    }

    #[test]
    fn integration_test_replace_accept_replace_at_capacity_succeeds() {
        test_with(|currency_id| {
            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED;
            let new_vault_additional_collateral = Amount::new(10_000, currency_id);

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount(), accept_amount);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(currency_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_capacity_succeeds() {
        test_with(|currency_id| {
            // accept only 25%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let new_vault_additional_collateral = Amount::new(10_000, currency_id);

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount(), accept_amount);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL / 4);

            assert_state_after_accept_replace_correct(currency_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_above_capacity_succeeds() {
        test_with(|currency_id| {
            // try to accept 400%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED * 4;
            let new_vault_additional_collateral = Amount::new(10_000, currency_id);

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount(), accept_amount / 4);
            assert_eq!(replace.collateral().unwrap(), new_vault_additional_collateral / 4);
            assert_eq!(replace.griefing_collateral(), DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(currency_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_by_vault_that_does_not_accept_issues_succeeds() {
        test_with(|currency_id| {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::accept_new_issues(false))
                .dispatch(origin_of(account_of(NEW_VAULT))));

            let (_, replace) = accept_replace(wrapped(1000), griefing(1000));

            assert_state_after_accept_replace_correct(currency_id, &replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_dust_fails() {
        test_with(|currency_id| {
            // if the new_vault _asks_ for an amount below below DUST, it gets rejected

            assert_noop!(
                Call::Replace(ReplaceCall::accept_replace(
                    account_of(OLD_VAULT),
                    1,
                    10_000,
                    BtcAddress::P2PKH(H160([1; 20]))
                ))
                .dispatch(origin_of(account_of(NEW_VAULT))),
                ReplaceError::AmountBelowDustAmount
            );

            // if the old_vault does not have sufficient to-be-replaced tokens, it gets rejected
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: wrapped(1),
                    ..default_vault_state(currency_id)
                },
            );
            assert_noop!(
                Call::Replace(ReplaceCall::accept_replace(
                    account_of(OLD_VAULT),
                    1000,
                    10_000,
                    BtcAddress::P2PKH(H160([1; 20]))
                ))
                .dispatch(origin_of(account_of(NEW_VAULT))),
                ReplaceError::AmountBelowDustAmount
            );
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_self_fails() {
        test_with(|_currency_id| {
            assert_noop!(
                Call::Replace(ReplaceCall::accept_replace(
                    account_of(OLD_VAULT),
                    DEFAULT_VAULT_TO_BE_REPLACED.amount(),
                    10_000,
                    BtcAddress::P2PKH(H160([1; 20]))
                ))
                .dispatch(origin_of(account_of(OLD_VAULT))),
                ReplaceError::ReplaceSelfNotAllowed
            );
        });
    }
}

mod request_replace_tests {
    use super::{assert_eq, *};
    #[test]
    fn integration_test_replace_should_fail_if_not_running() {
        test_without_initialization(|_currency_id| {
            SecurityPallet::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(OLD_VAULT))),
                SecurityError::ParachainShutdown,
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_at_capacity_succeeds() {
        test_with(|currency_id| {
            let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
            let griefing_collateral = griefing(200);

            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));
            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_above_capacity_succeeds() {
        test_with(|currency_id| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) * 2;
            let griefing_collateral = griefing(200);

            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));

            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount / 2;
                    old_vault.griefing_collateral += griefing_collateral / 2;
                    old_vault.replace_collateral += griefing_collateral / 2;
                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() -= griefing_collateral / 2;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_below_capacity_succeeds() {
        test_with(|currency_id| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) / 2;
            let griefing_collateral = griefing(200);

            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));
            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_zero_btc_succeeds() {
        test_with(|currency_id| {
            let amount = wrapped(0);
            let griefing_collateral = griefing(200);

            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_zero_collateral_succeeds() {
        test_with(|currency_id| {
            let amount = wrapped(1000);
            let griefing_collateral = griefing(0);

            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_insufficient_collateral() {
        test_with(|currency_id| {
            let amount = wrapped(1000);

            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: wrapped(5_000),
                    replace_collateral: griefing(1),
                    ..default_vault_state(currency_id)
                },
            );

            // check that failing to lock sufficient collateral gives an error
            assert_noop!(
                Call::Replace(ReplaceCall::request_replace(amount.amount(), 0))
                    .dispatch(origin_of(account_of(OLD_VAULT))),
                ReplaceError::InsufficientCollateral
            );

            let pre_request_state = ParachainTwoVaultState::get();

            // check that by locking sufficient collateral we can recover
            let griefing_collateral = griefing(1000);
            assert_ok!(Call::Replace(ReplaceCall::request_replace(
                amount.amount(),
                griefing_collateral.amount()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))));

            assert_eq!(
                ParachainTwoVaultState::get(),
                pre_request_state.with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() -= griefing_collateral;
                })
            );
        });
    }
}

mod withdraw_replace_tests {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_replace_withdraw_replace_at_capacity_succeeds() {
        test_with(|currency_id| {
            let amount = DEFAULT_VAULT_TO_BE_REPLACED;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount.amount()))
                .dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_below_capacity_succeeds() {
        test_with(|currency_id| {
            // withdraw 25%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount.amount()))
                .dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL / 4;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_above_capacity_succeeds() {
        test_with(|currency_id| {
            // withdraw 200% - should just be capped to 100%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED * 2;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount.amount()))
                .dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::get_default(currency_id).with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount / 2;

                    *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }
    #[test]
    fn integration_test_replace_withdraw_replace_with_zero_to_be_replaced_tokens_fails() {
        test_with(|currency_id| {
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: wrapped(0),
                    ..default_vault_state(currency_id)
                },
            );

            assert_noop!(
                Call::Replace(ReplaceCall::withdraw_replace(1000)).dispatch(origin_of(account_of(OLD_VAULT))),
                ReplaceError::NoPendingRequest
            );
        });
    }
}

mod expiry_test {
    use super::{assert_eq, *};

    /// test replace created by accept
    fn test_with(initial_period: u32, execute: impl Fn(H256)) {
        let amount_btc = wrapped(5_000);
        let griefing_collateral = griefing(1000);
        super::test_with(|_currency_id| {
            set_replace_period(initial_period);
            let (replace_id, _replace) = accept_replace(amount_btc, griefing_collateral);
            execute(replace_id);
        });
    }

    fn set_replace_period(period: u32) {
        assert_ok!(Call::Replace(ReplaceCall::set_replace_period(period)).dispatch(root()));
    }

    fn cancel_replace(replace_id: H256) -> DispatchResultWithPostInfo {
        Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(NEW_VAULT)))
    }

    #[test]
    fn integration_test_replace_expiry_only_parachain_blocks_expired() {
        test_with(1000, |replace_id| {
            mine_blocks(1);
            SecurityPallet::set_active_block_number(1500);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_only_bitcoin_blocks_expired() {
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(500);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_pre_expiry() {
        test_with(1000, |replace_id| {
            mine_blocks(7);
            SecurityPallet::set_active_block_number(750);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_post_expiry() {
        // can still execute after expiry
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(execute_replace(replace_id));
        });

        // but new-vault can also cancel.. whoever is first wins
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);

            assert_ok!(cancel_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_decrease() {
        test_with(2000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(1000);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_increase() {
        test_with(1000, |replace_id| {
            mine_blocks(15);
            SecurityPallet::set_active_block_number(1100);
            set_replace_period(2000);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }
}

mod execute_replace_payment_limits {
    use super::{assert_eq, *};

    #[test]
    fn integration_test_execute_replace_with_exact_amount_succeeds() {
        test_with(|_currency_id| {
            let (replace_id, replace) = accept_replace(wrapped(1000000), DEFAULT_GRIEFING_COLLATERAL);
            assert_ok!(execute_replace_with_amount(replace_id, replace.amount()));
        });
    }
    #[test]
    fn integration_test_execute_replace_with_overpayment_fails() {
        test_with(|_currency_id| {
            let (replace_id, replace) = accept_replace(wrapped(1000000), DEFAULT_GRIEFING_COLLATERAL);
            assert_err!(
                execute_replace_with_amount(replace_id, replace.amount().with_amount(|x| x + 1)),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
    #[test]
    fn integration_test_execute_replace_with_underpayment_fails() {
        test_with(|_currency_id| {
            let (replace_id, replace) = accept_replace(wrapped(1000000), DEFAULT_GRIEFING_COLLATERAL);
            assert_err!(
                execute_replace_with_amount(replace_id, replace.amount().with_amount(|x| x - 1)),
                BTCRelayError::InvalidPaymentAmount
            );
        });
    }
}

#[test]
fn integration_test_replace_with_parachain_shutdown_fails() {
    test_with(|_currency_id| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(OLD_VAULT))),
            SecurityError::ParachainShutdown,
        );
        assert_noop!(
            Call::Replace(ReplaceCall::withdraw_replace(0,)).dispatch(origin_of(account_of(OLD_VAULT))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Replace(ReplaceCall::accept_replace(
                Default::default(),
                0,
                0,
                Default::default()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))),
            SecurityError::ParachainShutdown
        );

        assert_noop!(
            Call::Replace(ReplaceCall::execute_replace(
                Default::default(),
                Default::default(),
                Default::default()
            ))
            .dispatch(origin_of(account_of(OLD_VAULT))),
            SecurityError::ParachainShutdown
        );

        assert_noop!(
            Call::Replace(ReplaceCall::cancel_replace(Default::default())).dispatch(origin_of(account_of(OLD_VAULT))),
            SecurityError::ParachainShutdown
        );
    })
}

#[test]
fn integration_test_replace_execute_replace() {
    test_without_initialization(|currency_id| {
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
        set_default_thresholds();
        SecurityPallet::set_active_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let griefing_collateral = griefing(500);
        let collateral = Amount::new(4_000, currency_id);
        let issued_tokens = wrapped(1_000);

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, issued_tokens);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral.amount(),
            dummy_public_key(),
            currency_id
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(Call::Replace(ReplaceCall::request_replace(
            issued_tokens.amount(),
            griefing_collateral.amount()
        ))
        .dispatch(origin_of(account_of(old_vault))));

        assert_request_event();

        // alice accepts bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            account_of(old_vault),
            issued_tokens.amount(),
            collateral.amount(),
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        let replace_id = assert_accept_event();

        // send the btc from the old_vault to the new_vault
        let (_tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(new_vault_btc_address, issued_tokens, Some(replace_id), None);

        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);
        let r = Call::Replace(ReplaceCall::execute_replace(replace_id, merkle_proof, raw_tx))
            .dispatch(origin_of(account_of(old_vault)));
        assert_ok!(r);
    });
}

#[test]
fn integration_test_replace_cancel_replace() {
    test_without_initialization(|currency_id| {
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
        set_default_thresholds();
        SecurityPallet::set_active_block_number(1);

        let amount = wrapped(1000);
        //FIXME: get this from storage
        let griefing_collateral = griefing(200);
        let collateral = amount.convert_to(currency_id).unwrap() * 2;

        // alice creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            amount.amount(),
            dummy_public_key(),
            currency_id
        ))
        .dispatch(origin_of(account_of(ALICE))));
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(Call::Replace(ReplaceCall::request_replace(
            amount.amount(),
            griefing_collateral.amount()
        ))
        .dispatch(origin_of(account_of(BOB))));
        // alice accepts bob's request
        assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            account_of(BOB),
            amount.amount(),
            collateral.amount(),
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let replace_id = assert_accept_event();

        // set block height
        // alice cancels replacement
        mine_blocks(2);
        SecurityPallet::set_active_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(ALICE))));
    });
}

// liquidation tests..

fn setup_replace(issued_tokens: Amount<Runtime>) -> (ReplaceRequest<AccountId32, u32, u128>, H256) {
    // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
    let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

    assert_ok!(Call::Replace(ReplaceCall::request_replace(
        issued_tokens.amount(),
        DEFAULT_GRIEFING_COLLATERAL.amount()
    ))
    .dispatch(origin_of(account_of(OLD_VAULT))));

    assert_request_event();

    // alice accepts bob's request
    assert_ok!(Call::Replace(ReplaceCall::accept_replace(
        account_of(OLD_VAULT),
        issued_tokens.amount(),
        0,
        new_vault_btc_address
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    let replace_id = assert_accept_event();
    let replace = ReplacePallet::get_open_replace_request(&replace_id).unwrap();
    (replace, replace_id)
}

fn execute_replace(replace_id: H256) -> DispatchResultWithPostInfo {
    let replace = ReplacePallet::get_open_replace_request(&replace_id).unwrap();
    execute_replace_with_amount(replace_id, replace.amount())
}

fn execute_replace_with_amount(replace_id: H256, amount: Amount<Runtime>) -> DispatchResultWithPostInfo {
    let replace = ReplacePallet::get_open_replace_request(&replace_id).unwrap();

    // send the btc from the old_vault to the new_vault
    let (_tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(replace.btc_address, amount, Some(replace_id), None);

    SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

    Call::Replace(ReplaceCall::execute_replace(replace_id, merkle_proof, raw_tx))
        .dispatch(origin_of(account_of(OLD_VAULT)))
}

fn cancel_replace(replace_id: H256) {
    // set block height
    // alice cancels replacement
    mine_blocks(2);
    SecurityPallet::set_active_block_number(30);
    assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(NEW_VAULT))));
}

#[test]
fn integration_test_replace_execute_replace_success() {
    test_with(|_currency_id| {
        let (replace, replace_id) = setup_replace(wrapped(1000));

        let pre_execute_state = ParachainTwoVaultState::get();

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= wrapped(1000);
                new_vault.issued += wrapped(1000);
                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.issued -= wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}

fn calculate_replace_collateral(
    vault_data: &CoreVaultData,
    replace_amount: Amount<Runtime>,
    currency_id: CurrencyId,
) -> Amount<Runtime> {
    Amount::new(
        (vault_data.backing_collateral.amount() * replace_amount.amount())
            / (vault_data.issued + vault_data.to_be_issued).amount(),
        currency_id,
    )
}

#[test]
fn integration_test_replace_execute_replace_old_vault_liquidated() {
    test_with(|currency_id| {
        let replace_tokens = wrapped(1000);
        let (replace, replace_id) = setup_replace(replace_tokens);

        let old = CoreVaultData::vault(OLD_VAULT);

        liquidate_vault(currency_id, OLD_VAULT);

        let pre_execution_state = ParachainTwoVaultState::get();

        assert_ok!(execute_replace(replace_id));

        let collateral_for_replace = calculate_replace_collateral(&old, replace.amount(), currency_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_execution_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                liquidation_vault.issued -= wrapped(1000);
                liquidation_vault.to_be_redeemed -= wrapped(1000);

                new_vault.to_be_issued -= wrapped(1000);
                new_vault.issued += wrapped(1000);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.liquidated_collateral -= collateral_for_replace;
                old_vault.backing_collateral += collateral_for_replace; // TODO: probably should be free
                *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_new_vault_liquidated() {
    test_with(|currency_id| {
        let replace_tokens = wrapped(1000);
        let (replace, replace_id) = setup_replace(replace_tokens);

        liquidate_vault(currency_id, NEW_VAULT);
        let pre_execution_state = ParachainTwoVaultState::get();

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                liquidation_vault.to_be_issued -= wrapped(1000);
                liquidation_vault.issued += wrapped(1000);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.issued -= wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_both_vaults_liquidated() {
    test_with(|currency_id| {
        let replace_tokens = wrapped(1000);
        let (replace, replace_id) = setup_replace(replace_tokens);

        let old = CoreVaultData::vault(OLD_VAULT);

        liquidate_vault(currency_id, OLD_VAULT);
        liquidate_vault(currency_id, NEW_VAULT);

        let pre_execution_state = ParachainTwoVaultState::get();

        assert_ok!(execute_replace(replace_id));

        let collateral_for_replace = calculate_replace_collateral(&old, replace.amount(), currency_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_execution_state.with_changes(|old_vault, _new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                liquidation_vault.to_be_redeemed -= wrapped(1000);
                liquidation_vault.to_be_issued -= wrapped(1000);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.liquidated_collateral -= collateral_for_replace;
                old_vault.backing_collateral += collateral_for_replace; // TODO: probably should be free
                *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_success() {
    test_with(|currency_id| {
        let (replace, replace_id) = setup_replace(wrapped(1000));

        let pre_cancellation_state = ParachainTwoVaultState::get();

        cancel_replace(replace_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_cancellation_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= wrapped(1000);
                new_vault.backing_collateral -= replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&currency_id).unwrap() += replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                old_vault.to_be_redeemed -= wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_old_vault_liquidated() {
    test_with(|currency_id| {
        let (replace, replace_id) = setup_replace(wrapped(1000));

        let old = CoreVaultData::vault(OLD_VAULT);

        liquidate_vault(currency_id, OLD_VAULT);

        let pre_cancellation_state = ParachainTwoVaultState::get();

        cancel_replace(replace_id);

        let collateral_for_replace = calculate_replace_collateral(&old, replace.amount(), currency_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();
                old_vault.liquidated_collateral -= collateral_for_replace;

                new_vault.to_be_issued -= wrapped(1000);
                new_vault.backing_collateral -= replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&currency_id).unwrap() += replace.collateral().unwrap();
                *new_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                liquidation_vault.to_be_redeemed -= wrapped(1000);
                liquidation_vault.collateral += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_new_vault_liquidated() {
    test_with(|currency_id| {
        let (replace, replace_id) = setup_replace(wrapped(1000));

        liquidate_vault(currency_id, NEW_VAULT);

        let pre_cancellation_state = ParachainTwoVaultState::get();

        cancel_replace(replace_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();

                *new_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                liquidation_vault.to_be_issued -= wrapped(1000);
            })
        );
    });
}

#[test]
fn integration_test_replace_cancel_replace_both_vaults_liquidated() {
    test_with(|currency_id| {
        let (replace, replace_id) = setup_replace(wrapped(1000));

        let old = CoreVaultData::vault(OLD_VAULT);

        liquidate_vault(currency_id, OLD_VAULT);
        liquidate_vault(currency_id, NEW_VAULT);

        let pre_cancellation_state = ParachainTwoVaultState::get();

        cancel_replace(replace_id);

        let collateral_for_replace = calculate_replace_collateral(&old, replace.amount(), currency_id);

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_cancellation_state.with_changes(|old_vault, new_vault, liquidation_vault| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.griefing_collateral -= replace.griefing_collateral();
                old_vault.liquidated_collateral -= collateral_for_replace;

                *new_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();

                liquidation_vault.to_be_redeemed -= wrapped(1000);
                liquidation_vault.to_be_issued -= wrapped(1000);
                liquidation_vault.collateral += collateral_for_replace;
            })
        );
    });
}

#[test]
fn integration_test_issue_using_griefing_collateral_fails() {
    test_without_initialization(|currency_id| {
        assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
        set_default_thresholds();
        SecurityPallet::set_active_block_number(1);

        let amount = wrapped(1000);
        let collateral = amount.convert_to(currency_id).unwrap() * 2;
        let issue_amount = amount * 10;
        let griefing_collateral = griefing(1_000_000);
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(
                1000,
                account_of(OLD_VAULT),
                issue_amount.amount()
            ))
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );

        assert_ok!(Call::Replace(ReplaceCall::request_replace(
            amount.amount(),
            griefing_collateral.amount()
        ))
        .dispatch(origin_of(account_of(OLD_VAULT))));

        // still can't do the issue, even though the vault locked griefing collateral
        assert_noop!(
            Call::Issue(IssueCall::request_issue(
                1000,
                account_of(OLD_VAULT),
                issue_amount.amount()
            ))
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );
    });
}

#[test]
fn integration_test_replace_vault_with_different_currency_succeeds() {
    test_without_initialization(|currency_id| {
        for currency_id in iter_collateral_currencies() {
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
        }
        set_default_thresholds();
        SecurityPallet::set_active_block_number(1);

        let other_currency = if let CurrencyId::DOT = currency_id {
            CurrencyId::KSM
        } else {
            CurrencyId::DOT
        };

        CoreVaultData::force_to(OLD_VAULT, default_vault_state(currency_id));
        CoreVaultData::force_to(NEW_VAULT, default_vault_state(other_currency));

        let (replace, replace_id) = setup_replace(wrapped(1000));

        let pre_execute_state = ParachainTwoVaultState::get();

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            ParachainTwoVaultState::get(),
            pre_execute_state.with_changes(|old_vault, new_vault, _| {
                new_vault.to_be_issued -= wrapped(1000);
                new_vault.issued += wrapped(1000);
                old_vault.to_be_redeemed -= wrapped(1000);
                old_vault.issued -= wrapped(1000);

                old_vault.griefing_collateral -= replace.griefing_collateral();
                *old_vault.free_balance.get_mut(&GRIEFING_CURRENCY).unwrap() += replace.griefing_collateral();
            })
        );
    });
}
