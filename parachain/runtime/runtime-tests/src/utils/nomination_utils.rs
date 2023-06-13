use currency::Amount;

use crate::setup::{assert_eq, *};

pub const USER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;
pub const VAULT2: [u8; 32] = CAROL;

pub const DEFAULT_BACKING_COLLATERAL: Balance = 1_000_000;
pub const DEFAULT_NOMINATION: Balance = 20_000;
pub const DEFAULT_NOMINATION_LIMIT: Balance = 1_000_000;

pub const DEFAULT_VAULT_UNBONDING_PERIOD: u32 = 100;
pub const DEFAULT_NOMINATOR_UNBONDING_PERIOD: u32 = 50;

pub const COMMISSION: f64 = 0.75;
pub const NOMINATOR_SHARE: f64 = 1.0 - COMMISSION;

pub fn default_backing_collateral(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_BACKING_COLLATERAL, currency_id)
}

pub fn enable_nomination() {
    assert_ok!(
        RuntimeCall::Nomination(NominationCall::set_nomination_enabled { enabled: true })
            .dispatch(<Runtime as frame_system::Config>::RuntimeOrigin::root())
    );
}

pub fn disable_nomination() {
    assert_ok!(
        RuntimeCall::Nomination(NominationCall::set_nomination_enabled { enabled: false })
            .dispatch(<Runtime as frame_system::Config>::RuntimeOrigin::root())
    );
}

pub fn nomination_opt_in(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    RuntimeCall::Nomination(NominationCall::opt_in_to_nomination {
        currency_pair: vault_id.currencies.clone(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn assert_nomination_opt_in(vault_id: &VaultId) {
    assert_ok!(nomination_opt_in(vault_id));
    assert_ok!(RuntimeCall::Nomination(NominationCall::set_nomination_limit {
        currency_pair: vault_id.currencies.clone(),
        limit: DEFAULT_NOMINATION_LIMIT
    })
    .dispatch(origin_of(vault_id.account_id.clone())));
}

pub fn nomination_opt_out(vault_id: &DefaultVaultId<Runtime>) -> DispatchResultWithPostInfo {
    RuntimeCall::Nomination(NominationCall::opt_out_of_nomination {
        currency_pair: vault_id.currencies.clone(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn nominate_collateral(
    vault_id: &VaultId,
    nominator_id: AccountId,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    RuntimeCall::Nomination(NominationCall::deposit_collateral {
        vault_id: vault_id.clone(),
        amount: amount_collateral.amount(),
    })
    .dispatch(origin_of(nominator_id))
}

pub fn assert_nominate_collateral(vault_id: &VaultId, nominator_id: AccountId, amount_collateral: Amount<Runtime>) {
    assert_eq!(vault_id.collateral_currency(), amount_collateral.currency());
    assert_ok!(nominate_collateral(vault_id, nominator_id, amount_collateral));
}

pub fn withdraw_vault_collateral(vault_id: &VaultId, amount_collateral: Amount<Runtime>) -> DispatchResultWithPostInfo {
    RuntimeCall::Nomination(NominationCall::withdraw_collateral {
        vault_id: vault_id.clone(),
        index: None,
        amount: amount_collateral.amount(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn withdraw_nominator_collateral(
    nominator_id: AccountId,
    vault_id: &VaultId,
    amount_collateral: Amount<Runtime>,
) -> DispatchResultWithPostInfo {
    RuntimeCall::Nomination(NominationCall::withdraw_collateral {
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

pub fn set_commission(vault_id: &VaultId, commission: FixedU128) {
    FeePallet::set_commission(
        origin_of(vault_id.account_id.clone()),
        vault_id.currencies.clone(),
        commission,
    )
    .unwrap();
}
