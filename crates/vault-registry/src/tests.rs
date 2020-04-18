use frame_support::{assert_err, assert_ok};
use sp_core::H160;

use mocktopus::mocking::*;

use crate::mock::{run_test, Origin, System, Test, TestEvent, VaultRegistry};
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

#[test]
fn register_vault_succeeds() {
    run_test(|| {
        VaultRegistry::get_minimum_collateral_vault.mock_safe(|| MockResult::Return(100));
        let id = 3;
        let collateral = 100;
        let origin = Origin::signed(id);
        let result = VaultRegistry::register_vault(origin, collateral, H160::zero());
        assert_ok!(result);
        assert_emitted!(Event::RegisterVault(id, collateral));
    });
}

#[test]
fn register_vault_fails_when_collateral_too_low() {
    run_test(|| {
        VaultRegistry::get_minimum_collateral_vault.mock_safe(|| MockResult::Return(200));
        let id = 3;
        let collateral = 100;
        let origin = Origin::signed(id);
        let result = VaultRegistry::register_vault(origin, collateral, H160::zero());
        assert_err!(result, Error::InsuficientVaultCollateralAmount);
        assert_not_emitted!(Event::RegisterVault(id, collateral));
    });
}

#[test]
fn register_vault_fails_when_already_registered() {
    run_test(|| {
        VaultRegistry::get_minimum_collateral_vault.mock_safe(|| MockResult::Return(100));
        let id = 3;
        let collateral = 100;
        let origin = Origin::signed(id);
        let result = VaultRegistry::register_vault(origin.clone(), collateral, H160::zero());
        assert_ok!(result);
        let next_result = VaultRegistry::register_vault(origin, collateral, H160::zero());
        assert_err!(next_result, Error::VaultAlreadyRegistered);
        assert_emitted!(Event::RegisterVault(id, collateral), 1);
    });
}
