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

const DEFAULT_COLLATERAL: u64 = 100;

fn create_sample_vault() -> <Test as system::Trait>::AccountId {
    VaultRegistry::get_minimum_collateral_vault
        .mock_safe(|| MockResult::Return(DEFAULT_COLLATERAL));
    let id = 3;
    let collateral = DEFAULT_COLLATERAL;
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
fn register_vault_fails_when_collateral_too_low() {
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
        let res = VaultRegistry::lock_additional_collateral(Origin::signed(id), 50);
        assert_ok!(res);
        let vault = VaultRegistry::get_vault_from_id(&id)?;
        assert_eq!(vault.collateral, DEFAULT_COLLATERAL + 50);

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
