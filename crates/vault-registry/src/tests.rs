use crate::ext;
use crate::mock::{
    run_test, CollateralError, Origin, SecurityError, System, Test, TestError, TestEvent,
    VaultRegistry, DEFAULT_COLLATERAL, DEFAULT_ID, MULTI_VAULT_TEST_COLLATERAL,
    MULTI_VAULT_TEST_IDS, OTHER_ID, RICH_COLLATERAL, RICH_ID,
};
use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::OnInitialize;
use crate::types::BtcAddress;
use crate::GRANULARITY;
use crate::{Vault, VaultStatus, Wallet};
use crate::{H160, H256};
use btc_relay::BtcAddress;
use frame_support::{assert_err, assert_noop, assert_ok, StorageMap, StorageValue};
use mocktopus::mocking::*;
use sp_runtime::traits::Header;

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::test_events($event);
        assert_eq!(
            System::events()
                .iter()
                .filter(|a| a.event == test_event)
                .count(),
            $times
        );
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::test_events($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

fn set_default_thresholds() {
    let secure = 200_000; // 200%
    let auction = 150_000; // 150%
    let premium = 120_000; // 120%
    let liquidation = 110_000; // 110%

    VaultRegistry::_set_secure_collateral_threshold(secure);
    VaultRegistry::_set_auction_collateral_threshold(auction);
    VaultRegistry::_set_premium_redeem_threshold(premium);
    VaultRegistry::_set_liquidation_collateral_threshold(liquidation);
}

fn create_vault_with_collateral(
    id: u64,
    collateral: u128,
) -> <Test as frame_system::Trait>::AccountId {
    VaultRegistry::get_minimum_collateral_vault.mock_safe(move || MockResult::Return(collateral));
    let origin = Origin::signed(id);
    let result = VaultRegistry::register_vault(origin, collateral, BtcAddress::random());
    assert_ok!(result);
    id
}

fn create_vault(id: u64) -> <Test as frame_system::Trait>::AccountId {
    create_vault_with_collateral(id, DEFAULT_COLLATERAL)
}

fn create_sample_vault() -> <Test as frame_system::Trait>::AccountId {
    create_vault(DEFAULT_ID)
}

fn create_vault_and_issue_tokens(
    issue_tokens: u128,
    collateral: u128,
    id: u64,
) -> <Test as frame_system::Trait>::AccountId {
    set_default_thresholds();

    // vault has no tokens issued yet
    let id = create_vault_with_collateral(id, collateral);

    // exchange rate 1 Satoshi = 10 Planck (smallest unit of DOT)
    ext::oracle::dots_to_btc::<Test>.mock_safe(move |x| MockResult::Return(Ok((x / 10).into())));

    // issue PolkaBTC with 200% collateralization of DEFAULT_COLLATERAL
    let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
    assert_ok!(
        VaultRegistry::_increase_to_be_issued_tokens(&id, issue_tokens),
        vault.wallet.get_btc_address()
    );
    let res = VaultRegistry::_issue_tokens(&id, issue_tokens);
    assert_ok!(res);

    // mint tokens to the vault
    treasury::Module::<Test>::mint(id, issue_tokens);

    id
}

fn create_sample_vault_and_issue_tokens(
    issue_tokens: u128,
) -> <Test as frame_system::Trait>::AccountId {
    create_vault_and_issue_tokens(issue_tokens, DEFAULT_COLLATERAL, DEFAULT_ID)
}

#[test]
fn register_vault_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        assert_emitted!(Event::RegisterVault(id, DEFAULT_COLLATERAL));
    });
}

#[test]
fn register_vault_fails_when_given_collateral_too_low() {
    run_test(|| {
        VaultRegistry::get_minimum_collateral_vault.mock_safe(|| MockResult::Return(200));
        let id = 3;
        let collateral = 100;
        let result =
            VaultRegistry::register_vault(Origin::signed(id), collateral, BtcAddress::default());
        assert_err!(result, TestError::InsufficientVaultCollateralAmount);
        assert_not_emitted!(Event::RegisterVault(id, collateral));
    });
}

#[test]
fn register_vault_fails_when_account_funds_too_low() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL + 1;
        let result = VaultRegistry::register_vault(
            Origin::signed(DEFAULT_ID),
            collateral,
            BtcAddress::P2PKH(H160([1; 20])),
        );
        assert_err!(result, CollateralError::InsufficientFunds);
        assert_not_emitted!(Event::RegisterVault(DEFAULT_ID, collateral));
    });
}

#[test]
fn register_vault_fails_when_already_registered() {
    run_test(|| {
        let id = create_sample_vault();
        let result = VaultRegistry::register_vault(
            Origin::signed(id),
            DEFAULT_COLLATERAL,
            BtcAddress::default(),
        );
        assert_err!(result, TestError::VaultAlreadyRegistered);
        assert_emitted!(Event::RegisterVault(id, DEFAULT_COLLATERAL), 1);
    });
}

#[test]
fn lock_additional_collateral_succeeds() {
    run_test(|| {
        let id = create_vault(RICH_ID);
        let additional = RICH_COLLATERAL - DEFAULT_COLLATERAL;
        let res = VaultRegistry::lock_additional_collateral(Origin::signed(id), additional);
        assert_ok!(res);
        let new_collateral = ext::collateral::for_account::<Test>(&id);
        assert_eq!(new_collateral, DEFAULT_COLLATERAL + additional);
        assert_emitted!(Event::LockAdditionalCollateral(
            id,
            additional,
            RICH_COLLATERAL,
            RICH_COLLATERAL
        ));
    });
}

#[test]
fn lock_additional_collateral_fails_when_vault_does_not_exist() {
    run_test(|| {
        let res = VaultRegistry::lock_additional_collateral(Origin::signed(3), 50);
        assert_err!(res, TestError::VaultNotFound);
    })
}

#[test]
fn withdraw_collateral_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::withdraw_collateral(Origin::signed(id), 50);
        assert_ok!(res);
        let new_collateral = ext::collateral::for_account::<Test>(&id);
        assert_eq!(new_collateral, DEFAULT_COLLATERAL - 50);
        assert_emitted!(Event::WithdrawCollateral(id, 50, DEFAULT_COLLATERAL - 50));
    });
}

#[test]
fn withdraw_collateral_fails_when_vault_does_not_exist() {
    run_test(|| {
        let res = VaultRegistry::withdraw_collateral(Origin::signed(3), 50);
        assert_err!(res, TestError::VaultNotFound);
    })
}

#[test]
fn withdraw_collateral_fails_when_not_enough_collateral() {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::withdraw_collateral(Origin::signed(id), DEFAULT_COLLATERAL + 1);
        assert_err!(res, CollateralError::InsufficientCollateralAvailable);
    })
}

#[test]
fn increase_to_be_issued_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        set_default_thresholds();
        let res = VaultRegistry::_increase_to_be_issued_tokens(&id, 50);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_ok!(res, vault.wallet.get_btc_address());
        assert_eq!(vault.to_be_issued_tokens, 50);
        assert_emitted!(Event::IncreaseToBeIssuedTokens(id, 50));
    });
}

#[test]
fn increase_to_be_issued_tokens_fails_with_insufficient_collateral() {
    run_test(|| {
        let id = create_sample_vault();
        let vault = VaultRegistry::rich_vault_from_id(&id).unwrap();
        let res =
            VaultRegistry::_increase_to_be_issued_tokens(&id, vault.issuable_tokens().unwrap() + 1);
        assert_err!(res, TestError::ExceedingVaultLimit);
    });
}

#[test]
fn decrease_to_be_issued_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let mut vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        set_default_thresholds();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        let res = VaultRegistry::_decrease_to_be_issued_tokens(&id, 50);
        assert_ok!(res);
        vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.to_be_issued_tokens, 0);
        assert_emitted!(Event::DecreaseToBeIssuedTokens(id, 50));
    });
}

#[test]
fn decrease_to_be_issued_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::_decrease_to_be_issued_tokens(&id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn issue_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let mut vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        set_default_thresholds();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        let res = VaultRegistry::_issue_tokens(&id, 50);
        assert_ok!(res);
        vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.to_be_issued_tokens, 0);
        assert_eq!(vault.issued_tokens, 50);
        assert_emitted!(Event::IssueTokens(id, 50));
    });
}

#[test]
fn issue_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::_issue_tokens(&id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn increase_to_be_redeemed_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let mut vault = VaultRegistry::_get_vault_from_id(&id).unwrap();

        set_default_thresholds();

        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        let res = VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50);
        assert_ok!(res);
        vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 50);
        assert_eq!(vault.to_be_redeemed_tokens, 50);
        assert_emitted!(Event::IncreaseToBeRedeemedTokens(id, 50));
    });
}

#[test]
fn increase_to_be_redeemed_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn decrease_to_be_redeemed_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let mut vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        set_default_thresholds();

        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::_decrease_to_be_redeemed_tokens(&id, 50);
        assert_ok!(res);
        vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 50);
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::DecreaseToBeRedeemedTokens(id, 50));
    });
}

#[test]
fn decrease_to_be_redeemed_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::_decrease_to_be_redeemed_tokens(&id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn decrease_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        set_default_thresholds();
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::_decrease_tokens(&id, &user_id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 0);
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::DecreaseTokens(id, user_id, 50));
    });
}

#[test]
fn decrease_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        set_default_thresholds();
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        let res = VaultRegistry::_decrease_tokens(&id, &user_id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn redeem_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        set_default_thresholds();
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::_redeem_tokens(&id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 0);
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::RedeemTokens(id, 50));
    });
}

#[test]
fn redeem_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        set_default_thresholds();
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        let res = VaultRegistry::_redeem_tokens(&id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn redeem_tokens_premium_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        set_default_thresholds();
        // TODO: emulate assert_called
        ext::collateral::slash::<Test>.mock_safe(move |sender, _receiver, _amount| {
            assert_eq!(sender, &id);
            MockResult::Return(Ok(()))
        });
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::_redeem_tokens_premium(&id, 50, 30, &user_id);
        assert_ok!(res);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 0);
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::RedeemTokensPremium(id, 50, 30, user_id));
    });
}

#[test]
fn redeem_tokens_premium_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        set_default_thresholds();
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        let res = VaultRegistry::_redeem_tokens_premium(&id, 50, 30, &user_id);
        assert_err!(res, TestError::InsufficientTokensCommitted);
        assert_not_emitted!(Event::RedeemTokensPremium(id, 50, 30, user_id));
    });
}

#[test]
fn redeem_tokens_liquidation_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        <crate::LiquidationVault<Test>>::put(id);
        let user_id = 5;
        set_default_thresholds();
        // TODO: emulate assert_called
        ext::collateral::slash::<Test>.mock_safe(move |sender, _receiver, _amount| {
            assert_eq!(sender, &id);
            MockResult::Return(Ok(()))
        });
        ext::security::recover_from_liquidation::<Test>.mock_safe(|| MockResult::Return(Ok(())));
        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 50));
        let res = VaultRegistry::_redeem_tokens_liquidation(&user_id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 0);
        assert_emitted!(Event::RedeemTokensLiquidation(user_id, 50));
    });
}

#[test]
fn redeem_tokens_liquidation_does_not_call_recover_when_unnecessary() {
    run_test(|| {
        let id = create_sample_vault();
        <crate::LiquidationVault<Test>>::put(id);
        let user_id = 5;
        set_default_thresholds();
        ext::collateral::slash::<Test>.mock_safe(move |sender, _receiver, _amount| {
            assert_eq!(sender, &id);
            MockResult::Return(Ok(()))
        });

        ext::security::recover_from_liquidation::<Test>.mock_safe(|| {
            panic!("this should not be called");
        });
        VaultRegistry::_increase_to_be_issued_tokens(&id, 25).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 25));
        let res = VaultRegistry::_redeem_tokens_liquidation(&user_id, 10);
        assert_ok!(res);
        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_eq!(vault.issued_tokens, 15);
        assert_emitted!(Event::RedeemTokensLiquidation(user_id, 10));
    });
}

#[test]
fn redeem_tokens_liquidation_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        <crate::LiquidationVault<Test>>::put(id);
        set_default_thresholds();
        let res = VaultRegistry::_redeem_tokens_liquidation(&user_id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
        assert_not_emitted!(Event::RedeemTokensLiquidation(user_id, 50));
    });
}

#[test]
fn replace_tokens_liquidation_succeeds() {
    run_test(|| {
        let old_id = create_sample_vault();
        let new_id = create_vault(OTHER_ID);
        set_default_thresholds();

        ext::collateral::lock::<Test>.mock_safe(move |sender, amount| {
            assert_eq!(sender, &new_id);
            assert_eq!(amount, 20);
            MockResult::Return(Ok(()))
        });

        VaultRegistry::_increase_to_be_issued_tokens(&old_id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&old_id, 50));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&old_id, 50));

        let res = VaultRegistry::_replace_tokens(&old_id, &new_id, 50, 20);
        assert_ok!(res);
        let old_vault = VaultRegistry::_get_vault_from_id(&old_id).unwrap();
        let new_vault = VaultRegistry::_get_vault_from_id(&new_id).unwrap();
        assert_eq!(old_vault.issued_tokens, 0);
        assert_eq!(old_vault.to_be_redeemed_tokens, 0);
        assert_eq!(new_vault.issued_tokens, 50);
        assert_emitted!(Event::ReplaceTokens(old_id, new_id, 50, 20));
    });
}

#[test]
fn replace_tokens_liquidation_fails_with_insufficient_tokens() {
    run_test(|| {
        let old_id = create_sample_vault();
        let new_id = create_vault(OTHER_ID);

        let res = VaultRegistry::_replace_tokens(&old_id, &new_id, 50, 20);
        assert_err!(res, TestError::InsufficientTokensCommitted);
        assert_not_emitted!(Event::ReplaceTokens(old_id, new_id, 50, 20));
    });
}

#[test]
fn liquidate_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let liquidation_id = create_vault(OTHER_ID);
        <crate::LiquidationVault<Test>>::put(liquidation_id);
        set_default_thresholds();

        let expected_slashed_amount = (DEFAULT_COLLATERAL
            * VaultRegistry::_get_premium_redeem_threshold())
            / 10u128.pow(GRANULARITY);

        ext::collateral::slash::<Test>.mock_safe(move |sender, receiver, amount| {
            assert_eq!(sender, &id);
            assert_eq!(receiver, &liquidation_id);
            assert_eq!(amount, expected_slashed_amount);
            MockResult::Return(Ok(()))
        });

        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 25));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 10));

        let old_liquidation_vault = VaultRegistry::_get_vault_from_id(&liquidation_id).unwrap();
        let res = VaultRegistry::_liquidate_vault(&id);
        assert_ok!(res);
        let liquidation_vault = VaultRegistry::_get_vault_from_id(&liquidation_id).unwrap();

        let liquidated_vault = <crate::Vaults<Test>>::get(&id);
        assert_eq!(liquidated_vault.status, VaultStatus::Liquidated);

        assert_eq!(
            liquidation_vault.issued_tokens,
            old_liquidation_vault.issued_tokens + 25
        );

        assert_eq!(
            liquidation_vault.to_be_issued_tokens,
            old_liquidation_vault.to_be_issued_tokens + 25
        );

        assert_eq!(
            liquidation_vault.to_be_redeemed_tokens,
            old_liquidation_vault.to_be_redeemed_tokens + 10
        );
        assert_emitted!(Event::LiquidateVault(id));
    });
}

#[test]
fn liquidate_with_status_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let liquidation_id = create_vault(OTHER_ID);
        <crate::LiquidationVault<Test>>::put(liquidation_id);
        set_default_thresholds();

        let expected_slashed_amount = (DEFAULT_COLLATERAL
            * VaultRegistry::_get_premium_redeem_threshold())
            / 10u128.pow(GRANULARITY);

        ext::collateral::slash::<Test>.mock_safe(move |sender, receiver, amount| {
            assert_eq!(sender, &id);
            assert_eq!(receiver, &liquidation_id);
            assert_eq!(amount, expected_slashed_amount);
            MockResult::Return(Ok(()))
        });

        VaultRegistry::_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::_issue_tokens(&id, 25));
        assert_ok!(VaultRegistry::_increase_to_be_redeemed_tokens(&id, 10));

        let old_liquidation_vault = VaultRegistry::_get_vault_from_id(&liquidation_id).unwrap();
        let res = VaultRegistry::_liquidate_vault_with_status(&id, VaultStatus::CommittedTheft);
        assert_ok!(res);
        let liquidation_vault = VaultRegistry::_get_vault_from_id(&liquidation_id).unwrap();

        let liquidated_vault = <crate::Vaults<Test>>::get(&id);
        assert_eq!(liquidated_vault.status, VaultStatus::CommittedTheft);

        assert_eq!(
            liquidation_vault.issued_tokens,
            old_liquidation_vault.issued_tokens + 25
        );

        assert_eq!(
            liquidation_vault.to_be_issued_tokens,
            old_liquidation_vault.to_be_issued_tokens + 25
        );

        assert_eq!(
            liquidation_vault.to_be_redeemed_tokens,
            old_liquidation_vault.to_be_redeemed_tokens + 10
        );
        assert_emitted!(Event::LiquidateVault(id));
    });
}

#[test]
fn is_collateral_below_threshold_true_succeeds() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL;
        let btc_amount = 50;
        let threshold = 201000; // 201%

        ext::oracle::dots_to_btc::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(collateral.clone())));

        assert_eq!(
            VaultRegistry::is_collateral_below_threshold(collateral, btc_amount, threshold),
            Ok(true)
        );
    })
}

#[test]
fn is_collateral_below_threshold_false_succeeds() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL;
        let btc_amount = 50;
        let threshold = 200000; // 200%

        ext::oracle::dots_to_btc::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(collateral.clone())));

        assert_eq!(
            VaultRegistry::is_collateral_below_threshold(collateral, btc_amount, threshold),
            Ok(false)
        );
    })
}

#[test]
fn calculate_max_polkabtc_from_collateral_for_threshold_succeeds() {
    run_test(|| {
        let collateral: u128 = u128::MAX;
        let threshold = 200000; // 200%

        ext::oracle::dots_to_btc::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(collateral.clone())));

        assert_eq!(
            VaultRegistry::calculate_max_polkabtc_from_collateral_for_threshold(
                collateral, threshold
            ),
            Ok(170141183460469231731687303715884105727)
        );
    })
}
#[test]
fn get_required_collateral_for_polkabtc_with_threshold_succeeds() {
    run_test(|| {
        let threshold = 19999; // 199.99%
        let random_start = 987529387592 as u128;
        for btc in random_start..random_start + threshold {
            ext::oracle::dots_to_btc::<Test>.mock_safe(move |x| MockResult::Return(Ok(x.clone())));
            ext::oracle::btc_to_dots::<Test>.mock_safe(move |x| MockResult::Return(Ok(x.clone())));

            let min_collateral =
                VaultRegistry::get_required_collateral_for_polkabtc_with_threshold(btc, threshold)
                    .unwrap();

            let max_btc_for_min_collateral =
                VaultRegistry::calculate_max_polkabtc_from_collateral_for_threshold(
                    min_collateral,
                    threshold,
                )
                .unwrap();

            let max_btc_for_below_min_collateral =
                VaultRegistry::calculate_max_polkabtc_from_collateral_for_threshold(
                    min_collateral - 1,
                    threshold,
                )
                .unwrap();

            // Check that the amount we found is indeed the lowest amount that is sufficient for `btc`
            assert!(max_btc_for_min_collateral >= btc);
            assert!(max_btc_for_below_min_collateral < btc);
        }
    })
}

#[test]
fn _is_vault_below_auction_threshold_false_succeeds() {
    run_test(|| {
        // vault has 200% collateral ratio
        let id = create_sample_vault();

        set_default_thresholds();

        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        let res = VaultRegistry::_issue_tokens(&id, 50);
        assert_ok!(res);

        ext::collateral::for_account::<Test>.mock_safe(|_| MockResult::Return(DEFAULT_COLLATERAL));
        ext::oracle::dots_to_btc::<Test>.mock_safe(|_| MockResult::Return(Ok(DEFAULT_COLLATERAL)));

        assert_eq!(
            VaultRegistry::is_vault_below_auction_threshold(&id),
            Ok(false)
        )
    });
}

// Security integration tests
#[test]
fn register_vault_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_noop!(
            VaultRegistry::register_vault(
                Origin::signed(DEFAULT_ID),
                DEFAULT_COLLATERAL,
                BtcAddress::default()
            ),
            SecurityError::ParachainNotRunning
        );
    });
}

#[test]
fn lock_additional_collateral_parachain_not_running_fails() {
    run_test(|| {
        let id = create_vault(RICH_ID);
        let additional = RICH_COLLATERAL - DEFAULT_COLLATERAL;
        ext::security::ensure_parachain_status_not_shutdown::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainShutdown.into())));

        assert_noop!(
            VaultRegistry::lock_additional_collateral(Origin::signed(id), additional),
            SecurityError::ParachainShutdown
        );
    })
}

#[test]
fn _is_vault_below_liquidation_threshold_true_succeeds() {
    run_test(|| {
        // vault has 100% collateral ratio
        let id = create_sample_vault();

        set_default_thresholds();

        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, 50),
            vault.wallet.get_btc_address()
        );
        let res = VaultRegistry::_issue_tokens(&id, 50);
        assert_ok!(res);

        ext::collateral::for_account::<Test>.mock_safe(|_| MockResult::Return(DEFAULT_COLLATERAL));
        ext::oracle::dots_to_btc::<Test>
            .mock_safe(|_| MockResult::Return(Ok(DEFAULT_COLLATERAL / 2)));

        assert_eq!(
            VaultRegistry::_is_vault_below_liquidation_threshold(&id),
            Ok(true)
        );
    })
}

#[test]
fn get_collateralization_from_vault_fails_with_no_tokens_issued() {
    run_test(|| {
        // vault has no tokens issued yet
        let id = create_sample_vault();

        assert_err!(
            VaultRegistry::get_collateralization_from_vault(id, false),
            TestError::NoTokensIssued
        );
    })
}

#[test]
fn get_collateralization_from_vault_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_eq!(
            VaultRegistry::get_collateralization_from_vault(id, false),
            Ok(2 * 10u64.pow(GRANULARITY))
        );
    })
}

#[test]
fn get_unsettled_collateralization_from_vault_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 4; // = 2
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, issue_tokens),
            vault.wallet.get_btc_address()
        );

        assert_eq!(
            VaultRegistry::get_collateralization_from_vault(id, true),
            Ok(5 * 10u64.pow(GRANULARITY))
        );
    })
}

#[test]
fn get_settled_collateralization_from_vault_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 4; // = 2
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        let vault = VaultRegistry::_get_vault_from_id(&id).unwrap();
        assert_ok!(
            VaultRegistry::_increase_to_be_issued_tokens(&id, issue_tokens),
            vault.wallet.get_btc_address()
        );

        assert_eq!(
            VaultRegistry::get_collateralization_from_vault(id, false),
            Ok(25 * 10u64.pow(GRANULARITY - 1))
        );
    })
}

#[test]
fn get_first_vault_with_sufficient_collateral_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = 4;
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_eq!(
            VaultRegistry::get_first_vault_with_sufficient_collateral(issue_tokens),
            Ok(id)
        );
    })
}

#[test]
fn get_first_vault_with_sufficient_tokens_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_eq!(
            VaultRegistry::get_first_vault_with_sufficient_tokens(issue_tokens),
            Ok(id)
        );
    })
}

#[test]
fn get_total_collateralization_with_tokens_issued() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
        let _id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_eq!(
            VaultRegistry::get_total_collateralization(),
            Ok(2 * 10u64.pow(GRANULARITY))
        );
    })
}

#[test]
fn wallet_add_btc_address_succeeds() {
    run_test(|| {
        let address1 = BtcAddress::random();
        let address2 = BtcAddress::random();
        let address3 = BtcAddress::random();

        let mut wallet = Wallet::new(address1);
        assert_eq!(wallet.get_btc_address(), address1);

        wallet.add_btc_address(address2);
        assert_eq!(wallet.get_btc_address(), address2);

        wallet.add_btc_address(address3);
        assert_eq!(wallet.get_btc_address(), address3);
    });
}

#[test]
fn wallet_has_btc_address_succeeds() {
    run_test(|| {
        let address1 = BtcAddress::random();
        let address2 = BtcAddress::random();

        let wallet = Wallet::new(address1);
        assert_eq!(wallet.has_btc_address(&address1), true);
        assert_eq!(wallet.has_btc_address(&address2), false);
    });
}

#[test]
fn update_btc_address_fails_with_btc_address_taken() {
    run_test(|| {
        let origin = DEFAULT_ID;
        let address = BtcAddress::random();

        let mut vault = Vault::default();
        vault.id = origin;
        vault.wallet = Wallet::new(address);
        VaultRegistry::_insert_vault(&origin, vault);

        assert_err!(
            VaultRegistry::update_btc_address(Origin::signed(origin), address),
            TestError::BtcAddressTaken
        );
    });
}

#[test]
fn update_btc_address_succeeds() {
    run_test(|| {
        let origin = DEFAULT_ID;
        let address1 = BtcAddress::random();
        let address2 = BtcAddress::random();

        let mut vault = Vault::default();
        vault.id = origin;
        vault.wallet = Wallet::new(address1);
        VaultRegistry::_insert_vault(&origin, vault);

        assert_ok!(VaultRegistry::update_btc_address(
            Origin::signed(origin),
            address2
        ));
    });
}
fn setup_block(i: u64, parent_hash: H256) -> H256 {
    System::initialize(
        &i,
        &parent_hash,
        &Default::default(),
        &Default::default(),
        frame_system::InitKind::Full,
    );
    <pallet_randomness_collective_flip::Module<Test>>::on_initialize(i);

    let header = System::finalize();
    System::set_block_number(*header.number());
    header.hash()
}
fn setup_blocks(blocks: u64) {
    let mut parent_hash = System::parent_hash();
    for i in 1..(blocks + 1) {
        parent_hash = setup_block(i, parent_hash);
    }
}

#[test]
fn get_first_vault_with_sufficient_tokens_returns_different_vaults_for_different_amounts() {
    run_test(|| {
        setup_blocks(100);

        let vault_ids = MULTI_VAULT_TEST_IDS
            .iter()
            .map(|&i| {
                create_vault_and_issue_tokens(
                    MULTI_VAULT_TEST_COLLATERAL / 100,
                    MULTI_VAULT_TEST_COLLATERAL,
                    i,
                )
            })
            .collect::<Vec<_>>();
        let selected_ids = (1..50)
            .map(|i| VaultRegistry::get_first_vault_with_sufficient_tokens(i).unwrap())
            .collect::<Vec<_>>();

        // check that all vaults have been selected at least once
        assert!(vault_ids
            .iter()
            .all(|&x| selected_ids.iter().any(|&y| x == y)));
    });
}

#[test]
fn get_first_vault_with_sufficient_tokens_returns_different_vaults_for_different_blocks() {
    run_test(|| {
        setup_blocks(100);

        let vault_ids = MULTI_VAULT_TEST_IDS
            .iter()
            .map(|&i| {
                create_vault_and_issue_tokens(
                    MULTI_VAULT_TEST_COLLATERAL / 100,
                    MULTI_VAULT_TEST_COLLATERAL,
                    i,
                )
            })
            .collect::<Vec<_>>();
        let selected_ids = (101..150)
            .map(|i| {
                setup_block(i, System::parent_hash());
                VaultRegistry::get_first_vault_with_sufficient_tokens(5).unwrap()
            })
            .collect::<Vec<_>>();

        // check that all vaults have been selected at least once
        assert!(vault_ids
            .iter()
            .all(|&x| selected_ids.iter().any(|&y| x == y)));
    });
}
#[test]
fn get_first_vault_with_sufficient_collateral_returns_different_vaults_for_different_amounts() {
    run_test(|| {
        setup_blocks(100);

        let vault_ids = MULTI_VAULT_TEST_IDS
            .iter()
            .map(|&i| {
                create_vault_and_issue_tokens(
                    MULTI_VAULT_TEST_COLLATERAL / 100,
                    MULTI_VAULT_TEST_COLLATERAL,
                    i,
                )
            })
            .collect::<Vec<_>>();
        let selected_ids = (1..50)
            .map(|i| VaultRegistry::get_first_vault_with_sufficient_collateral(i).unwrap())
            .collect::<Vec<_>>();

        // check that all vaults have been selected at least once
        assert!(vault_ids
            .iter()
            .all(|&x| selected_ids.iter().any(|&y| x == y)));
    });
}

#[test]
fn get_first_vault_with_sufficient_collateral_returns_different_vaults_for_different_blocks() {
    run_test(|| {
        setup_blocks(100);

        let vault_ids = MULTI_VAULT_TEST_IDS
            .iter()
            .map(|&i| {
                create_vault_and_issue_tokens(
                    MULTI_VAULT_TEST_COLLATERAL / 100,
                    MULTI_VAULT_TEST_COLLATERAL,
                    i,
                )
            })
            .collect::<Vec<_>>();
        let selected_ids = (101..150)
            .map(|i| {
                setup_block(i, System::parent_hash());
                VaultRegistry::get_first_vault_with_sufficient_collateral(5).unwrap()
            })
            .collect::<Vec<_>>();

        // check that all vaults have been selected at least once
        assert!(vault_ids
            .iter()
            .all(|&x| selected_ids.iter().any(|&y| x == y)));
    });
}
