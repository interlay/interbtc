use currency::Amount;

use crate::{assert_eq, *};

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

pub const DEFAULT_BACKING_COLLATERAL: Balance = 1_000_000;
pub const DEFAULT_NOMINATION: Balance = 20_000;

pub const DEFAULT_VAULT_UNBONDING_PERIOD: u32 = 100;
pub const DEFAULT_NOMINATOR_UNBONDING_PERIOD: u32 = 50;

pub fn default_backing_collateral(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_BACKING_COLLATERAL, currency_id)
}

pub fn enable_nomination() {
    assert_ok!(
        Call::Nomination(NominationCall::set_nomination_enabled { enabled: true })
            .dispatch(<Runtime as frame_system::Config>::Origin::root())
    );
}

pub fn disable_nomination() {
    assert_ok!(
        Call::Nomination(NominationCall::set_nomination_enabled { enabled: false })
            .dispatch(<Runtime as frame_system::Config>::Origin::root())
    );
}

pub fn nomination_opt_in(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_in_to_nomination {
        currency_pair: vault_id.currencies.clone(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn assert_nomination_opt_in(vault_id: &VaultId) {
    assert_ok!(nomination_opt_in(vault_id));
}

pub fn nomination_opt_out(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_out_of_nomination {
        currency_pair: vault_id.currencies.clone(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn nominate_collateral(
    vault_id: &VaultId,
    nominator_id: AccountId,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::deposit_collateral {
        vault_id: vault_id.clone(),
        amount: amount_collateral.amount(),
    })
    .dispatch(origin_of(nominator_id))
}

pub fn assert_nominate_collateral(vault_id: &VaultId, nominator_id: AccountId, amount_collateral: Amount<Runtime>) {
    assert_ok!(nominate_collateral(vault_id, nominator_id, amount_collateral));
}

pub fn withdraw_vault_collateral(vault_id: &VaultId, amount_collateral: Amount<Runtime>) -> DispatchResultWithPostInfo {
    Call::VaultRegistry(VaultRegistryCall::withdraw_collateral {
        currency_pair: vault_id.currencies.clone(),
        amount: amount_collateral.amount(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn withdraw_nominator_collateral(
    nominator_id: AccountId,
    vault_id: &VaultId,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::withdraw_collateral {
        vault_id: vault_id.clone(),
        amount: amount_collateral.amount(),
        index: None,
    })
    .dispatch(origin_of(nominator_id))
}

pub fn assert_withdraw_nominator_collateral(nominator_id: AccountId, vault_id: &VaultId, amount_dot: Amount<Runtime>) {
    assert_ok!(withdraw_nominator_collateral(nominator_id, vault_id, amount_dot));
}

pub fn assert_total_nominated_collateral_is(vault_id: &VaultId, amount_collateral: Amount<Runtime>) {
    let nominated_collateral = NominationPallet::get_total_nominated_collateral(vault_id).unwrap();
    assert_eq!(nominated_collateral, amount_collateral);
}

pub fn get_nominator_collateral(vault_id: &VaultId, nominator_id: AccountId) -> Amount<Runtime> {
    NominationPallet::get_nominator_collateral(vault_id, &nominator_id).unwrap()
}
