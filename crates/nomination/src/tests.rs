use std::sync::{Arc, Mutex};

use crate::mock::*;
use frame_support::{assert_err, assert_ok};
use sp_arithmetic::FixedI128;
use vault_registry::{Collateral, Slashable, SlashingError, TryDepositCollateral, TryWithdrawCollateral};

#[derive(Default)]
struct SimpleVault {
    collateral: u128,
    total_collateral: u128,
    backing_collateral: u128,
    slash_per_token: FixedI128,
    slash_tally: FixedI128,
}

impl Slashable<u128, FixedI128, SlashingError> for SimpleVault {
    fn mut_slash_per_token<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut FixedI128) -> Result<(), SlashingError>,
    {
        func(&mut self.slash_per_token)
    }
}

impl Collateral<u128, FixedI128, SlashingError> for SimpleVault {
    fn get_slash_per_token(&self) -> Result<FixedI128, SlashingError> {
        Ok(self.slash_per_token)
    }

    fn get_collateral(&self) -> u128 {
        self.collateral
    }

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        func(&mut self.collateral)
    }

    fn get_total_collateral(&self) -> Result<u128, SlashingError> {
        Ok(self.total_collateral)
    }

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        func(&mut self.total_collateral)
    }

    fn get_backing_collateral(&self) -> Result<u128, SlashingError> {
        Ok(self.backing_collateral)
    }

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        func(&mut self.backing_collateral)
    }

    fn get_slash_tally(&self) -> FixedI128 {
        self.slash_tally
    }

    fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut FixedI128) -> Result<(), SlashingError>,
    {
        func(&mut self.slash_tally)
    }
}

struct SimpleNominator {
    collateral: u128,
    slash_tally: FixedI128,
    vault: Arc<Mutex<SimpleVault>>,
}

impl SimpleNominator {
    pub fn new(arc_vault: Arc<Mutex<SimpleVault>>) -> Self {
        SimpleNominator {
            collateral: Default::default(),
            slash_tally: Default::default(),
            vault: arc_vault,
        }
    }
}

impl Collateral<u128, FixedI128, SlashingError> for SimpleNominator {
    fn get_slash_per_token(&self) -> Result<FixedI128, SlashingError> {
        self.vault.lock().unwrap().get_slash_per_token()
    }

    fn get_collateral(&self) -> u128 {
        self.collateral
    }

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        func(&mut self.collateral)
    }

    fn get_total_collateral(&self) -> Result<u128, SlashingError> {
        self.vault.lock().unwrap().get_total_collateral()
    }

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        self.vault.lock().unwrap().mut_total_collateral(func)
    }

    fn get_backing_collateral(&self) -> Result<u128, SlashingError> {
        self.vault.lock().unwrap().get_backing_collateral()
    }

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut u128) -> Result<(), SlashingError>,
    {
        self.vault.lock().unwrap().mut_backing_collateral(func)
    }

    fn get_slash_tally(&self) -> FixedI128 {
        self.slash_tally
    }

    fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), SlashingError>
    where
        F: Fn(&mut FixedI128) -> Result<(), SlashingError>,
    {
        func(&mut self.slash_tally)
    }
}

#[test]
fn test_non_vaults_cannot_become_operators() {
    run_test(|| {
        assert_err!(
            Nomination::opt_in_to_nomination(Origin::signed(BOB)),
            TestError::NotAVault
        );
    })
}

fn nominate_slash_nominate(
    arc_vault: Arc<Mutex<SimpleVault>>,
    nominator1: &mut SimpleNominator,
    nominator2: &mut SimpleNominator,
    vault_deposit: u128,
    nominator1_deposit: u128,
    nominator2_deposit: u128,
    slash: u128,
) {
    let backing_collateral_at_slash = vault_deposit + nominator1_deposit;
    assert_ok!(arc_vault.lock().unwrap().try_deposit_collateral(vault_deposit));
    assert_ok!(nominator1.try_deposit_collateral(nominator1_deposit));
    assert_ok!(arc_vault.lock().unwrap().slash_collateral(slash));
    assert_ok!(nominator2.try_deposit_collateral(nominator2_deposit));
    assert_ok!(
        arc_vault.lock().unwrap().get_backing_collateral(),
        backing_collateral_at_slash - slash + nominator2_deposit
    );
    assert_ok!(
        arc_vault.lock().unwrap().compute_collateral(),
        vault_deposit - slash * vault_deposit / backing_collateral_at_slash - 1
    );
}

#[test]
fn test_nomination_slash_should_be_correct() {
    let vault_deposit = 10000;
    let nominator1_deposit = 5000;
    let nominator2_deposit = 4000;
    let slash = 100;
    let backing_collateral_at_slash = vault_deposit + nominator1_deposit;

    let vault = SimpleVault::default();
    let arc_vault = Arc::new(Mutex::new(vault));
    let mut nominator1 = SimpleNominator::new(arc_vault.clone());
    let mut nominator2 = SimpleNominator::new(arc_vault.clone());

    nominate_slash_nominate(
        arc_vault.clone(),
        &mut nominator1,
        &mut nominator2,
        vault_deposit,
        nominator1_deposit,
        nominator2_deposit,
        slash,
    );

    assert_ok!(
        nominator1.compute_collateral(),
        nominator1_deposit - slash * nominator1_deposit / backing_collateral_at_slash - 1
    );
    assert_ok!(nominator2.compute_collateral(), nominator2_deposit);
}

#[test]
fn test_nomination_nominator_withdrawal_after_slash() {
    let vault_deposit = 10000;
    let nominator1_deposit = 5000;
    let nominator2_deposit = 4000;
    let slash = 100;
    let backing_collateral_at_slash = vault_deposit + nominator1_deposit;

    let vault = SimpleVault::default();
    let arc_vault = Arc::new(Mutex::new(vault));
    let mut nominator1 = SimpleNominator::new(arc_vault.clone());
    let mut nominator2 = SimpleNominator::new(arc_vault.clone());

    nominate_slash_nominate(
        arc_vault.clone(),
        &mut nominator1,
        &mut nominator2,
        vault_deposit,
        nominator1_deposit,
        nominator2_deposit,
        slash,
    );

    assert_ok!(nominator1
        .try_withdraw_collateral(nominator1_deposit - slash * nominator1_deposit / backing_collateral_at_slash - 1));
    assert_ok!(nominator2.compute_collateral(), nominator2_deposit);
}

#[test]
fn test_nomination_vault_withdrawal_after_slash() {
    let vault_deposit = 10000;
    let nominator1_deposit = 5000;
    let nominator2_deposit = 4000;
    let slash = 100;
    let backing_collateral_at_slash = vault_deposit + nominator1_deposit;

    let vault = SimpleVault::default();
    let arc_vault = Arc::new(Mutex::new(vault));
    let mut nominator1 = SimpleNominator::new(arc_vault.clone());
    let mut nominator2 = SimpleNominator::new(arc_vault.clone());

    nominate_slash_nominate(
        arc_vault.clone(),
        &mut nominator1,
        &mut nominator2,
        vault_deposit,
        nominator1_deposit,
        nominator2_deposit,
        slash,
    );

    assert_ok!(arc_vault
        .lock()
        .unwrap()
        .try_withdraw_collateral(vault_deposit - slash * vault_deposit / backing_collateral_at_slash - 1));
    assert_ok!(nominator2.compute_collateral(), nominator2_deposit);
}

#[test]
fn test_nomination_slash_twice() {
    let vault_deposit = 10000;
    let nominator1_deposit = 5000;
    let nominator2_deposit = 4000;
    let slash = 100;
    let backing_collateral_at_first_slash = vault_deposit + nominator1_deposit;
    let backing_collateral_at_second_slash = backing_collateral_at_first_slash - slash + nominator2_deposit;

    let vault = SimpleVault::default();
    let arc_vault = Arc::new(Mutex::new(vault));
    let mut nominator1 = SimpleNominator::new(arc_vault.clone());
    let mut nominator2 = SimpleNominator::new(arc_vault.clone());

    nominate_slash_nominate(
        arc_vault.clone(),
        &mut nominator1,
        &mut nominator2,
        vault_deposit,
        nominator1_deposit,
        nominator2_deposit,
        slash,
    );
    let nominator1_collateral_after_first_slash = nominator1.compute_collateral().unwrap();
    let nominator2_collateral_after_first_slash = nominator2.compute_collateral().unwrap();
    assert_ok!(arc_vault.lock().unwrap().slash_collateral(slash));

    assert_ok!(
        nominator1.compute_collateral(),
        nominator1_collateral_after_first_slash
            - slash * nominator1_collateral_after_first_slash / backing_collateral_at_second_slash
    );
    assert_ok!(
        nominator2.compute_collateral(),
        nominator2_collateral_after_first_slash
            - slash * nominator2_collateral_after_first_slash / backing_collateral_at_second_slash
            - 1
    );
}
