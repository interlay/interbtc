mod mock;

use mock::*;

use primitive_types::H256;

type IssueCall = issue::Call<Runtime>;

pub type VaultRegistryError = vault_registry::Error<Runtime>;

const USER: [u8; 32] = ALICE;
const OLD_VAULT: [u8; 32] = BOB;
const NEW_VAULT: [u8; 32] = CAROL;
pub const DEFAULT_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_GRIEFING_COLLATERAL: u128 = 5_000;

// asserts request event happen and extracts its id for further testing
fn assert_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::RequestReplace(id, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

// asserts auction event happen and extracts its id for further testing
fn assert_auction_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::replace(ReplaceEvent::AuctionReplace(id, _, _, _, _, _, _, _, _)) => {
                Some(id.clone())
            }
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

#[test]
fn integration_test_replace_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Replace(ReplaceCall::request_replace(0, 0)).dispatch(origin_of(account_of(BOB))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_replace_request_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        let collateral = amount * 2;
        let griefing_collateral = 200;

        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // assert request event
        let _request_id = assert_request_event();
    });
}

#[test]
fn integration_test_replace_withdraw_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let griefing_collateral = 2000;
        let amount = 50_000;
        let collateral = amount * 2;

        let bob = origin_of(account_of(BOB));

        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(5000, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // bob withdraws his replace
        let replace_id = assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::withdraw_replace(replace_id)).dispatch(bob));
    });
}

#[test]
fn integration_test_replace_accept_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        let griefing_collateral = 500;
        let collateral = amount * 2;

        // alice creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            amount,
            dummy_public_key(),
        ))
        .dispatch(origin_of(account_of(ALICE))));
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        let replace_id = assert_request_event();
        // alice accept bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(ALICE))));
    });
}

#[test]
fn integration_test_replace_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

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
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));
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

        // take auction fee from old vault collateral
        let replace_amount_dot = ExchangeRateOracleModule::btc_to_dots(polkabtc).unwrap();
        let auction_fee = FeeModule::get_auction_redeem_fee(replace_amount_dot).unwrap();
        assert_eq!(
            final_old_vault_collateral,
            initial_old_vault_collateral - auction_fee
        );
    });
}

#[test]
fn integration_test_replace_execute_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

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
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));

        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(polkabtc, griefing_collateral))
                .dispatch(origin_of(account_of(old_vault)))
        );

        let replace_id = assert_request_event();

        // alice accepts bob's request
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            new_vault_btc_address
        ))
        .dispatch(origin_of(account_of(new_vault))));

        // send the btc from the old_vault to the new_vault
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(new_vault_btc_address, polkabtc, Some(replace_id));

        SystemModule::set_block_number(1 + CONFIRMATIONS);
        let r = Call::Replace(ReplaceCall::execute_replace(
            replace_id,
            tx_id,
            merkle_proof,
            raw_tx,
        ))
        .dispatch(origin_of(account_of(old_vault)));
        assert_ok!(r);
    });
}

#[test]
fn integration_test_replace_cancel_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        //FIXME: get this from storage
        let griefing_collateral = 200;
        let collateral = amount * 2;

        // alice creates a vault
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            amount,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(ALICE))));
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);
        // bob requests a replace
        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(BOB)))
        );
        // alice accepts bob's request
        let replace_id = assert_request_event();
        assert_ok!(Call::Replace(ReplaceCall::accept_replace(
            replace_id,
            collateral,
            BtcAddress::P2PKH(H160([1; 20]))
        ))
        .dispatch(origin_of(account_of(BOB))));
        // set block height
        // alice cancels replacement
        SystemModule::set_block_number(30);
        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
            .dispatch(origin_of(account_of(BOB))));
    });
}

#[test]
fn integration_test_replace_cancel_auction_replace() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);
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
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));

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

        SystemModule::set_block_number(30);

        assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
            .dispatch(origin_of(account_of(BOB))));

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
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

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

        // old vault has issued some tokens with the user
        force_issue_tokens(user, old_vault, collateral, polkabtc);

        // new vault joins
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(new_vault))));
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
        assert_noop!(
            Call::Replace(ReplaceCall::auction_replace(
                account_of(old_vault),
                200,
                replace_collateral,
                new_vault_btc_address3
            ))
            .dispatch(origin_of(account_of(new_vault))),
            VaultRegistryError::InsufficientTokensCommitted
        );
    });
}

// liquidation tests..

fn setup_replace(polkabtc: u128) -> H256 {
    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::one()
    ));
    set_default_thresholds();
    SystemModule::set_block_number(1);

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

    assert_ok!(Call::Replace(ReplaceCall::request_replace(
        polkabtc,
        DEFAULT_GRIEFING_COLLATERAL
    ))
    .dispatch(origin_of(account_of(OLD_VAULT))));

    let replace_id = assert_request_event();

    // alice accepts bob's request
    assert_ok!(Call::Replace(ReplaceCall::accept_replace(
        replace_id,
        DEFAULT_COLLATERAL / 2,
        new_vault_btc_address
    ))
    .dispatch(origin_of(account_of(NEW_VAULT))));

    replace_id
}

fn execute_replace(replace_id: H256) {
    let replace = ReplaceModule::get_open_replace_request(&replace_id).unwrap();
    let btc_address = replace.btc_address.unwrap();

    // send the btc from the old_vault to the new_vault
    let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(btc_address, replace.amount, Some(replace_id));

    SystemModule::set_block_number(1 + CONFIRMATIONS);
    assert_ok!(Call::Replace(ReplaceCall::execute_replace(
        replace_id,
        tx_id,
        merkle_proof,
        raw_tx,
    ))
    .dispatch(origin_of(account_of(OLD_VAULT))));
}

fn cancel_replace(replace_id: H256) {
    // set block height
    // alice cancels replacement
    SystemModule::set_block_number(30);
    assert_ok!(Call::Replace(ReplaceCall::cancel_replace(replace_id))
        .dispatch(origin_of(account_of(NEW_VAULT))));
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

        execute_replace(replace_id);

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
        execute_replace(replace_id);

        let old_vault = CoreVaultData::vault(OLD_VAULT);
        let new_vault = CoreVaultData::vault(NEW_VAULT);
        assert_eq!(
            old_vault,
            CoreVaultData {
                to_be_redeemed: 250,
                backing_collateral: (DEFAULT_COLLATERAL * 250) / 2500,
                free_balance: (DEFAULT_COLLATERAL * replace_tokens) / 2500
                    + DEFAULT_GRIEFING_COLLATERAL,
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
        execute_replace(replace_id);

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
        execute_replace(replace_id);

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
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        set_default_thresholds();
        SystemModule::set_block_number(1);

        let amount = 1000;
        let collateral = amount * 2;
        let issue_amount = amount * 10;
        let griefing_collateral = 1_000_000;
        // bob creates a vault
        force_issue_tokens(ALICE, BOB, collateral, amount);

        assert_noop!(
            Call::Issue(IssueCall::request_issue(
                1000,
                account_of(OLD_VAULT),
                issue_amount
            ))
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );

        assert_ok!(
            Call::Replace(ReplaceCall::request_replace(amount, griefing_collateral))
                .dispatch(origin_of(account_of(OLD_VAULT)))
        );

        // still can't do the issue, even though the vault locked griefing collateral
        assert_noop!(
            Call::Issue(IssueCall::request_issue(
                1000,
                account_of(OLD_VAULT),
                issue_amount
            ))
            .dispatch(origin_of(account_of(USER))),
            VaultRegistryError::ExceedingVaultLimit,
        );
    });
}
