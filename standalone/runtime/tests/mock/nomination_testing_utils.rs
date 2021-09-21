use currency::Amount;
use vault_registry::DefaultVaultId;

use crate::{assert_eq, *};

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

pub const DEFAULT_GRIEFING_COLLATERAL: Amount<Runtime> = griefing(5_000);
pub const DEFAULT_BACKING_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_NOMINATION: u128 = 20_000;

pub const DEFAULT_VAULT_UNBONDING_PERIOD: u32 = 100;
pub const DEFAULT_NOMINATOR_UNBONDING_PERIOD: u32 = 50;

pub fn default_backing_collateral(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_BACKING_COLLATERAL, currency_id)
}

pub fn enable_nomination() {
    assert_ok!(Call::Nomination(NominationCall::set_nomination_enabled(true))
        .dispatch(<Runtime as frame_system::Config>::Origin::root()));
}

pub fn disable_nomination() {
    assert_ok!(Call::Nomination(NominationCall::set_nomination_enabled(false))
        .dispatch(<Runtime as frame_system::Config>::Origin::root()));
}

pub fn register_vault(currency_id: CurrencyId, vault: [u8; 32]) -> DispatchResultWithPostInfo {
    Call::VaultRegistry(VaultRegistryCall::register_vault(
        currency_id,
        DEFAULT_WRAPPED_CURRENCY,
        DEFAULT_BACKING_COLLATERAL,
        dummy_public_key(),
    ))
    .dispatch(origin_of(account_of(vault)))
}

pub fn assert_register_vault(currency_id: CurrencyId, vault: [u8; 32]) {
    assert_ok!(register_vault(currency_id, vault));
}

pub fn nomination_opt_in(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_in_to_nomination(
        vault_id.currencies.collateral,
        vault_id.currencies.wrapped,
    ))
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn assert_nomination_opt_in(vault_id: &DefaultVaultId<Runtime>) {
    assert_ok!(nomination_opt_in(vault_id));
}

pub fn nomination_opt_out(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::opt_out_of_nomination(
        vault_id.currencies.collateral,
        vault_id.currencies.wrapped,
    ))
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn nominate_collateral(
    vault_id: &DefaultVaultId<Runtime>,
    nominator_id: AccountId,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::deposit_collateral(
        vault_id.clone(),
        amount_collateral.amount(),
    ))
    .dispatch(origin_of(nominator_id))
}

pub fn assert_nominate_collateral(
    vault_id: &DefaultVaultId<Runtime>,
    nominator_id: AccountId,
    amount_collateral: Amount<Runtime>,
) {
    assert_ok!(nominate_collateral(vault_id, nominator_id, amount_collateral));
}

pub fn withdraw_vault_collateral(
    vault_id: &DefaultVaultId<Runtime>,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    Call::VaultRegistry(VaultRegistryCall::withdraw_collateral(
        vault_id.currencies.collateral,
        vault_id.currencies.wrapped,
        amount_collateral.amount(),
    ))
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn withdraw_nominator_collateral(
    nominator_id: AccountId,
    vault_id: &DefaultVaultId<Runtime>,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    Call::Nomination(NominationCall::withdraw_collateral(
        vault_id.clone(),
        amount_collateral.amount(),
        None,
    ))
    .dispatch(origin_of(nominator_id))
}

pub fn assert_withdraw_nominator_collateral(
    nominator_id: AccountId,
    vault_id: &DefaultVaultId<Runtime>,
    amount_dot: Amount<Runtime>,
) {
    assert_ok!(withdraw_nominator_collateral(nominator_id, vault_id, amount_dot));
}

pub fn assert_total_nominated_collateral_is(vault_id: &DefaultVaultId<Runtime>, amount_collateral: Amount<Runtime>) {
    let nominated_collateral = NominationPallet::get_total_nominated_collateral(vault_id).unwrap();
    assert_eq!(nominated_collateral, amount_collateral);
}

pub fn get_nominator_collateral(vault_id: &DefaultVaultId<Runtime>, nominator_id: AccountId) -> Amount<Runtime> {
    NominationPallet::get_nominator_collateral(vault_id, &nominator_id).unwrap()
}
