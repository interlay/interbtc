use crate::{
    ext,
    mock::{
        run_test, CollateralError, LiquidationTarget, Origin, SecurityError, System, Test, TestError, TestEvent,
        VaultRegistry, DEFAULT_COLLATERAL, DEFAULT_ID, MULTI_VAULT_TEST_COLLATERAL, MULTI_VAULT_TEST_IDS, OTHER_ID,
        RICH_COLLATERAL, RICH_ID,
    },
    types::{Backing, BtcAddress, Issuing},
    BtcPublicKey, CurrencySource, DispatchError, Error, UpdatableVault, Vault, VaultStatus, Vaults, Wallet, H256,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use mocktopus::mocking::*;
use primitive_types::U256;
use security::Pallet as Security;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_runtime::traits::Header;
use sp_std::convert::TryInto;
use std::{collections::HashMap, rc::Rc};
type Event = crate::Event<Test>;
use frame_support::traits::OnInitialize;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::vault_registry($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::vault_registry($event);
        assert_eq!(
            System::events().iter().filter(|a| a.event == test_event).count(),
            $times
        );
    };
}

macro_rules! assert_not_emitted {
    ($event:expr) => {
        let test_event = TestEvent::vault_registry($event);
        assert!(!System::events().iter().any(|a| a.event == test_event));
    };
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn create_vault_with_collateral(id: u64, collateral: u128) -> <Test as frame_system::Config>::AccountId {
    VaultRegistry::get_minimum_collateral_vault.mock_safe(move || MockResult::Return(collateral));
    let origin = Origin::signed(id);
    let result = VaultRegistry::register_vault(origin, collateral, dummy_public_key());
    assert_ok!(result);
    id
}

fn create_vault(id: u64) -> <Test as frame_system::Config>::AccountId {
    create_vault_with_collateral(id, DEFAULT_COLLATERAL)
}

fn create_sample_vault() -> <Test as frame_system::Config>::AccountId {
    create_vault(DEFAULT_ID)
}

fn create_vault_and_issue_tokens(
    issue_tokens: u128,
    collateral: u128,
    id: u64,
) -> <Test as frame_system::Config>::AccountId {
    // vault has no tokens issued yet
    let id = create_vault_with_collateral(id, collateral);

    // exchange rate 1 Satoshi = 10 Planck (smallest unit of DOT)
    ext::oracle::backing_to_issuing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x / 10)));

    // issue tokens with 200% collateralization of DEFAULT_COLLATERAL
    assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, issue_tokens,));
    let res = VaultRegistry::issue_tokens(&id, issue_tokens);
    assert_ok!(res);

    // mint tokens to the vault
    currency::Pallet::<Test, currency::Instance2>::mint(id, issue_tokens);

    id
}

fn create_sample_vault_and_issue_tokens(issue_tokens: u128) -> <Test as frame_system::Config>::AccountId {
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
        let result = VaultRegistry::register_vault(Origin::signed(id), collateral, dummy_public_key());
        assert_err!(result, TestError::InsufficientVaultCollateralAmount);
        assert_not_emitted!(Event::RegisterVault(id, collateral));
    });
}

#[test]
fn register_vault_fails_when_account_funds_too_low() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL + 1;
        let result = VaultRegistry::register_vault(Origin::signed(DEFAULT_ID), collateral, dummy_public_key());
        assert_err!(result, CollateralError::InsufficientFreeBalance);
        assert_not_emitted!(Event::RegisterVault(DEFAULT_ID, collateral));
    });
}

#[test]
fn register_vault_fails_when_already_registered() {
    run_test(|| {
        let id = create_sample_vault();
        let result = VaultRegistry::register_vault(Origin::signed(id), DEFAULT_COLLATERAL, dummy_public_key());
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
        assert_err!(res, TestError::InsufficientCollateral);
    })
}

#[test]
fn try_increase_to_be_issued_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::try_increase_to_be_issued_tokens(&id, 50);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_ok!(res);
        assert_eq!(vault.data.to_be_issued_tokens, 50);
        assert_emitted!(Event::IncreaseToBeIssuedTokens(id, 50));
    });
}

#[test]
fn try_increase_to_be_issued_tokens_fails_with_insufficient_collateral() {
    run_test(|| {
        let id = create_sample_vault();
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        let res = VaultRegistry::try_increase_to_be_issued_tokens(&id, vault.issuable_tokens().unwrap() + 1);
        // important: should not change the storage state
        assert_noop!(res, TestError::ExceedingVaultLimit);
    });
}

#[test]
fn decrease_to_be_issued_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        let res = VaultRegistry::decrease_to_be_issued_tokens(&id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.to_be_issued_tokens, 0);
        assert_emitted!(Event::DecreaseToBeIssuedTokens(id, 50));
    });
}

#[test]
fn decrease_to_be_issued_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::decrease_to_be_issued_tokens(&id, 50);
        assert_err!(res, TestError::ArithmeticUnderflow);
    });
}

#[test]
fn issue_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        let res = VaultRegistry::issue_tokens(&id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.to_be_issued_tokens, 0);
        assert_eq!(vault.data.issued_tokens, 50);
        assert_emitted!(Event::IssueTokens(id, 50));
    });
}

#[test]
fn issue_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        assert_err!(VaultRegistry::issue_tokens(&id, 50), TestError::ArithmeticUnderflow);
    });
}

#[test]
fn try_increase_to_be_replaced_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();

        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        let res = VaultRegistry::try_increase_to_be_replaced_tokens(&id, 50, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 50);
        assert_eq!(vault.data.to_be_replaced_tokens, 50);
        assert_emitted!(Event::IncreaseToBeReplacedTokens(id, 50));
    });
}

#[test]
fn try_increase_to_be_replaced_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::try_increase_to_be_replaced_tokens(&id, 50, 50);

        // important: should not change the storage state
        assert_noop!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn try_increase_to_be_redeemed_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();

        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        let res = VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 50);
        assert_eq!(vault.data.to_be_redeemed_tokens, 50);
        assert_emitted!(Event::IncreaseToBeRedeemedTokens(id, 50));
    });
}

#[test]
fn try_increase_to_be_redeemed_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50);

        // important: should not change the storage state
        assert_noop!(res, TestError::InsufficientTokensCommitted);
    });
}

#[test]
fn decrease_to_be_redeemed_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::decrease_to_be_redeemed_tokens(&id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 50);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::DecreaseToBeRedeemedTokens(id, 50));
    });
}

#[test]
fn decrease_to_be_redeemed_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::decrease_to_be_redeemed_tokens(&id, 50);
        assert_err!(res, TestError::ArithmeticUnderflow);
    });
}

#[test]
fn decrease_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::decrease_tokens(&id, &user_id, 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 0);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::DecreaseTokens(id, user_id, 50));
    });
}

#[test]
fn decrease_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        let res = VaultRegistry::decrease_tokens(&id, &user_id, 50);
        assert_err!(res, TestError::ArithmeticUnderflow);
    });
}

#[test]
fn redeem_tokens_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50));
        let res = VaultRegistry::redeem_tokens(&id, 50, 0, &0);
        assert_ok!(res);
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 0);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::RedeemTokens(id, 50));
    });
}

#[test]
fn redeem_tokens_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        let res = VaultRegistry::redeem_tokens(&id, 50, 0, &0);
        assert_err!(res, TestError::ArithmeticUnderflow);
    });
}

#[test]
fn redeem_tokens_premium_succeeds() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        // TODO: emulate assert_called
        VaultRegistry::slash_collateral.mock_safe(move |sender, receiver, _amount| {
            assert_eq!(sender, CurrencySource::Backing(id));
            assert_eq!(receiver, CurrencySource::FreeBalance(user_id));
            MockResult::Return(Ok(()))
        });

        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&id, 50));
        assert_ok!(VaultRegistry::redeem_tokens(&id, 50, 30, &user_id));

        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, 0);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::RedeemTokensPremium(id, 50, 30, user_id));
    });
}

#[test]
fn redeem_tokens_premium_fails_with_insufficient_tokens() {
    run_test(|| {
        let id = create_sample_vault();
        let user_id = 5;
        VaultRegistry::try_increase_to_be_issued_tokens(&id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, 50));
        let res = VaultRegistry::redeem_tokens(&id, 50, 30, &user_id);
        assert_err!(res, TestError::ArithmeticUnderflow);
        assert_not_emitted!(Event::RedeemTokensPremium(id, 50, 30, user_id));
    });
}

#[test]
fn redeem_tokens_liquidation_succeeds() {
    run_test(|| {
        let mut liquidation_vault = VaultRegistry::get_rich_liquidation_vault();
        let user_id = 5;

        // TODO: emulate assert_called
        VaultRegistry::slash_collateral.mock_safe(move |sender, receiver, _amount| {
            assert_eq!(sender, CurrencySource::LiquidationVault);
            assert_eq!(receiver, CurrencySource::FreeBalance(user_id));
            MockResult::Return(Ok(()))
        });

        // liquidation vault collateral
        ext::collateral::for_account::<Test>.mock_safe(|_| MockResult::Return(1000u32.into()));

        assert_ok!(liquidation_vault.increase_to_be_issued(50));
        assert_ok!(liquidation_vault.increase_issued(50));

        assert_ok!(VaultRegistry::redeem_tokens_liquidation(&user_id, 50));
        let liquidation_vault = VaultRegistry::get_rich_liquidation_vault();
        assert_eq!(liquidation_vault.data.issued_tokens, 0);
        assert_emitted!(Event::RedeemTokensLiquidation(user_id, 50, 500));
    });
}

#[test]
fn redeem_tokens_liquidation_does_not_call_recover_when_unnecessary() {
    run_test(|| {
        let mut liquidation_vault = VaultRegistry::get_rich_liquidation_vault();
        let user_id = 5;

        VaultRegistry::slash_collateral.mock_safe(move |sender, receiver, _amount| {
            assert_eq!(sender, CurrencySource::LiquidationVault);
            assert_eq!(receiver, CurrencySource::FreeBalance(user_id));
            MockResult::Return(Ok(()))
        });

        // liquidation vault collateral
        ext::collateral::for_account::<Test>.mock_safe(|_| MockResult::Return(1000u32.into()));

        assert_ok!(liquidation_vault.increase_to_be_issued(25));
        assert_ok!(liquidation_vault.increase_issued(25));

        assert_ok!(VaultRegistry::redeem_tokens_liquidation(&user_id, 10));
        let liquidation_vault = VaultRegistry::get_rich_liquidation_vault();
        assert_eq!(liquidation_vault.data.issued_tokens, 15);
        assert_emitted!(Event::RedeemTokensLiquidation(user_id, 10, (1000 * 10) / 50));
    });
}

#[test]
fn redeem_tokens_liquidation_fails_with_insufficient_tokens() {
    run_test(|| {
        let user_id = 5;
        let res = VaultRegistry::redeem_tokens_liquidation(&user_id, 50);
        assert_err!(res, TestError::InsufficientTokensCommitted);
        assert_not_emitted!(Event::RedeemTokensLiquidation(user_id, 50, 50));
    });
}

#[test]
fn replace_tokens_liquidation_succeeds() {
    run_test(|| {
        let old_id = create_sample_vault();
        let new_id = create_vault(OTHER_ID);

        ext::collateral::lock::<Test>.mock_safe(move |sender, amount| {
            assert_eq!(sender, &new_id);
            assert_eq!(amount, 20);
            MockResult::Return(Ok(()))
        });

        VaultRegistry::try_increase_to_be_issued_tokens(&old_id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&old_id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&old_id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&new_id, 50));

        assert_ok!(VaultRegistry::replace_tokens(&old_id, &new_id, 50, 20));

        let old_vault = VaultRegistry::get_active_rich_vault_from_id(&old_id).unwrap();
        let new_vault = VaultRegistry::get_active_rich_vault_from_id(&new_id).unwrap();
        assert_eq!(old_vault.data.issued_tokens, 0);
        assert_eq!(old_vault.data.to_be_redeemed_tokens, 0);
        assert_eq!(new_vault.data.issued_tokens, 50);
        assert_eq!(new_vault.data.to_be_issued_tokens, 0);
        assert_emitted!(Event::ReplaceTokens(old_id, new_id, 50, 20));
    });
}

#[test]
fn cancel_replace_tokens_succeeds() {
    run_test(|| {
        let old_id = create_sample_vault();
        let new_id = create_vault(OTHER_ID);

        ext::collateral::lock::<Test>.mock_safe(move |sender, amount| {
            assert_eq!(sender, &new_id);
            assert_eq!(amount, 20);
            MockResult::Return(Ok(()))
        });

        VaultRegistry::try_increase_to_be_issued_tokens(&old_id, 50).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&old_id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&old_id, 50));
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&new_id, 50));

        assert_ok!(VaultRegistry::cancel_replace_tokens(&old_id, &new_id, 50));

        let old_vault = VaultRegistry::get_active_rich_vault_from_id(&old_id).unwrap();
        let new_vault = VaultRegistry::get_active_rich_vault_from_id(&new_id).unwrap();
        assert_eq!(old_vault.data.issued_tokens, 50);
        assert_eq!(old_vault.data.to_be_redeemed_tokens, 0);
        assert_eq!(new_vault.data.issued_tokens, 0);
        assert_eq!(new_vault.data.to_be_issued_tokens, 0);
    });
}

#[test]
fn liquidate_succeeds() {
    run_test(|| {
        let vault_id = create_sample_vault();

        let issued_tokens = 100;
        let to_be_issued_tokens = 25;
        let to_be_redeemed_tokens = 40;

        let liquidation_vault_before = VaultRegistry::get_rich_liquidation_vault();

        VaultRegistry::set_secure_collateral_threshold(
            FixedU128::checked_from_rational(1, 100).unwrap(), // 1%
        );

        let collateral_before = ext::collateral::for_account::<Test>(&vault_id);
        assert_eq!(collateral_before, DEFAULT_COLLATERAL); // sanity check

        // required for `issue_tokens` to work
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(
            &vault_id,
            issued_tokens
        ));
        assert_ok!(VaultRegistry::issue_tokens(&vault_id, issued_tokens));
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(
            &vault_id,
            to_be_issued_tokens
        ));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(
            &vault_id,
            to_be_redeemed_tokens
        ));

        let vault_orig = <crate::Vaults<Test>>::get(&vault_id);

        ext::oracle::issuing_to_backing::<Test>.mock_safe(|_| MockResult::Return(Ok(1000000000u32.into())));

        assert_ok!(VaultRegistry::liquidate_vault(&vault_id));

        let liquidation_vault_after = VaultRegistry::get_rich_liquidation_vault();

        let liquidated_vault = <crate::Vaults<Test>>::get(&vault_id);
        assert_eq!(liquidated_vault.status, VaultStatus::Liquidated);
        assert_emitted!(Event::LiquidateVault(
            vault_id,
            vault_orig.issued_tokens,
            vault_orig.to_be_issued_tokens,
            vault_orig.to_be_redeemed_tokens,
            vault_orig.to_be_replaced_tokens,
            vault_orig.backing_collateral,
            VaultStatus::Liquidated,
            vault_orig.replace_collateral,
        ));

        let moved_collateral = (collateral_before * (issued_tokens + to_be_issued_tokens - to_be_redeemed_tokens))
            / (issued_tokens + to_be_issued_tokens);

        // check liquidation_vault tokens & collateral
        assert_eq!(
            liquidation_vault_after.data.issued_tokens,
            liquidation_vault_before.data.issued_tokens + issued_tokens
        );
        assert_eq!(
            liquidation_vault_after.data.to_be_issued_tokens,
            liquidation_vault_before.data.to_be_issued_tokens + to_be_issued_tokens
        );
        assert_eq!(
            liquidation_vault_after.data.to_be_redeemed_tokens,
            liquidation_vault_before.data.to_be_redeemed_tokens + to_be_redeemed_tokens
        );
        assert_eq!(
            ext::collateral::for_account::<Test>(&liquidation_vault_before.id()),
            moved_collateral
        );

        // check vault tokens & collateral
        let user_vault_after = VaultRegistry::get_rich_vault_from_id(&vault_id).unwrap();
        assert_eq!(user_vault_after.data.issued_tokens, 0);
        assert_eq!(user_vault_after.data.to_be_issued_tokens, 0);
        assert_eq!(user_vault_after.data.to_be_redeemed_tokens, to_be_redeemed_tokens);
        assert_eq!(
            ext::collateral::for_account::<Test>(&vault_id),
            collateral_before - moved_collateral
        );
    });
}

#[test]
fn liquidate_at_most_secure_threshold() {
    run_test(|| {
        let vault_id = create_sample_vault();

        let issued_tokens = 100;
        let to_be_issued_tokens = 25;
        let to_be_redeemed_tokens = 40;
        let used_collateral = 50u128;
        let liquidation_vault_before = VaultRegistry::get_rich_liquidation_vault();

        VaultRegistry::set_secure_collateral_threshold(
            FixedU128::checked_from_rational(1, 100).unwrap(), // 1%
        );

        let collateral_before = ext::collateral::for_account::<Test>(&vault_id);
        assert_eq!(collateral_before, DEFAULT_COLLATERAL); // sanity check

        // required for `issue_tokens` to work
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(
            &vault_id,
            issued_tokens
        ));
        assert_ok!(VaultRegistry::issue_tokens(&vault_id, issued_tokens));
        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(
            &vault_id,
            to_be_issued_tokens
        ));
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(
            &vault_id,
            to_be_redeemed_tokens
        ));

        let vault_orig = <crate::Vaults<Test>>::get(&vault_id);

        // set used for used_collateral
        ext::oracle::issuing_to_backing::<Test>.mock_safe(|_| MockResult::Return(Ok(50)));
        VaultRegistry::set_secure_collateral_threshold(FixedU128::one());
        assert_ok!(VaultRegistry::liquidate_vault(&vault_id));

        let liquidation_vault_after = VaultRegistry::get_rich_liquidation_vault();

        let liquidated_vault = <crate::Vaults<Test>>::get(&vault_id);
        assert_eq!(liquidated_vault.status, VaultStatus::Liquidated);
        assert_emitted!(Event::LiquidateVault(
            vault_id,
            vault_orig.issued_tokens,
            vault_orig.to_be_issued_tokens,
            vault_orig.to_be_redeemed_tokens,
            vault_orig.to_be_replaced_tokens,
            vault_orig.backing_collateral,
            VaultStatus::Liquidated,
            vault_orig.replace_collateral,
        ));

        let moved_collateral = (used_collateral * (issued_tokens + to_be_issued_tokens - to_be_redeemed_tokens))
            / (to_be_issued_tokens + issued_tokens);

        // check liquidation_vault tokens & collateral
        assert_eq!(
            liquidation_vault_after.data.issued_tokens,
            liquidation_vault_before.data.issued_tokens + issued_tokens
        );
        assert_eq!(
            liquidation_vault_after.data.to_be_issued_tokens,
            liquidation_vault_before.data.to_be_issued_tokens + to_be_issued_tokens
        );
        assert_eq!(
            liquidation_vault_after.data.to_be_redeemed_tokens,
            liquidation_vault_before.data.to_be_redeemed_tokens + to_be_redeemed_tokens
        );
        assert_eq!(
            ext::collateral::for_account::<Test>(&liquidation_vault_before.id()),
            moved_collateral
        );

        // check vault tokens & collateral
        let user_vault_after = VaultRegistry::get_rich_vault_from_id(&vault_id).unwrap();
        assert_eq!(user_vault_after.data.issued_tokens, 0);
        assert_eq!(user_vault_after.data.to_be_issued_tokens, 0);
        assert_eq!(user_vault_after.data.to_be_redeemed_tokens, to_be_redeemed_tokens);
        assert_eq!(
            ext::collateral::for_account::<Test>(&vault_id),
            used_collateral - moved_collateral
        );
        assert_eq!(
            ext::collateral::get_free_balance::<Test>(&vault_id),
            DEFAULT_COLLATERAL - used_collateral
        );
    });
}

#[test]
fn liquidate_with_status_succeeds() {
    run_test(|| {
        let id = create_sample_vault();

        let vault_orig = <crate::Vaults<Test>>::get(&id);

        assert_ok!(VaultRegistry::liquidate_vault_with_status(
            &id,
            VaultStatus::CommittedTheft
        ));

        let liquidated_vault = <crate::Vaults<Test>>::get(&id);

        assert_eq!(liquidated_vault.status, VaultStatus::CommittedTheft);

        assert_emitted!(Event::LiquidateVault(
            id,
            vault_orig.issued_tokens,
            vault_orig.to_be_issued_tokens,
            vault_orig.to_be_redeemed_tokens,
            vault_orig.to_be_replaced_tokens,
            vault_orig.backing_collateral,
            VaultStatus::CommittedTheft,
            vault_orig.replace_collateral,
        ));
    });
}

#[test]
fn is_collateral_below_threshold_true_succeeds() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL;
        let btc_amount = 50;
        let threshold = FixedU128::checked_from_rational(201, 100).unwrap(); // 201%

        ext::oracle::backing_to_issuing::<Test>.mock_safe(move |_| MockResult::Return(Ok(collateral)));

        assert_eq!(
            VaultRegistry::is_collateral_below_threshold(collateral, btc_amount, threshold),
            Ok(true)
        );
    })
}

#[test]
fn test_liquidate_undercollateralized_vaults_no_liquidation() {
    run_test(|| {
        Vaults::<Test>::insert(
            0,
            Vault {
                id: 0,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            1,
            Vault {
                id: 1,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            2,
            Vault {
                id: 2,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            3,
            Vault {
                id: 3,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            4,
            Vault {
                id: 4,
                ..Default::default()
            },
        );

        let vaults: HashMap<<Test as frame_system::Config>::AccountId, bool> =
            vec![(0, false), (1, false), (2, false), (3, false), (4, false)]
                .into_iter()
                .collect();

        VaultRegistry::is_vault_below_liquidation_threshold
            .mock_safe(move |vault, _| MockResult::Return(Ok(*vaults.get(&vault.id).unwrap())));
        VaultRegistry::liquidate_vault.mock_safe(move |_| {
            panic!("Should not liquidate any vaults");
        });

        VaultRegistry::liquidate_undercollateralized_vaults(LiquidationTarget::NonOperatorsOnly);
    });
}

#[test]
fn test_liquidate_undercollateralized_vaults_succeeds() {
    run_test(|| {
        Vaults::<Test>::insert(
            0,
            Vault {
                id: 0,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            1,
            Vault {
                id: 1,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            2,
            Vault {
                id: 2,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            3,
            Vault {
                id: 3,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            4,
            Vault {
                id: 4,
                ..Default::default()
            },
        );

        let vaults: HashMap<<Test as frame_system::Config>::AccountId, bool> =
            vec![(0, true), (1, false), (2, true), (3, false), (4, false)]
                .into_iter()
                .collect();
        let vaults1 = Rc::new(vaults);
        let vaults2 = vaults1.clone();

        VaultRegistry::is_vault_below_liquidation_threshold
            .mock_safe(move |vault, _| MockResult::Return(Ok(*vaults1.get(&vault.id).unwrap())));
        VaultRegistry::liquidate_vault.mock_safe(move |id| {
            assert!(vaults2.get(id).unwrap());
            MockResult::Return(Ok(10u128))
        });

        VaultRegistry::liquidate_undercollateralized_vaults(LiquidationTarget::NonOperatorsOnly);
    });
}

#[test]
fn is_collateral_below_threshold_false_succeeds() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL;
        let btc_amount = 50;
        let threshold = FixedU128::checked_from_rational(200, 100).unwrap(); // 200%

        ext::oracle::backing_to_issuing::<Test>.mock_safe(move |_| MockResult::Return(Ok(collateral)));

        assert_eq!(
            VaultRegistry::is_collateral_below_threshold(collateral, btc_amount, threshold),
            Ok(false)
        );
    })
}

#[test]
fn calculate_max_issuing_from_collateral_for_threshold_succeeds() {
    run_test(|| {
        let collateral: u128 = u64::MAX as u128;
        let threshold = FixedU128::checked_from_rational(200, 100).unwrap(); // 200%

        ext::oracle::backing_to_issuing::<Test>.mock_safe(move |_| MockResult::Return(Ok(collateral)));

        assert_eq!(
            VaultRegistry::calculate_max_issuing_from_collateral_for_threshold(collateral, threshold),
            Ok((u64::MAX / 2) as u128)
        );
    })
}

#[test]
fn test_threshold_equivalent_to_legacy_calculation() {
    /// old version
    fn legacy_calculate_max_issuing_from_collateral_for_threshold(
        collateral: Backing<Test>,
        threshold: u128,
    ) -> Result<Issuing<Test>, DispatchError> {
        let granularity = 5;
        // convert the collateral to issuing
        let collateral_in_issuing = ext::oracle::backing_to_issuing::<Test>(collateral)?;
        let collateral_in_issuing = VaultRegistry::issuing_to_u128(collateral_in_issuing)?;
        let collateral_in_issuing = U256::from(collateral_in_issuing);

        // calculate how many tokens should be maximally issued given the threshold
        let scaled_collateral_in_issuing = collateral_in_issuing
            .checked_mul(U256::from(10).pow(granularity.into()))
            .ok_or(Error::<Test>::ArithmeticOverflow)?;
        let scaled_max_tokens = scaled_collateral_in_issuing
            .checked_div(threshold.into())
            .unwrap_or(0.into());

        VaultRegistry::u128_to_issuing(scaled_max_tokens.try_into()?)
    }

    run_test(|| {
        let threshold = FixedU128::checked_from_rational(199999, 100000).unwrap(); // 199.999%
        let random_start = 987529462328 as u128;
        for btc in random_start..random_start + 199999 {
            ext::oracle::backing_to_issuing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));
            ext::oracle::issuing_to_backing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));
            let old = legacy_calculate_max_issuing_from_collateral_for_threshold(btc, 199999).unwrap();
            let new = VaultRegistry::calculate_max_issuing_from_collateral_for_threshold(btc, threshold).unwrap();
            assert_eq!(old, new);
        }
    })
}

#[test]
fn test_get_required_collateral_threshold_equivalent_to_legacy_calculation_() {
    // old version
    fn legacy_get_required_collateral_for_issuing_with_threshold(
        btc: Issuing<Test>,
        threshold: u128,
    ) -> Result<Backing<Test>, DispatchError> {
        let granularity = 5;
        let btc = VaultRegistry::issuing_to_u128(btc)?;
        let btc = U256::from(btc);

        // Step 1: inverse of the scaling applied in calculate_max_issuing_from_collateral_for_threshold

        // inverse of the div
        let btc = btc
            .checked_mul(threshold.into())
            .ok_or(Error::<Test>::ArithmeticOverflow)?;

        // To do the inverse of the multiplication, we need to do division, but
        // we need to round up. To round up (a/b), we need to do ((a+b-1)/b):
        let rounding_addition = U256::from(10).pow(granularity.into()) - U256::from(1);
        let btc = (btc + rounding_addition)
            .checked_div(U256::from(10).pow(granularity.into()))
            .ok_or(Error::<Test>::ArithmeticUnderflow)?;

        // Step 2: convert the amount to backing
        let scaled = VaultRegistry::u128_to_issuing(btc.try_into()?)?;
        let amount_in_backing = ext::oracle::issuing_to_backing::<Test>(scaled)?;
        Ok(amount_in_backing)
    }

    run_test(|| {
        let threshold = FixedU128::checked_from_rational(199999, 100000).unwrap(); // 199.999%
        let random_start = 987529462328 as u128;
        for btc in random_start..random_start + 199999 {
            ext::oracle::backing_to_issuing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));
            ext::oracle::issuing_to_backing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));
            let old = legacy_get_required_collateral_for_issuing_with_threshold(btc, 199999);
            let new = VaultRegistry::get_required_collateral_for_issuing_with_threshold(btc, threshold);
            assert_eq!(old, new);
        }
    })
}

#[test]
fn get_required_collateral_for_issuing_with_threshold_succeeds() {
    run_test(|| {
        let threshold = FixedU128::checked_from_rational(19999, 10000).unwrap(); // 199.99%
        let random_start = 987529387592 as u128;
        for btc in random_start..random_start + 19999 {
            ext::oracle::backing_to_issuing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));
            ext::oracle::issuing_to_backing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x)));

            let min_collateral =
                VaultRegistry::get_required_collateral_for_issuing_with_threshold(btc, threshold).unwrap();

            let max_btc_for_min_collateral =
                VaultRegistry::calculate_max_issuing_from_collateral_for_threshold(min_collateral, threshold).unwrap();

            let max_btc_for_below_min_collateral =
                VaultRegistry::calculate_max_issuing_from_collateral_for_threshold(min_collateral - 1, threshold)
                    .unwrap();

            // Check that the amount we found is indeed the lowest amount that is sufficient for `btc`
            assert!(max_btc_for_min_collateral >= btc);
            assert!(max_btc_for_below_min_collateral < btc);
        }
    })
}

// Security integration tests
#[test]
fn register_vault_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_not_shutdown::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        assert_noop!(
            VaultRegistry::register_vault(Origin::signed(DEFAULT_ID), DEFAULT_COLLATERAL, dummy_public_key()),
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
fn is_vault_below_liquidation_threshold_true_succeeds() {
    run_test(|| {
        // vault has 100% collateral ratio
        let id = create_sample_vault();

        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, 50),);
        let res = VaultRegistry::issue_tokens(&id, 50);
        assert_ok!(res);

        ext::collateral::for_account::<Test>.mock_safe(|_| MockResult::Return(DEFAULT_COLLATERAL));
        ext::oracle::backing_to_issuing::<Test>.mock_safe(|_| MockResult::Return(Ok(DEFAULT_COLLATERAL / 2)));

        let vault = VaultRegistry::get_vault_from_id(&id).unwrap();
        let threshold = VaultRegistry::get_liquidation_collateral_threshold();
        assert_eq!(
            VaultRegistry::is_vault_below_liquidation_threshold(&vault, threshold),
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
            Ok(FixedU128::checked_from_rational(200, 100).unwrap())
        );
    })
}

#[test]
fn get_unsettled_collateralization_from_vault_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 4; // = 2
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, issue_tokens),);

        assert_eq!(
            VaultRegistry::get_collateralization_from_vault(id, true),
            Ok(FixedU128::checked_from_rational(500, 100).unwrap())
        );
    })
}

#[test]
fn get_settled_collateralization_from_vault_succeeds() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 4; // = 2
        let id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_ok!(VaultRegistry::try_increase_to_be_issued_tokens(&id, issue_tokens),);

        assert_eq!(
            VaultRegistry::get_collateralization_from_vault(id, false),
            Ok(FixedU128::checked_from_rational(250, 100).unwrap())
        );
    })
}

mod get_vaults_below_premium_collaterlization_tests {
    use super::*;

    /// sets premium_redeem threshold to 1
    pub fn run_test(test: impl FnOnce()) {
        super::run_test(|| {
            VaultRegistry::set_secure_collateral_threshold(FixedU128::from_float(0.001));
            VaultRegistry::set_premium_redeem_threshold(FixedU128::one());

            test()
        })
    }

    fn add_vault(id: u64, issued_tokens: u128, collateral: u128) {
        create_vault_with_collateral(id, collateral);

        VaultRegistry::try_increase_to_be_issued_tokens(&id, issued_tokens).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, issued_tokens));

        // sanity check
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, issued_tokens);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
    }

    #[test]
    fn get_vaults_below_premium_collateralization_fails() {
        run_test(|| {
            add_vault(4, 50, 100);

            assert_err!(
                VaultRegistry::get_premium_redeem_vaults(),
                TestError::NoVaultUnderThePremiumRedeemThreshold
            );
        })
    }

    #[test]
    fn get_vaults_below_premium_collateralization_succeeds() {
        run_test(|| {
            let id1 = 3;
            let issue_tokens1: u128 = 50;
            let collateral1 = 49;

            let id2 = 4;
            let issue_tokens2: u128 = 50;
            let collateral2 = 48;

            add_vault(id1, issue_tokens1, collateral1);
            add_vault(id2, issue_tokens2, collateral2);

            assert_eq!(
                VaultRegistry::get_premium_redeem_vaults(),
                Ok(vec!((id1, issue_tokens1), (id2, issue_tokens2)))
            );
        })
    }

    #[test]
    fn get_vaults_below_premium_collateralization_filters_banned_and_sufficiently_collateralized_vaults() {
        run_test(|| {
            // not returned, because is is not under premium threshold (which is set to 100% for this test)
            let id1 = 3;
            let issue_tokens1: u128 = 50;
            let collateral1 = 50;
            add_vault(id1, issue_tokens1, collateral1);

            // returned
            let id2 = 4;
            let issue_tokens2: u128 = 50;
            let collateral2 = 49;
            add_vault(id2, issue_tokens2, collateral2);

            // not returned because it's banned
            let id3 = 5;
            let issue_tokens3: u128 = 50;
            let collateral3 = 49;
            add_vault(id3, issue_tokens3, collateral3);
            let mut vault3 = VaultRegistry::get_active_rich_vault_from_id(&id3).unwrap();
            vault3.ban_until(1000);

            assert_eq!(
                VaultRegistry::get_premium_redeem_vaults(),
                Ok(vec!((id2, issue_tokens2)))
            );
        })
    }
}

mod get_vaults_with_issuable_tokens_tests {
    use super::*;

    #[test]
    fn get_vaults_with_issuable_tokens_succeeds() {
        run_test(|| {
            let id1 = 3;
            let collateral1 = 100;
            create_vault_with_collateral(id1, collateral1);
            let issuable_tokens1 =
                VaultRegistry::get_issuable_tokens_from_vault(id1).expect("Sample vault is unable to issue tokens");

            let id2 = 4;
            let collateral2 = 50;
            create_vault_with_collateral(id2, collateral2);
            let issuable_tokens2 =
                VaultRegistry::get_issuable_tokens_from_vault(id2).expect("Sample vault is unable to issue tokens");

            // Check result is ordered in descending order
            assert_eq!(issuable_tokens1.gt(&issuable_tokens2), true);
            assert_eq!(
                VaultRegistry::get_vaults_with_issuable_tokens(),
                Ok(vec!((id1, issuable_tokens1), (id2, issuable_tokens2)))
            );
        })
    }
    #[test]
    fn get_vaults_with_issuable_tokens_succeeds_when_there_are_liquidated_vaults() {
        run_test(|| {
            let id1 = 3;
            let collateral1 = 100;
            create_vault_with_collateral(id1, collateral1);
            let issuable_tokens1 =
                VaultRegistry::get_issuable_tokens_from_vault(id1).expect("Sample vault is unable to issue tokens");

            let id2 = 4;
            let collateral2 = 50;
            create_vault_with_collateral(id2, collateral2);

            // liquidate vault
            assert_ok!(VaultRegistry::liquidate_vault_with_status(
                &id2,
                VaultStatus::Liquidated
            ));

            assert_eq!(
                VaultRegistry::get_vaults_with_issuable_tokens(),
                Ok(vec!((id1, issuable_tokens1)))
            );
        })
    }

    #[test]
    fn get_vaults_with_issuable_tokens_filters_out_banned_vaults() {
        run_test(|| {
            let id1 = 3;
            let collateral1 = 100;
            create_vault_with_collateral(id1, collateral1);
            let issuable_tokens1 =
                VaultRegistry::get_issuable_tokens_from_vault(id1).expect("Sample vault is unable to issue tokens");

            let id2 = 4;
            let collateral2 = 50;
            create_vault_with_collateral(id2, collateral2);

            // ban the vault
            let mut vault = VaultRegistry::get_rich_vault_from_id(&id2).unwrap();
            vault.ban_until(1000);

            let issuable_tokens2 =
                VaultRegistry::get_issuable_tokens_from_vault(id2).expect("Sample vault is unable to issue tokens");

            assert_eq!(issuable_tokens2, 0);

            // Check that the banned vault is not returned by get_vaults_with_issuable_tokens
            assert_eq!(
                VaultRegistry::get_vaults_with_issuable_tokens(),
                Ok(vec!((id1, issuable_tokens1)))
            );
        })
    }

    #[test]
    fn get_vaults_with_issuable_tokens_filters_out_vault_that_do_not_accept_new_issues() {
        run_test(|| {
            let id1 = 3;
            let collateral1 = 100;
            create_vault_with_collateral(id1, collateral1);
            let issuable_tokens1 =
                VaultRegistry::get_issuable_tokens_from_vault(id1).expect("Sample vault is unable to issue tokens");

            let id2 = 4;
            let collateral2 = 50;
            create_vault_with_collateral(id2, collateral2);
            assert_ok!(VaultRegistry::accept_new_issues(Origin::signed(id2), false));

            // Check that the vault that does not accept issues is not returned by get_vaults_with_issuable_tokens
            assert_eq!(
                VaultRegistry::get_vaults_with_issuable_tokens(),
                Ok(vec!((id1, issuable_tokens1)))
            );
        })
    }

    #[test]
    fn get_vaults_with_issuable_tokens_returns_empty() {
        run_test(|| {
            let issue_tokens: u128 = 50;
            let id = create_sample_vault();

            VaultRegistry::try_increase_to_be_issued_tokens(&id, issue_tokens).unwrap();
            // issue 50 tokens at 200% rate
            assert_ok!(VaultRegistry::issue_tokens(&id, issue_tokens));
            let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
            assert_eq!(vault.data.issued_tokens, issue_tokens);
            assert_eq!(vault.data.to_be_redeemed_tokens, 0);

            // update the exchange rate
            ext::oracle::backing_to_issuing::<Test>.mock_safe(move |x| MockResult::Return(Ok(x / 2)));

            assert_eq!(VaultRegistry::get_vaults_with_issuable_tokens(), Ok(vec!()));
        })
    }
}

mod get_vaults_with_redeemable_tokens_test {
    use super::*;

    fn create_vault_with_issue(id: u64, to_issue: u128) {
        create_vault(id);
        VaultRegistry::try_increase_to_be_issued_tokens(&id, to_issue).unwrap();
        assert_ok!(VaultRegistry::issue_tokens(&id, to_issue));
        let vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
        assert_eq!(vault.data.issued_tokens, to_issue);
        assert_eq!(vault.data.to_be_redeemed_tokens, 0);
    }

    #[test]
    fn get_vaults_with_redeemable_tokens_returns_empty() {
        run_test(|| {
            // create a vault with no redeemable tokens
            create_sample_vault();
            // nothing issued, so nothing can be redeemed
            assert_eq!(VaultRegistry::get_vaults_with_redeemable_tokens(), Ok(vec!()));
        })
    }

    #[test]
    fn get_vaults_with_redeemable_tokens_succeeds() {
        run_test(|| {
            let id1 = 3;
            let issued_tokens1: u128 = 10;
            create_vault_with_issue(id1, issued_tokens1);

            let id2 = 4;
            let issued_tokens2: u128 = 20;
            create_vault_with_issue(id2, issued_tokens2);

            // Check result is ordered in descending order
            assert_eq!(issued_tokens2.gt(&issued_tokens1), true);
            assert_eq!(
                VaultRegistry::get_vaults_with_redeemable_tokens(),
                Ok(vec!((id2, issued_tokens2), (id1, issued_tokens1)))
            );
        })
    }

    #[test]
    fn get_vaults_with_redeemable_tokens_filters_out_banned_vaults() {
        run_test(|| {
            let id1 = 3;
            let issued_tokens1: u128 = 10;
            create_vault_with_issue(id1, issued_tokens1);

            let id2 = 4;
            let issued_tokens2: u128 = 20;
            create_vault_with_issue(id2, issued_tokens2);

            // ban the vault
            let mut vault = VaultRegistry::get_rich_vault_from_id(&id2).unwrap();
            vault.ban_until(1000);

            // Check that the banned vault is not returned by get_vaults_with_redeemable_tokens
            assert_eq!(
                VaultRegistry::get_vaults_with_redeemable_tokens(),
                Ok(vec!((id1, issued_tokens1)))
            );
        })
    }

    #[test]
    fn get_vaults_with_issuable_tokens_filters_out_liquidated_vaults() {
        run_test(|| {
            let id1 = 3;
            let issued_tokens1: u128 = 10;
            create_vault_with_issue(id1, issued_tokens1);

            let id2 = 4;
            let issued_tokens2: u128 = 20;
            create_vault_with_issue(id2, issued_tokens2);

            // liquidate vault
            assert_ok!(VaultRegistry::liquidate_vault_with_status(
                &id2,
                VaultStatus::Liquidated
            ));

            assert_eq!(
                VaultRegistry::get_vaults_with_redeemable_tokens(),
                Ok(vec!((id1, issued_tokens1)))
            );
        })
    }
}

#[test]
fn get_total_collateralization_with_tokens_issued() {
    run_test(|| {
        let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
        let _id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_eq!(
            VaultRegistry::get_total_collateralization(),
            Ok(FixedU128::checked_from_rational(200, 100).unwrap())
        );
    })
}

// #[test]
// fn wallet_add_btc_address_succeeds() {
//     run_test(|| {
//         let address1 = BtcAddress::random();
//         let address2 = BtcAddress::random();
//         let address3 = BtcAddress::random();

//         let mut wallet = Wallet::new(address1);
//         assert_eq!(wallet.get_btc_address(), address1);

//         wallet.add_btc_address(address2);
//         assert_eq!(wallet.get_btc_address(), address2);

//         wallet.add_btc_address(address3);
//         assert_eq!(wallet.get_btc_address(), address3);
//     });
// }

#[test]
fn wallet_has_btc_address_succeeds() {
    use sp_std::collections::btree_set::BTreeSet;

    run_test(|| {
        let address1 = BtcAddress::random();
        let address2 = BtcAddress::random();

        let mut addresses = BTreeSet::new();
        addresses.insert(address1);

        let wallet = Wallet {
            addresses,
            public_key: dummy_public_key(),
        };
        assert_eq!(wallet.has_btc_address(&address1), true);
        assert_eq!(wallet.has_btc_address(&address2), false);
    });
}

// #[test]
// fn update_btc_address_fails_with_btc_address_taken() {
//     run_test(|| {
//         let origin = DEFAULT_ID;
//         let address = BtcAddress::random();

//         let mut vault = Vault::default();
//         vault.id = origin;
//         vault.wallet = Wallet::new(address);
//         VaultRegistry::insert_vault(&origin, vault);

//         assert_err!(
//             VaultRegistry::update_btc_address(Origin::signed(origin), address),
//             TestError::BtcAddressTaken
//         );
//     });
// }

// #[test]
// fn update_btc_address_succeeds() {
//     run_test(|| {
//         let origin = DEFAULT_ID;
//         let address1 = BtcAddress::random();
//         let address2 = BtcAddress::random();

//         let mut vault = Vault::default();
//         vault.id = origin;
//         vault.wallet = Wallet::new(address1);
//         VaultRegistry::insert_vault(&origin, vault);

//         assert_ok!(VaultRegistry::update_btc_address(
//             Origin::signed(origin),
//             address2
//         ));
//     });
// }

fn setup_block(i: u64, parent_hash: H256) -> H256 {
    System::initialize(&i, &parent_hash, &Default::default(), frame_system::InitKind::Full);
    <pallet_randomness_collective_flip::Pallet<Test>>::on_initialize(i);

    let header = System::finalize();
    Security::<Test>::set_active_block_number(*header.number());
    header.hash()
}

fn setup_blocks(blocks: u64) {
    let mut parent_hash = System::parent_hash();
    for i in 1..(blocks + 1) {
        parent_hash = setup_block(i, parent_hash);
    }
}

#[test]
fn runtime_upgrade_succeeds() {
    run_test(|| {
        Vaults::<Test>::insert(
            0,
            Vault {
                backing_collateral: 10,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            1,
            Vault {
                backing_collateral: 20,
                ..Default::default()
            },
        );
        Vaults::<Test>::insert(
            2,
            Vault {
                backing_collateral: 30,
                ..Default::default()
            },
        );
        assert_eq!(0, VaultRegistry::get_total_backing_collateral(false).unwrap());
        VaultRegistry::_on_runtime_upgrade();
        assert_eq!(60, VaultRegistry::get_total_backing_collateral(false).unwrap());
    })
}

mod get_first_vault_with_sufficient_tokens_tests {
    use super::*;

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
    fn get_first_vault_with_sufficient_tokens_considers_to_be_redeemed() {
        run_test(|| {
            let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
            let id = create_sample_vault_and_issue_tokens(issue_tokens);
            let mut vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();

            assert_ok!(vault.increase_to_be_redeemed(2));

            assert_noop!(
                VaultRegistry::get_first_vault_with_sufficient_tokens(issue_tokens),
                TestError::NoVaultWithSufficientTokens
            );

            assert_eq!(VaultRegistry::get_first_vault_with_sufficient_tokens(3), Ok(id));
        })
    }

    #[test]
    fn get_first_vault_with_sufficient_tokens_filters_banned_vaults() {
        run_test(|| {
            let issue_tokens: u128 = DEFAULT_COLLATERAL / 10 / 2; // = 5
            let id = create_sample_vault_and_issue_tokens(issue_tokens);
            let mut vault = VaultRegistry::get_active_rich_vault_from_id(&id).unwrap();
            vault.ban_until(1000);

            assert_noop!(
                VaultRegistry::get_first_vault_with_sufficient_tokens(issue_tokens),
                TestError::NoVaultWithSufficientTokens
            );
        })
    }

    #[test]
    fn get_first_vault_with_sufficient_tokens_returns_different_vaults_for_different_amounts() {
        run_test(|| {
            setup_blocks(100);

            let vault_ids = MULTI_VAULT_TEST_IDS
                .iter()
                .map(|&i| {
                    create_vault_and_issue_tokens(MULTI_VAULT_TEST_COLLATERAL / 100, MULTI_VAULT_TEST_COLLATERAL, i)
                })
                .collect::<Vec<_>>();
            let selected_ids = (1..50)
                .map(|i| VaultRegistry::get_first_vault_with_sufficient_tokens(i).unwrap())
                .collect::<Vec<_>>();

            // check that all vaults have been selected at least once
            assert!(vault_ids.iter().all(|&x| selected_ids.iter().any(|&y| x == y)));
        });
    }

    #[test]
    fn get_first_vault_with_sufficient_tokens_returns_different_vaults_for_different_blocks() {
        run_test(|| {
            setup_blocks(100);

            let vault_ids = MULTI_VAULT_TEST_IDS
                .iter()
                .map(|&i| {
                    create_vault_and_issue_tokens(MULTI_VAULT_TEST_COLLATERAL / 100, MULTI_VAULT_TEST_COLLATERAL, i)
                })
                .collect::<Vec<_>>();
            let selected_ids = (101..150)
                .map(|i| {
                    setup_block(i, System::parent_hash());
                    VaultRegistry::get_first_vault_with_sufficient_tokens(5).unwrap()
                })
                .collect::<Vec<_>>();

            // check that all vaults have been selected at least once
            assert!(vault_ids.iter().all(|&x| selected_ids.iter().any(|&y| x == y)));
        });
    }
}

mod get_first_vault_with_sufficient_collateral_test {
    use super::*;

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
    fn get_first_vault_with_sufficient_collateral_with_no_suitable_vaults_fails() {
        run_test(|| {
            create_sample_vault_and_issue_tokens(4);

            assert_err!(
                VaultRegistry::get_first_vault_with_sufficient_collateral(5),
                TestError::NoVaultWithSufficientCollateral
            );
        })
    }

    #[test]
    fn get_first_vault_with_sufficient_collateral_filters_vaults_that_do_not_accept_new_issues() {
        run_test(|| {
            let issue_tokens: u128 = 4;
            let id = create_sample_vault_and_issue_tokens(issue_tokens);
            assert_ok!(VaultRegistry::accept_new_issues(Origin::signed(id), false));

            assert_err!(
                VaultRegistry::get_first_vault_with_sufficient_collateral(issue_tokens),
                TestError::NoVaultWithSufficientCollateral
            );
        })
    }

    #[test]
    fn get_first_vault_with_sufficient_collateral_returns_different_vaults_for_different_amounts() {
        run_test(|| {
            setup_blocks(100);

            let vault_ids = MULTI_VAULT_TEST_IDS
                .iter()
                .map(|&i| {
                    create_vault_and_issue_tokens(MULTI_VAULT_TEST_COLLATERAL / 100, MULTI_VAULT_TEST_COLLATERAL, i)
                })
                .collect::<Vec<_>>();
            let selected_ids = (1..50)
                .map(|i| VaultRegistry::get_first_vault_with_sufficient_collateral(i).unwrap())
                .collect::<Vec<_>>();

            // check that all vaults have been selected at least once
            assert!(vault_ids.iter().all(|&x| selected_ids.iter().any(|&y| x == y)));
        });
    }

    #[test]
    fn get_first_vault_with_sufficient_collateral_returns_different_vaults_for_different_blocks() {
        run_test(|| {
            setup_blocks(100);

            let vault_ids = MULTI_VAULT_TEST_IDS
                .iter()
                .map(|&i| {
                    create_vault_and_issue_tokens(MULTI_VAULT_TEST_COLLATERAL / 100, MULTI_VAULT_TEST_COLLATERAL, i)
                })
                .collect::<Vec<_>>();
            let selected_ids = (101..150)
                .map(|i| {
                    setup_block(i, System::parent_hash());
                    VaultRegistry::get_first_vault_with_sufficient_collateral(5).unwrap()
                })
                .collect::<Vec<_>>();

            // check that all vaults have been selected at least once
            assert!(vault_ids.iter().all(|&x| selected_ids.iter().any(|&y| x == y)));
        });
    }
}

#[test]
fn test_try_increase_to_be_replaced_tokens() {
    run_test(|| {
        let issue_tokens: u128 = 4;
        let vault_id = create_sample_vault_and_issue_tokens(issue_tokens);
        assert_ok!(VaultRegistry::try_increase_to_be_redeemed_tokens(&vault_id, 1));

        let (total_issuing, total_backing) =
            VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 2, 10).unwrap();
        assert!(total_issuing == 2);
        assert!(total_backing == 10);

        // check that we can't request more than we have issued tokens
        assert_noop!(
            VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 3, 10),
            TestError::InsufficientTokensCommitted
        );

        // check that we can't request replacement for tokens that are marked as to-be-redeemed
        assert_noop!(
            VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 2, 10),
            TestError::InsufficientTokensCommitted
        );

        let (total_issuing, total_backing) =
            VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 1, 20).unwrap();
        assert!(total_issuing == 3);
        assert!(total_backing == 30);

        // check that is was written to storage
        let vault = VaultRegistry::get_active_vault_from_id(&vault_id).unwrap();
        assert_eq!(vault.to_be_replaced_tokens, 3);
        assert_eq!(vault.replace_collateral, 30);
    })
}

#[test]
fn test_decrease_to_be_replaced_tokens_over_capacity() {
    run_test(|| {
        let issue_tokens: u128 = 4;
        let vault_id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_ok!(VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 4, 10));

        let (tokens, collateral) = VaultRegistry::decrease_to_be_replaced_tokens(&vault_id, 5).unwrap();
        assert_eq!(tokens, 4);
        assert_eq!(collateral, 10);
    })
}

#[test]
fn test_decrease_to_be_replaced_tokens_below_capacity() {
    run_test(|| {
        let issue_tokens: u128 = 4;
        let vault_id = create_sample_vault_and_issue_tokens(issue_tokens);

        assert_ok!(VaultRegistry::try_increase_to_be_replaced_tokens(&vault_id, 4, 10));

        let (tokens, collateral) = VaultRegistry::decrease_to_be_replaced_tokens(&vault_id, 3).unwrap();
        assert_eq!(tokens, 3);
        assert_eq!(collateral, 7);
    })
}
