mod mock;

use mock::*;

use primitive_types::H256;

type IssueCall = issue::Call<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;
pub type ReplaceError = replace::Error<Runtime>;

const USER: [u8; 32] = ALICE;
const OLD_VAULT: [u8; 32] = BOB;
const NEW_VAULT: [u8; 32] = CAROL;
pub const DEFAULT_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_GRIEFING_COLLATERAL: u128 = 5_000;

fn test_with<R>(execute: impl FnOnce() -> R) -> R {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        UserData::force_to(USER, default_user_state());
        CoreVaultData::force_to(OLD_VAULT, default_vault_state());
        CoreVaultData::force_to(NEW_VAULT, default_vault_state());
        execute()
    })
}

fn assert_request_event() {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::RequestReplace(_, _, _)) => Some(()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 1);
}

pub fn assert_accept_event() -> H256 {
    SystemModule::events()
        .iter()
        .rev()
        .find_map(|record| match record.event {
            Event::replace(ReplaceEvent::AcceptReplace(id, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .unwrap()
}

pub fn assert_auction_event() -> H256 {
    SystemModule::events()
        .iter()
        .rev()
        .find_map(|record| match record.event {
            Event::replace(ReplaceEvent::AuctionReplace(id, _, _, _, _, _, _, _)) => Some(id),
            _ => None,
        })
        .unwrap()
}

fn accept_replace(amount_btc: u128, griefing_collateral: u128) -> (H256, ReplaceRequest<AccountId32, u32, u128, u128>) {
    assert_ok!(Call::Replace(ReplaceCall::accept_replace(
        account_of(OLD_VAULT),
        amount_btc,
        griefing_collateral,
        BtcAddress::P2PKH(H160([1; 20]))
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    let replace_id = assert_accept_event();
    let replace = ReplaceModule::get_open_replace_request(&replace_id).unwrap();
    (replace_id, replace)
}

fn auction_replace(
    amount_btc: u128,
    griefing_collateral: u128,
) -> (H256, ReplaceRequest<AccountId32, u32, u128, u128>) {
    assert_ok!(Call::Replace(ReplaceCall::auction_replace(
        account_of(OLD_VAULT),
        amount_btc,
        griefing_collateral,
        BtcAddress::P2PKH(H160([1; 20]))
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    let replace_id = assert_auction_event();
    let replace = ReplaceModule::get_open_replace_request(&replace_id).unwrap();
    (replace_id, replace)
}

#[cfg(test)]
mod accept_replace_tests {
    use super::*;

    fn assert_state_after_accept_replace_correct(replace: &ReplaceRequest<AccountId32, u32, u128, u128>) {
        assert_eq!(
            ParachainTwoVaultState::get(),
            ParachainTwoVaultState::default().with_changes(|old_vault, new_vault, _| {
                new_vault.free_balance -= replace.collateral;
                new_vault.backing_collateral += replace.collateral;

                old_vault.replace_collateral -=
                    (old_vault.replace_collateral * replace.amount) / old_vault.to_be_replaced;
                old_vault.to_be_replaced -= replace.amount;

                old_vault.to_be_redeemed += replace.amount;
                new_vault.to_be_issued += replace.amount;
            })
        );
    }

    #[test]
    fn integration_test_replace_accept_replace_at_capacity() {
        test_with(|| {
            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, accept_amount);
            assert_eq!(replace.collateral, new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral, DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_capacity() {
        test_with(|| {
            // accept only 25%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, accept_amount);
            assert_eq!(replace.collateral, new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral, DEFAULT_VAULT_REPLACE_COLLATERAL / 4);

            assert_state_after_accept_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_above_capacity() {
        test_with(|| {
            // try to accept 400%

            let accept_amount = DEFAULT_VAULT_TO_BE_REPLACED * 4;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = accept_replace(accept_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, accept_amount / 4);
            assert_eq!(replace.collateral, new_vault_additional_collateral / 4);
            assert_eq!(replace.griefing_collateral, DEFAULT_VAULT_REPLACE_COLLATERAL);

            assert_state_after_accept_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_by_vault_that_does_not_accept_issues_succeeds() {
        test_with(|| {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::accept_new_issues(false))
                .dispatch(origin_of(account_of(NEW_VAULT))));

            let (_, replace) = accept_replace(1000, 1000);

            assert_state_after_accept_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_accept_replace_below_dust_fails() {
        test_with(|| {
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
                    to_be_replaced: 1,
                    ..default_vault_state()
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
}

#[cfg(test)]
mod auction_replace_tests {
    use super::*;

    fn test_with<R>(execute: impl FnOnce() -> R) -> R {
        super::test_with(|| {
            VaultRegistryModule::set_auction_collateral_threshold(FixedU128::from(10_000));
            execute()
        })
    }

    fn assert_state_after_auction_replace_correct(replace: &ReplaceRequest<AccountId32, u32, u128, u128>) {
        assert_eq!(
            ParachainTwoVaultState::get(),
            ParachainTwoVaultState::default().with_changes(|old_vault, new_vault, _| {
                new_vault.free_balance -= replace.collateral;
                new_vault.backing_collateral += replace.collateral;

                // new-vault gets the reward
                let reward = FeeModule::get_auction_redeem_fee(replace.amount).unwrap();
                new_vault.backing_collateral += reward;
                old_vault.backing_collateral -= reward;

                // to_be_replaced & replace_collateral decreases
                let used_to_be_replaced_tokens = replace.amount.min(old_vault.to_be_replaced);
                let used_collateral =
                    (old_vault.replace_collateral * used_to_be_replaced_tokens) / old_vault.to_be_replaced;
                old_vault.to_be_replaced -= used_to_be_replaced_tokens;
                // since griefing_collateral includes replace_collateral, both decrease
                old_vault.replace_collateral -= used_collateral;
                old_vault.griefing_collateral -= used_collateral;

                // the used collateral gets unlocked
                old_vault.free_balance += used_collateral;

                old_vault.to_be_redeemed += replace.amount;
                new_vault.to_be_issued += replace.amount;
            })
        );
    }

    #[test]
    fn integration_test_replace_auction_replace_at_capacity() {
        test_with(|| {
            // auction 100% of to-be-replaced
            let auction_amount = DEFAULT_VAULT_TO_BE_REPLACED;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = auction_replace(auction_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, auction_amount);
            assert_eq!(replace.collateral, new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral, 0);

            assert_state_after_auction_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_auction_replace_below_capacity() {
        test_with(|| {
            // auction 25% of to-be-replaced

            let auction_amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = auction_replace(auction_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, auction_amount);
            assert_eq!(replace.collateral, new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral, 0);

            assert_state_after_auction_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_auction_replace_above_requested_capacity() {
        test_with(|| {
            // try 200% of to-be-replaced, but still below total issued tokens

            let auction_amount = DEFAULT_VAULT_TO_BE_REPLACED * 2;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = auction_replace(auction_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, auction_amount);
            assert_eq!(replace.collateral, new_vault_additional_collateral);
            assert_eq!(replace.griefing_collateral, 0);

            assert_state_after_auction_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_auction_replace_above_auctionable_capacity() {
        test_with(|| {
            // try 200% of auctionable tokens

            let auction_amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED) * 2;
            let new_vault_additional_collateral = 10_000;

            let (_, replace) = auction_replace(auction_amount, new_vault_additional_collateral);

            assert_eq!(replace.amount, auction_amount / 2);
            assert_eq!(replace.collateral, new_vault_additional_collateral / 2);
            assert_eq!(replace.griefing_collateral, 0);

            assert_state_after_auction_replace_correct(&replace);
        });
    }

    #[test]
    fn integration_test_replace_auction_replace_below_dust() {
        test_with(|| {
            // if the new_vault _asks_ for an amount below below DUST, it gets rejected

            assert_noop!(
                Call::Replace(ReplaceCall::auction_replace(
                    account_of(OLD_VAULT),
                    1,
                    10_000,
                    BtcAddress::P2PKH(H160([1; 20]))
                ))
                .dispatch(origin_of(account_of(NEW_VAULT))),
                ReplaceError::AmountBelowDustAmount
            );

            // check that auction fails if the old_vault does not have sufficient issued tokens
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    issued: 1,
                    backing_collateral: 100,
                    to_be_redeemed: 0,
                    to_be_replaced: 0,
                    ..default_vault_state()
                },
            );
            assert_noop!(
                Call::Replace(ReplaceCall::auction_replace(
                    account_of(OLD_VAULT),
                    1000,
                    10_000,
                    BtcAddress::P2PKH(H160([1; 20]))
                ))
                .dispatch(origin_of(account_of(NEW_VAULT))),
                ReplaceError::AmountBelowDustAmount
            );

            // check that auction _does_ succeed if to_be_replaced is low, but the issued tokens is sufficient
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: 1,
                    ..default_vault_state()
                },
            );
            assert_ok!(Call::Replace(ReplaceCall::auction_replace(
                account_of(OLD_VAULT),
                1000,
                10_000,
                BtcAddress::P2PKH(H160([1; 20]))
            ))
            .dispatch(origin_of(account_of(NEW_VAULT))));
        });
    }
}

mod request_replace_tests {
    use super::*;
    #[test]
    fn integration_test_replace_should_fail_if_not_running() {
        ExtBuilder::build().execute_with(|| {
            SecurityModule::set_status(StatusCode::Shutdown);

            assert_noop!(
                Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(OLD_VAULT))),
                SecurityError::ParachainNotRunning,
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_at_capacity_succeeds() {
        test_with(|| {
            let amount = DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED;
            let griefing_collateral = 200;

            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));
            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    old_vault.free_balance -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_above_capacity_succeeds() {
        test_with(|| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) * 2;
            let griefing_collateral = 200;

            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));

            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount / 2;
                    old_vault.griefing_collateral += griefing_collateral / 2;
                    old_vault.replace_collateral += griefing_collateral / 2;
                    old_vault.free_balance -= griefing_collateral / 2;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_below_capacity_succeeds() {
        test_with(|| {
            let amount = (DEFAULT_VAULT_ISSUED - DEFAULT_VAULT_TO_BE_REDEEMED - DEFAULT_VAULT_TO_BE_REPLACED) / 2;
            let griefing_collateral = 200;

            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));
            // assert request event
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    old_vault.free_balance -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_zero_btc_succeeds() {
        test_with(|| {
            let amount = 0;
            let griefing_collateral = 200;

            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    old_vault.free_balance -= griefing_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_zero_collateral_succeeds() {
        test_with(|| {
            let amount = 1000;
            let griefing_collateral = 0;

            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));
            let _request_id = assert_request_event();

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_request_replace_with_insufficient_collateral() {
        test_with(|| {
            let amount = 1000;

            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: 5_000,
                    replace_collateral: 1,
                    ..default_vault_state()
                },
            );

            // check that failing to lock sufficient collateral gives an error
            assert_noop!(
                Call::Replace(ReplaceCall::request_replace(amount, 0)).dispatch(origin_of(account_of(OLD_VAULT))),
                ReplaceError::InsufficientCollateral
            );

            let pre_request_state = ParachainTwoVaultState::get();

            // check that by locking sufficient collateral we can recover
            let griefing_collateral = 1000;
            assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT))));

            assert_eq!(
                ParachainTwoVaultState::get(),
                pre_request_state.with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced += amount;
                    old_vault.griefing_collateral += griefing_collateral;
                    old_vault.replace_collateral += griefing_collateral;
                    old_vault.free_balance -= griefing_collateral;
                })
            );
        });
    }
}

mod withdraw_replace_tests {
    use super::*;

    #[test]
    fn integration_test_replace_withdraw_replace_at_capacity_succeeds() {
        test_with(|| {
            let amount = DEFAULT_VAULT_TO_BE_REPLACED;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount)).dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    old_vault.free_balance += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_below_capacity_succeeds() {
        test_with(|| {
            // withdraw 25%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED / 4;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount)).dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL / 4;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount;

                    old_vault.free_balance += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }

    #[test]
    fn integration_test_replace_withdraw_replace_above_capacity_succeeds() {
        test_with(|| {
            // withdraw 200% - should just be capped to 100%
            let amount = DEFAULT_VAULT_TO_BE_REPLACED * 2;

            assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(amount)).dispatch(origin_of(account_of(OLD_VAULT))));

            let released_collateral = DEFAULT_VAULT_REPLACE_COLLATERAL;

            assert_eq!(
                ParachainTwoVaultState::get(),
                ParachainTwoVaultState::default().with_changes(|old_vault, _, _| {
                    old_vault.to_be_replaced -= amount / 2;

                    old_vault.free_balance += released_collateral;
                    old_vault.griefing_collateral -= released_collateral;
                    old_vault.replace_collateral -= released_collateral;
                })
            );
        });
    }
    #[test]
    fn integration_test_replace_withdraw_replace_with_zero_to_be_replaced_tokens_fails() {
        test_with(|| {
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    to_be_replaced: 0,
                    ..default_vault_state()
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
    use super::*;

    /// test replace created by accept and auction
    fn test_with(initial_period: u32, execute: impl Fn(H256) -> ()) {
        let amount_btc = 5_000;
        let griefing_collateral = 1000;
        super::test_with(|| {
            set_replace_period(initial_period);
            let (replace_id, _replace) = accept_replace(amount_btc, griefing_collateral);
            execute(replace_id);
        });

        super::test_with(|| {
            set_replace_period(initial_period);
            CoreVaultData::force_to(
                OLD_VAULT,
                CoreVaultData {
                    backing_collateral: 1000,
                    ..default_vault_state()
                },
            );
            let (replace_id, _replace) = auction_replace(amount_btc, griefing_collateral);
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
    fn integration_test_replace_expiry_no_period_change_pre_expiry() {
        test_with(100, |replace_id| {
            SecurityModule::set_active_block_number(75);

            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_no_period_change_post_expiry() {
        test_with(100, |replace_id| {
            SecurityModule::set_active_block_number(110);

            assert_err!(execute_replace(replace_id), ReplaceError::ReplacePeriodExpired);
            assert_ok!(cancel_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_decrease() {
        test_with(200, |replace_id| {
            SecurityModule::set_active_block_number(110);
            set_replace_period(100);

            // request still uses period = 200, so cancel fails and execute succeeds
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }

    #[test]
    fn integration_test_replace_expiry_with_period_increase() {
        test_with(100, |replace_id| {
            SecurityModule::set_active_block_number(110);
            set_replace_period(200);

            // request uses period = 200, so execute succeeds and cancel fails
            assert_err!(cancel_replace(replace_id), ReplaceError::ReplacePeriodNotExpired);
            assert_ok!(execute_replace(replace_id));
        });
    }
}

#[test]
fn integration_test_replace_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
                .dispatch(origin_of(account_of(new_vault)))
        );
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            polkabtc,
            replace_collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        let final_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // auction fee is taken from old vault collateral
        let replace_amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let auction_fee = FeeModule::get_auction_redeem_fee(replace_amount_dot).unwrap();
        assert_eq!(final_old_vault_collateral, initial_old_vault_collateral - auction_fee);
    });
}

#[test]
fn integration_test_replace_execute_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let griefing_collateral = 500;
        let collateral = 4_000;
        let polkabtc = 1_000;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
                .dispatch(origin_of(account_of(new_vault)))
        );

        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(polkabtc, griefing_collateral))
                .dispatch(origin_of(account_of(old_vault)))
        );

        assert_request_event();

        // alice accepts bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            account_of(old_vault),
            polkabtc,
            collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        let replace_id = assert_accept_event();

        // send the btc from the old_vault to the new_vault
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(new_vault_btc_address, polkabtc, Some(replace_id));

        SecurityModule::set_active_block_number(1 + CONFIRMATIONS);
        let r = Call::Replace(ReplaceCall::execute_replace(replace_id, tx_id, merkle_proof, raw_tx))
            .dispatch(origin_of(account_of(old_vault)));
        assert_ok!(r);
    });
}

#[test]
fn integration_test_replace_cancel_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);

        let amount = 1000;
        //FIXME: get this from storage
        let griefing_collateral = 200;
        let collateral = amount * 2;

        // alice creates a vault
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(amount, dummy_public_key()))
                .dispatch(origin_of(account_of(ALICE)))
        );
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
            .dispatch(origin_of(account_of(BOB))));
        // alice accepts bob's request
        assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            account_of(BOB),
            amount,
            collateral,
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(ALICE))));

        let replace_id = assert_accept_event();

        // set block height
        // alice cancels replacement
        SecurityModule::set_active_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(ALICE))));
    });
}

#[test]
fn integration_test_replace_cancel_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);
        let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
                .dispatch(origin_of(account_of(new_vault)))
        );

        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_new_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault));
        let initial_old_vault_collateral =
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            polkabtc,
            replace_collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // check old vault collateral
        let replace_amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let auction_fee = FeeModule::get_auction_redeem_fee(replace_amount_dot).unwrap();
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault)),
            initial_old_vault_collateral - auction_fee
        );
        // check new vault collateral
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault)),
            initial_new_vault_collateral + auction_fee + replace_collateral
        );

        let replace_id = assert_auction_event();

        SecurityModule::set_active_block_number(30);

        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(BOB))));

        // check old vault collateral
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault)),
            initial_old_vault_collateral - auction_fee
        );

        // check new vault collateral. It should have received auction fee, griefing collateral and
        // the collateral that was reserved for this replace should have been released
        assert_eq!(
            collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault)),
            initial_new_vault_collateral + auction_fee
        );
    });
}

#[test]
fn integration_test_replace_cancel_repeatedly_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);

        let user = CAROL;
        let old_vault = ALICE;
        let new_vault = BOB;
        let polkabtc = 1_000;
        let collateral = required_collateral_for_issue(polkabtc);
        let replace_collateral = collateral * 2;

        // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
        let new_vault_btc_address1 = BtcAddress::P2PKH(H160([2; 20]));
        let new_vault_btc_address2 = BtcAddress::P2PKH(H160([3; 20]));
        let new_vault_btc_address3 = BtcAddress::P2PKH(H160([4; 20]));
        let new_vault_btc_address4 = BtcAddress::P2PKH(H160([5; 20]));

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
                .dispatch(origin_of(account_of(new_vault)))
        );
        // exchange rate drops and vault is not collateralized any more
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        // let initial_new_vault_collateral =
        //     collateral::Module::<Runtime>::get_collateral_from_account(&account_of(new_vault));
        // let initial_old_vault_collateral =
        //     collateral::Module::<Runtime>::get_collateral_from_account(&account_of(old_vault));

        // new_vault takes over old_vault's position
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            750,
            replace_collateral,
            new_vault_btc_address1
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            200,
            replace_collateral,
            new_vault_btc_address2
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // old_vault at this point only has 50 satoshi left, so this should fail
        // TODO: change back to assert_noop
        assert_ok!(Call::Replace(ReplaceCall::auction_replace(
            account_of(old_vault),
            200,
            replace_collateral,
            new_vault_btc_address3
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // old_vault at this point only has 50 satoshi left. This should partially succeed
        // TODO: change back to assert_noop
        assert_noop!(
            Call::Replace(ReplaceCall::auction_replace(
                account_of(old_vault),
                200,
                replace_collateral,
                new_vault_btc_address4
            ))
            .dispatch(origin_of(account_of(new_vault))),
            ReplaceError::AmountBelowDustAmount
        );
    });
}

// liquidation tests..

fn setup_replace(polkabtc: u128) -> H256 {
    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
    set_default_thresholds();
    SecurityModule::set_active_block_number(1);

    // burn surplus free balance to make checking easier
    CollateralModule::transfer(
        account_of(OLD_VAULT),
        account_of(FAUCET),
        CollateralModule::get_balance_from_account(&account_of(OLD_VAULT))
            - DEFAULT_COLLATERAL
            - DEFAULT_GRIEFING_COLLATERAL,
    )
    .unwrap();
    CollateralModule::transfer(
        account_of(NEW_VAULT),
        account_of(FAUCET),
        CollateralModule::get_balance_from_account(&account_of(NEW_VAULT)) - DEFAULT_COLLATERAL,
    )
    .unwrap();

    // let old_vault_btc_address = BtcAddress::P2PKH(H160([1; 20]));
    let new_vault_btc_address = BtcAddress::P2PKH(H160([2; 20]));

    // old vault has issued some tokens with the user
    force_issue_tokens(USER, OLD_VAULT, DEFAULT_COLLATERAL, polkabtc);

    // new vault joins
    assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
        DEFAULT_COLLATERAL / 2, // rest we do in accept_replacec
        dummy_public_key()
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    assert_ok!(
        Call::Replace(ReplaceCall::request_replace(polkabtc, DEFAULT_GRIEFING_COLLATERAL))
            .dispatch(origin_of(account_of(OLD_VAULT)))
    );

    assert_request_event();

    // alice accepts bob's request
    assert_ok!(Call::Replace(ReplaceCall::accept_replace(
        account_of(OLD_VAULT),
        polkabtc,
        DEFAULT_COLLATERAL / 2,
        new_vault_btc_address
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    let replace_id = assert_accept_event();

    replace_id
}

fn execute_replace(replace_id: H256) -> DispatchResultWithPostInfo {
    let replace = ReplaceModule::get_open_replace_request(&replace_id).unwrap();

    // send the btc from the old_vault to the new_vault
    let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(replace.btc_address, replace.amount, Some(replace_id));

    SecurityModule::set_active_block_number(SecurityModule::active_block_number() + CONFIRMATIONS);

    Call::Replace(ReplaceCall::execute_replace(replace_id, tx_id, merkle_proof, raw_tx))
        .dispatch(origin_of(account_of(OLD_VAULT)))
}

fn cancel_replace(replace_id: H256) {
    // set block height
    // alice cancels replacement
    SecurityModule::set_active_block_number(30);
    assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id)).dispatch(origin_of(account_of(NEW_VAULT))));
}
#[test]
fn integration_test_replace_execute_replace_success() {
    ExtBuilder::build().execute_with(|| {
        let replace_id = setup_replace(1000);

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: 1000,
                to_be_redeemed: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        assert_ok!(execute_replace(replace_id));

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                free_balance: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );
    });
}

#[test]
fn integration_test_replace_execute_replace_old_vault_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_tokens = 1000;
        let replace_id = setup_replace(replace_tokens);

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: replace_tokens,
                to_be_redeemed: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            OLD_VAULT,
            CoreVaultData {
                issued: 2500,
                to_be_redeemed: 1250,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(OLD_VAULT);
        assert_ok!(execute_replace(replace_id));

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                to_be_redeemed: 250,
                backing_collateral: (DEFAULT_COLLATERAL * 250) / 2500,
                free_balance: (DEFAULT_COLLATERAL * replace_tokens) / 2500 + DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );
        assert_liquidation_vault_ok(replace_tokens + 500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_replace_execute_replace_new_vault_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_tokens = 1000;
        let replace_id = setup_replace(replace_tokens);
        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: replace_tokens,
                to_be_redeemed: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            NEW_VAULT,
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                issued: 500,         // new
                to_be_redeemed: 150, // new
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(NEW_VAULT);
        assert_ok!(execute_replace(replace_id));

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                free_balance: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                to_be_redeemed: 150,
                backing_collateral: (DEFAULT_COLLATERAL * 150) / (replace_tokens + 500),
                ..Default::default()
            }
        );

        assert_liquidation_vault_ok(replace_tokens + 500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_replace_execute_replace_both_vaults_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_tokens = 1000;
        let replace_id = setup_replace(replace_tokens);
        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: replace_tokens,
                to_be_redeemed: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            OLD_VAULT,
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                issued: replace_tokens + 250,        // new
                to_be_redeemed: replace_tokens + 50, // new
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            NEW_VAULT,
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                issued: 500,         // new
                to_be_redeemed: 150, // new
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(OLD_VAULT);
        drop_exchange_rate_and_liquidate(NEW_VAULT);
        assert_ok!(execute_replace(replace_id));

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                to_be_redeemed: 50,
                backing_collateral: (DEFAULT_COLLATERAL * 50) / (replace_tokens + 250),
                free_balance: (DEFAULT_COLLATERAL * replace_tokens) / (replace_tokens + 250)
                    + DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                to_be_redeemed: 150,
                backing_collateral: (DEFAULT_COLLATERAL * 150) / (replace_tokens + 500),
                ..Default::default()
            }
        );
        assert_liquidation_vault_ok(replace_tokens + 250 + 500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_replace_cancel_replace_success() {
    ExtBuilder::build().execute_with(|| {
        let replace_id = setup_replace(1000);

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: 1000,
                to_be_redeemed: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        cancel_replace(replace_id);

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        // changes: the additional collateral (= DEFAULT_COLLATERAL / 2) that the
        // new-vault locked for this replace gets unlocked. Also it receives the
        // griefing collateral
        assert_eq!(
            old_vault,
            CoreVaultData {
                issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL / 2 + DEFAULT_GRIEFING_COLLATERAL,
                free_balance: DEFAULT_COLLATERAL / 2,
                ..Default::default()
            }
        );
    });
}

fn assert_liquidation_vault_ok(issued: u128, old_vault: &CoreVaultData, new_vault: &CoreVaultData) {
    assert_eq!(
        CoreVaultData::liquidation_vault(),
        CoreVaultData {
            issued,
            to_be_redeemed: old_vault.to_be_redeemed + new_vault.to_be_redeemed,
            backing_collateral: 2 * DEFAULT_COLLATERAL + DEFAULT_GRIEFING_COLLATERAL
                - old_vault.backing_collateral
                - new_vault.backing_collateral
                - old_vault.griefing_collateral
                - new_vault.griefing_collateral
                - old_vault.free_balance
                - new_vault.free_balance,
            free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
            ..Default::default()
        }
    );
}

#[test]
fn integration_test_replace_cancel_replace_old_vault_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_id = setup_replace(1000);

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: 1000,
                to_be_redeemed: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            OLD_VAULT,
            CoreVaultData {
                issued: 2500,
                to_be_redeemed: 1250,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(OLD_VAULT);

        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                to_be_redeemed: 1250,
                backing_collateral: (DEFAULT_COLLATERAL * 1250) / 2500,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );

        cancel_replace(replace_id);

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                to_be_redeemed: 250,
                backing_collateral: (DEFAULT_COLLATERAL * 250) / 2500,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL / 2 + DEFAULT_GRIEFING_COLLATERAL,
                free_balance: DEFAULT_COLLATERAL / 2,
                ..Default::default()
            }
        );
        assert_liquidation_vault_ok(2500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_replace_cancel_replace_new_vault_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_tokens = 1000;
        let replace_id = setup_replace(replace_tokens);
        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: replace_tokens,
                to_be_redeemed: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            NEW_VAULT,
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                issued: 500,         // new
                to_be_redeemed: 150, // new
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(NEW_VAULT);
        cancel_replace(replace_id);

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        // griefing collateral is transfered to liquidation vault
        assert_eq!(
            old_vault,
            CoreVaultData {
                issued: 1000,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                to_be_redeemed: 150,
                backing_collateral: (DEFAULT_COLLATERAL * 150) / (500 + replace_tokens),
                free_balance: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_liquidation_vault_ok(500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_replace_cancel_replace_both_vaults_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let replace_tokens = 1000;
        let replace_id = setup_replace(replace_tokens);
        assert_eq!(
            CoreVaultData::vault(OLD_VAULT),
            CoreVaultData {
                issued: replace_tokens,
                to_be_redeemed: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            },
        );
        assert_eq!(
            CoreVaultData::vault(NEW_VAULT),
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                ..Default::default()
            }
        );

        CoreVaultData::force_to(
            OLD_VAULT,
            CoreVaultData {
                backing_collateral: DEFAULT_COLLATERAL,
                griefing_collateral: DEFAULT_GRIEFING_COLLATERAL,
                issued: replace_tokens + 250,        // new
                to_be_redeemed: replace_tokens + 50, // new
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            NEW_VAULT,
            CoreVaultData {
                to_be_issued: replace_tokens,
                backing_collateral: DEFAULT_COLLATERAL,
                issued: 500,         // new
                to_be_redeemed: 150, // new
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(OLD_VAULT);
        drop_exchange_rate_and_liquidate(NEW_VAULT);
        cancel_replace(replace_id);

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                to_be_redeemed: 50,
                backing_collateral: (DEFAULT_COLLATERAL * 50) / (replace_tokens + 250),
                ..Default::default()
            }
        );
        assert_eq!(
            new_vault,
            CoreVaultData {
                to_be_redeemed: 150,
                backing_collateral: (DEFAULT_COLLATERAL * 150) / (500 + replace_tokens),
                free_balance: DEFAULT_GRIEFING_COLLATERAL,
                ..Default::default()
            }
        );
        assert_liquidation_vault_ok(replace_tokens + 250 + 500, &old_vault, &new_vault);
    });
}

#[test]
fn integration_test_issue_using_griefing_collateral_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        set_default_thresholds();
        SecurityModule::set_active_block_number(1);

        let amount = 1000;
        let collateral = amount * 2;
        let issue_amount = amount * 10;
        let griefing_collateral = 1_000_000;
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(1000, account_of(OLD_VAULT), issue_amount))
                .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );

        assert_ok!(Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
            .dispatch(origin_of(account_of(OLD_VAULT))));

        // still can't do the issue, even though the vault locked griefing collateral
        assert_noop!(
            Call::Issue(IssueCall::request_issue(1000, account_of(OLD_VAULT), issue_amount))
                .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );
    });
}
