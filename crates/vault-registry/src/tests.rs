use frame_support::traits::Currency;
use frame_support::{assert_err, assert_ok};
use sp_core::H160;

use mocktopus::mocking::*;

use crate::mock::{run_test, Origin, System, Test, TestEvent, VaultRegistry};
use crate::{Error, DOT};

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
        let new_collateral = VaultRegistry::get_vault_collateral(&id);
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
