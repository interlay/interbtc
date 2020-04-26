use frame_support::traits::Currency;
use frame_support::{assert_err, assert_ok};
use sp_core::H160;

use mocktopus::mocking::*;

use crate::ext;
use crate::mock::{run_test, Origin, System, Test, TestEvent, VaultRegistry};
use crate::types::DOT;
use crate::Error;

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

const DEFAULT_ID: u64 = 3;
const DEFAULT_COLLATERAL: u64 = 100;

fn create_sample_vault() -> <Test as system::Trait>::AccountId {
    VaultRegistry::get_minimum_collateral_vault
        .mock_safe(|| MockResult::Return(DEFAULT_COLLATERAL));
    let id = DEFAULT_ID;
    let collateral = DEFAULT_COLLATERAL;
    let _ = <DOT<Test>>::deposit_creating(&id, collateral);
    let origin = Origin::signed(id);
    let result = VaultRegistry::register_vault(origin, collateral, H160::zero());
    assert_ok!(result);
    id
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
        let result = VaultRegistry::register_vault(Origin::signed(id), collateral, H160::zero());
        assert_err!(result, Error::InsuficientVaultCollateralAmount);
        assert_not_emitted!(Event::RegisterVault(id, collateral));
    });
}

#[test]
fn register_vault_fails_when_account_funds_too_low() {
    run_test(|| {
        let collateral = DEFAULT_COLLATERAL + 1;
        let result =
            VaultRegistry::register_vault(Origin::signed(DEFAULT_ID), collateral, H160::zero());
        assert_err!(result, Error::InsufficientFunds);
        assert_not_emitted!(Event::RegisterVault(DEFAULT_ID, collateral));
    });
}

#[test]
fn register_vault_fails_when_already_registered() {
    run_test(|| {
        let id = create_sample_vault();
        let result =
            VaultRegistry::register_vault(Origin::signed(id), DEFAULT_COLLATERAL, H160::zero());
        assert_err!(result, Error::VaultAlreadyRegistered);
        assert_emitted!(Event::RegisterVault(id, DEFAULT_COLLATERAL), 1);
    });
}

#[test]
fn lock_additional_collateral_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        let _ = <DOT<Test>>::deposit_creating(&id, 50);
        let res = VaultRegistry::lock_additional_collateral(Origin::signed(id), 50);
        assert_ok!(res);
        let new_collateral = ext::collateral::for_account::<Test>(&id);
        assert_eq!(new_collateral, DEFAULT_COLLATERAL + 50);
        assert_emitted!(Event::LockAdditionalCollateral(id, 50, 150, 150));

        Ok(())
    })
}

#[test]
fn lock_additional_collateral_fails_when_vault_does_not_exist() {
    run_test(|| {
        let res = VaultRegistry::lock_additional_collateral(Origin::signed(3), 50);
        assert_err!(res, Error::VaultNotFound);
    })
}

#[test]
fn withdraw_collateral_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::withdraw_collateral(Origin::signed(id), 50);
        assert_ok!(res);
        let new_collateral = ext::collateral::for_account::<Test>(&id);
        assert_eq!(new_collateral, DEFAULT_COLLATERAL - 50);
        assert_emitted!(Event::WithdrawCollateral(id, 50, DEFAULT_COLLATERAL - 50));

        Ok(())
    })
}

#[test]
fn withdraw_collateral_fails_when_vault_does_not_exist() {
    run_test(|| {
        let res = VaultRegistry::withdraw_collateral(Origin::signed(3), 50);
        assert_err!(res, Error::VaultNotFound);
    })
}

#[test]
fn withdraw_collateral_fails_when_not_enough_collateral() {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::withdraw_collateral(Origin::signed(id), DEFAULT_COLLATERAL + 1);
        assert_err!(res, Error::InsufficientCollateralAvailable);
    })
}

#[test]
fn increase_to_be_issued_tokens_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        let res = VaultRegistry::increase_to_be_issued_tokens(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.to_be_issued_tokens, 50);
        assert_emitted!(Event::IncreaseToBeIssuedTokens(id, 50));

        Ok(())
    })
}

#[test]
fn increase_to_be_issued_tokens_fails_with_insufficient_collateral() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        let vault = VaultRegistry::rich_vault_from_id(&id)?;
        let res = VaultRegistry::increase_to_be_issued_tokens(
            Origin::signed(id),
            vault.issuable_tokens()? + 1,
        );
        assert_err!(res, Error::ExceedingVaultLimit);

        Ok(())
    })
}

#[test]
fn decrease_to_be_issued_tokens_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::increase_to_be_issued_tokens(
            Origin::signed(id),
            50
        ));
        let res = VaultRegistry::decrease_to_be_issued_tokens(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.to_be_issued_tokens, 0);
        assert_emitted!(Event::DecreaseToBeIssuedTokens(id, 50));

        Ok(())
    })
}

#[test]
fn decrease_to_be_issued_tokens_fails_with_insufficient_tokens() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::decrease_to_be_issued_tokens(Origin::signed(id), 50);
        assert_err!(res, Error::InsufficientTokensCommitted);

        Ok(())
    })
}

#[test]
fn issue_tokens_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::increase_to_be_issued_tokens(
            Origin::signed(id),
            50
        ));
        let res = VaultRegistry::issue_tokens(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.to_be_issued_tokens, 0);
        assert_eq!(vault.issued_tokens, 50);
        assert_emitted!(Event::IssueTokens(id, 50));

        Ok(())
    })
}

#[test]
fn issue_tokens_fails_with_insufficient_tokens() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::issue_tokens(Origin::signed(id), 50);
        assert_err!(res, Error::InsufficientTokensCommitted);

        Ok(())
    })
}

#[test]
fn increase_to_be_redeemed_tokens_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::increase_to_be_issued_tokens(
            Origin::signed(id),
            50
        ));
        assert_ok!(VaultRegistry::issue_tokens(Origin::signed(id), 50));
        let res = VaultRegistry::increase_to_be_redeemed_tokens(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.issued_tokens, 50);
        assert_eq!(vault.to_be_redeemed_tokens, 50);
        assert_emitted!(Event::IncreaseToBeRedeemedTokens(id, 50));

        Ok(())
    })
}

#[test]
fn increase_to_be_redeemed_tokens_fails_with_insufficient_tokens() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::increase_to_be_redeemed_tokens(Origin::signed(id), 50);
        assert_err!(res, Error::InsufficientTokensCommitted);

        Ok(())
    })
}

#[test]
fn decrease_to_be_redeemed_tokens_succeeds() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();
        assert_ok!(VaultRegistry::increase_to_be_issued_tokens(
            Origin::signed(id),
            50
        ));
        assert_ok!(VaultRegistry::issue_tokens(Origin::signed(id), 50));
        assert_ok!(VaultRegistry::increase_to_be_redeemed_tokens(
            Origin::signed(id),
            50
        ));
        let res = VaultRegistry::decrease_to_be_redeemed_tokens(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.issued_tokens, 50);
        assert_eq!(vault.to_be_redeemed_tokens, 0);
        assert_emitted!(Event::DecreaseToBeRedeemedTokens(id, 50));

        Ok(())
    })
}

#[test]
fn decrease_to_be_redeemed_tokens_fails_with_insufficient_tokens() -> Result<(), Error> {
    run_test(|| {
        let id = create_sample_vault();

        let res = VaultRegistry::decrease_to_be_redeemed_tokens(Origin::signed(id), 50);
        assert_err!(res, Error::InsufficientTokensCommitted);

        Ok(())
    })
}
