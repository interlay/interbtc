#![allow(dead_code)]

extern crate hex;

use bitcoin::types::TransactionInputSource;
pub use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::*,
};
pub use btc_relay::{BtcAddress, BtcPublicKey};
use currency::Amount;
use frame_support::traits::GenesisBuild;
pub use frame_support::{
    assert_err, assert_noop, assert_ok,
    dispatch::{DispatchError, DispatchResultWithPostInfo},
};
pub use interbtc_runtime_standalone::{
    token_distribution, AccountId, Balance, BlockNumber, Call, CurrencyId, EscrowAnnuityInstance,
    EscrowRewardsInstance, Event, GetNativeCurrencyId, GetRelayChainCurrencyId, GetWrappedCurrencyId, Runtime,
    TechnicalCommitteeInstance, VaultAnnuityInstance, VaultRewardsInstance, YEARS,
};
pub use mocktopus::mocking::*;
pub use orml_tokens::CurrencyAdapter;
pub use primitives::{
    CurrencyId::Token, VaultCurrencyPair, VaultId as PrimitiveVaultId, DOT, IBTC, INTR, KBTC, KINT, KSM,
};
use redeem::RedeemRequestStatus;
use staking::DefaultVaultCurrencyPair;
use vault_registry::types::UpdatableVault;

pub use issue::{types::IssueRequestExt, IssueRequest, IssueRequestStatus};
pub use oracle::OracleKey;
pub use redeem::{types::RedeemRequestExt, RedeemRequest};
pub use refund::RefundRequest;
pub use replace::{types::ReplaceRequestExt, ReplaceRequest};
pub use reward::Rewards;
pub use security::{ErrorCode, StatusCode};
pub use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
pub use sp_core::{H160, H256, U256};
pub use sp_runtime::{
    traits::{Dispatchable, One, Zero},
    AccountId32,
};
pub use sp_std::convert::TryInto;
use std::collections::BTreeMap;
pub use std::convert::TryFrom;
pub use vault_registry::{CurrencySource, DefaultVaultId, Vault, VaultStatus};

use self::redeem_testing_utils::USER_BTC_ADDRESS;

pub mod issue_testing_utils;
pub mod nomination_testing_utils;
pub mod redeem_testing_utils;
pub mod replace_testing_utils;
pub mod reward_testing_utils;

pub use pretty_assertions::assert_eq;

pub type VaultId = DefaultVaultId<Runtime>;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CAROL: [u8; 32] = [2u8; 32];
pub const DAVE: [u8; 32] = [10u8; 32];
pub const EVE: [u8; 32] = [11u8; 32];
pub const FRANK: [u8; 32] = [12u8; 32];
pub const GRACE: [u8; 32] = [13u8; 32];

pub const FAUCET: [u8; 32] = [128u8; 32];
pub const DUMMY: [u8; 32] = [255u8; 32];

pub const FUND_LIMIT_CEILING: Balance = 1_000_000_000_000_000_000;

pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;

pub const DEFAULT_USER_FREE_TOKENS: Amount<Runtime> = wrapped(10_000_000);
pub const DEFAULT_USER_LOCKED_TOKENS: Amount<Runtime> = wrapped(1000);

pub const DEFAULT_VAULT_TO_BE_ISSUED: Amount<Runtime> = wrapped(10_000);
pub const DEFAULT_VAULT_ISSUED: Amount<Runtime> = wrapped(100_000);
pub const DEFAULT_VAULT_TO_BE_REDEEMED: Amount<Runtime> = wrapped(20_000);
pub const DEFAULT_VAULT_TO_BE_REPLACED: Amount<Runtime> = wrapped(40_000);
pub const DEFAULT_VAULT_FREE_TOKENS: Amount<Runtime> = wrapped(0);

pub const DEFAULT_VAULT_GRIEFING_COLLATERAL: Amount<Runtime> = griefing(30_000);
pub const DEFAULT_VAULT_REPLACE_COLLATERAL: Amount<Runtime> = griefing(20_000);

pub const DEFAULT_GRIEFING_COLLATERAL: Amount<Runtime> = griefing(5_000);

pub fn default_user_free_balance(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(1_000_000, currency_id)
}
pub fn default_user_locked_balance(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(100_000, currency_id)
}

pub fn default_vault_backing_collateral(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(1_000_000, currency_id)
}
pub fn default_vault_free_balance(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(200_000, currency_id)
}

pub const CONFIRMATIONS: u32 = 6;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u32 = 2016;

pub type BTCRelayCall = btc_relay::Call<Runtime>;
pub type BTCRelayPallet = btc_relay::Pallet<Runtime>;
pub type BTCRelayError = btc_relay::Error<Runtime>;
pub type BTCRelayEvent = btc_relay::Event<Runtime>;

pub type TokensCall = orml_tokens::Call<Runtime>;
pub type TokensError = orml_tokens::Error<Runtime>;
pub type TokensPallet = orml_tokens::Pallet<Runtime>;

pub type CollateralCurrency = CurrencyAdapter<Runtime, GetRelayChainCurrencyId>;
pub type NativeCurrency = CurrencyAdapter<Runtime, GetNativeCurrencyId>;
pub type WrappedCurrency = CurrencyAdapter<Runtime, GetWrappedCurrencyId>;

pub type OracleCall = oracle::Call<Runtime>;
pub type OraclePallet = oracle::Pallet<Runtime>;

pub type FeeCall = fee::Call<Runtime>;
pub type FeeError = fee::Error<Runtime>;
pub type FeePallet = fee::Pallet<Runtime>;

pub type EscrowRewardsPallet = reward::Pallet<Runtime, EscrowRewardsInstance>;

pub type VaultRewardsPallet = reward::Pallet<Runtime, VaultRewardsInstance>;
pub type VaultStakingPallet = staking::Pallet<Runtime>;

pub type IssueCall = issue::Call<Runtime>;
pub type IssuePallet = issue::Pallet<Runtime>;
pub type IssueEvent = issue::Event<Runtime>;
pub type IssueError = issue::Error<Runtime>;

pub type MultisigCall = pallet_multisig::Call<Runtime>;
pub type MultisigPallet = pallet_multisig::Pallet<Runtime>;

pub type RefundCall = refund::Call<Runtime>;
pub type RefundPallet = refund::Pallet<Runtime>;
pub type RefundEvent = refund::Event<Runtime>;
pub type RefundError = refund::Error<Runtime>;

pub type RedeemCall = redeem::Call<Runtime>;
pub type RedeemPallet = redeem::Pallet<Runtime>;
pub type RedeemError = redeem::Error<Runtime>;
pub type RedeemEvent = redeem::Event<Runtime>;

pub type ReplaceCall = replace::Call<Runtime>;
pub type ReplaceEvent = replace::Event<Runtime>;
pub type ReplaceError = replace::Error<Runtime>;
pub type ReplacePallet = replace::Pallet<Runtime>;

pub type SecurityError = security::Error<Runtime>;
pub type SecurityPallet = security::Pallet<Runtime>;
pub type SecurityCall = security::Call<Runtime>;

pub type SudoCall = pallet_sudo::Call<Runtime>;

pub type RelayCall = relay::Call<Runtime>;
pub type RelayPallet = relay::Pallet<Runtime>;
pub type RelayError = relay::Error<Runtime>;

pub type SystemPallet = frame_system::Pallet<Runtime>;
pub type SystemError = frame_system::Error<Runtime>;

pub type VaultRegistryCall = vault_registry::Call<Runtime>;
pub type VaultRegistryError = vault_registry::Error<Runtime>;
pub type VaultRegistryPallet = vault_registry::Pallet<Runtime>;

pub type NominationCall = nomination::Call<Runtime>;
pub type NominationError = nomination::Error<Runtime>;
pub type NominationEvent = nomination::Event<Runtime>;
pub type NominationPallet = nomination::Pallet<Runtime>;

pub type EscrowCall = escrow::Call<Runtime>;
pub type EscrowError = escrow::Error<Runtime>;
pub type EscrowPallet = escrow::Pallet<Runtime>;

pub type UtilityCall = pallet_utility::Call<Runtime>;

pub type SchedulerCall = pallet_scheduler::Call<Runtime>;
pub type SchedulerPallet = pallet_scheduler::Pallet<Runtime>;

pub const DEFAULT_COLLATERAL_CURRENCY: <Runtime as orml_tokens::Config>::CurrencyId = Token(DOT);
pub const DEFAULT_WRAPPED_CURRENCY: <Runtime as orml_tokens::Config>::CurrencyId = Token(IBTC);
pub const DEFAULT_NATIVE_CURRENCY: <Runtime as orml_tokens::Config>::CurrencyId = Token(INTR);
pub const DEFAULT_GRIEFING_CURRENCY: <Runtime as orml_tokens::Config>::CurrencyId = DEFAULT_NATIVE_CURRENCY;

pub fn default_vault_id_of(hash: [u8; 32]) -> VaultId {
    VaultId {
        account_id: account_of(hash),
        currencies: DefaultVaultCurrencyPair::<Runtime> {
            collateral: DEFAULT_COLLATERAL_CURRENCY,
            wrapped: DEFAULT_WRAPPED_CURRENCY,
        },
    }
}

pub fn dummy_vault_id_of() -> VaultId {
    PrimitiveVaultId::new(account_of(BOB), DEFAULT_COLLATERAL_CURRENCY, DEFAULT_WRAPPED_CURRENCY)
}
pub fn vault_id_of(id: [u8; 32], collateral_currency: CurrencyId) -> VaultId {
    PrimitiveVaultId::new(account_of(id), collateral_currency, DEFAULT_WRAPPED_CURRENCY)
}

pub fn default_user_state() -> UserData {
    let mut balances = BTreeMap::new();
    for currency_id in iter_collateral_currencies() {
        balances.insert(
            currency_id,
            AccountData {
                free: default_user_free_balance(currency_id),
                locked: default_user_locked_balance(currency_id),
            },
        );
    }
    for currency_id in iter_native_currencies() {
        balances.insert(
            currency_id,
            AccountData {
                free: default_user_free_balance(currency_id),
                locked: default_user_locked_balance(currency_id),
            },
        );
    }
    for currency_id in iter_wrapped_currencies() {
        balances.insert(
            currency_id,
            AccountData {
                free: Amount::new(DEFAULT_USER_FREE_TOKENS.amount(), currency_id),
                locked: Amount::new(DEFAULT_USER_LOCKED_TOKENS.amount(), currency_id),
            },
        );
    }
    UserData { balances }
}

pub fn default_vault_state(vault_id: &VaultId) -> CoreVaultData {
    CoreVaultData {
        to_be_issued: vault_id.wrapped(DEFAULT_VAULT_TO_BE_ISSUED.amount()),
        issued: vault_id.wrapped(DEFAULT_VAULT_ISSUED.amount()),
        to_be_redeemed: vault_id.wrapped(DEFAULT_VAULT_TO_BE_REDEEMED.amount()),
        to_be_replaced: vault_id.wrapped(DEFAULT_VAULT_TO_BE_REPLACED.amount()),
        griefing_collateral: DEFAULT_VAULT_GRIEFING_COLLATERAL,
        replace_collateral: DEFAULT_VAULT_REPLACE_COLLATERAL,
        backing_collateral: default_vault_backing_collateral(vault_id.collateral_currency()),
        free_balance: iter_all_currencies()
            .map(|x| (x, default_vault_free_balance(x)))
            .collect(),
        liquidated_collateral: Amount::new(0, vault_id.collateral_currency()),
    }
}

pub fn default_liquidation_vault_state(currency_pair: &DefaultVaultCurrencyPair<Runtime>) -> LiquidationVaultData {
    LiquidationVaultData::get_default(currency_pair)
}

pub fn premium_redeem_request(
    user_to_redeem: Amount<Runtime>,
    vault_id: &VaultId,
    user: [u8; 32],
) -> RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId> {
    let redeem_fee = FeePallet::get_redeem_fee(&user_to_redeem).unwrap();
    let burned_tokens = user_to_redeem - redeem_fee;
    let inclusion_fee = RedeemPallet::get_current_inclusion_fee(vault_id.wrapped_currency()).unwrap();
    let premium_redeem_fee = FeePallet::get_premium_redeem_fee(&(burned_tokens - inclusion_fee)).unwrap();

    RedeemRequest {
        premium: premium_redeem_fee.amount(),
        ..default_redeem_request(user_to_redeem, vault_id, user)
    }
}

pub fn default_redeem_request(
    user_to_redeem: Amount<Runtime>,
    vault_id: &VaultId,
    user: [u8; 32],
) -> RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId> {
    let redeem_fee = FeePallet::get_redeem_fee(&user_to_redeem).unwrap();
    let burned_tokens = user_to_redeem - redeem_fee;
    let inclusion_fee = RedeemPallet::get_current_inclusion_fee(vault_id.wrapped_currency()).unwrap();
    let redeem_period = RedeemPallet::redeem_period();
    let btc_height = BTCRelayPallet::get_best_block_height();
    let opentime = SecurityPallet::active_block_number();

    RedeemRequest {
        vault: vault_id.clone(),
        opentime,
        fee: redeem_fee.amount(),
        transfer_fee_btc: inclusion_fee.amount(),
        amount_btc: (burned_tokens - inclusion_fee).amount(),
        premium: 0,
        period: redeem_period,
        redeemer: account_of(user),
        btc_address: USER_BTC_ADDRESS,
        btc_height,
        status: RedeemRequestStatus::Pending,
    }
}

pub fn root() -> <Runtime as frame_system::Config>::Origin {
    <Runtime as frame_system::Config>::Origin::root()
}

pub fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Config>::Origin {
    <Runtime as frame_system::Config>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccountData {
    pub free: Amount<Runtime>,
    pub locked: Amount<Runtime>,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct UserData {
    // note: we use BTreeMap such that the debug print output is sorted, for easier diffing
    pub balances: BTreeMap<CurrencyId, AccountData>,
}

pub trait Collateral {
    fn collateral(&self, amount: Balance) -> Amount<Runtime>;
}

impl Collateral for VaultId {
    fn collateral(&self, amount: Balance) -> Amount<Runtime> {
        Amount::new(amount, self.collateral_currency())
    }
}

pub trait Wrapped {
    fn wrapped(&self, amount: Balance) -> Amount<Runtime>;
}

impl Wrapped for VaultId {
    fn wrapped(&self, amount: Balance) -> Amount<Runtime> {
        Amount::new(amount, self.wrapped_currency())
    }
}

pub fn iter_currency_pairs() -> impl Iterator<Item = DefaultVaultCurrencyPair<Runtime>> {
    iter_collateral_currencies().flat_map(|collateral_id| {
        iter_wrapped_currencies().map(move |wrapped_id| VaultCurrencyPair {
            collateral: collateral_id,
            wrapped: wrapped_id,
        })
    })
}

pub fn iter_collateral_currencies() -> impl Iterator<Item = CurrencyId> {
    vec![Token(DOT), Token(KSM)].into_iter()
}

pub fn iter_native_currencies() -> impl Iterator<Item = CurrencyId> {
    vec![Token(INTR), Token(KINT)].into_iter()
}

pub fn iter_wrapped_currencies() -> impl Iterator<Item = CurrencyId> {
    vec![Token(IBTC), Token(KBTC)].into_iter()
}

pub fn iter_all_currencies() -> impl Iterator<Item = CurrencyId> {
    iter_collateral_currencies()
        .chain(iter_native_currencies())
        .chain(iter_wrapped_currencies())
}

impl UserData {
    pub fn get(id: [u8; 32]) -> Self {
        let account_id = account_of(id);
        let mut hash_map = BTreeMap::new();

        for currency_id in iter_all_currencies() {
            let free = currency::get_free_balance::<Runtime>(currency_id, &account_id);
            let locked = currency::get_reserved_balance::<Runtime>(currency_id, &account_id);
            hash_map.insert(currency_id, AccountData { free, locked });
        }

        Self { balances: hash_map }
    }

    pub fn force_to(id: [u8; 32], new: Self) -> Self {
        let old = Self::get(id);
        let account_id = account_of(id);

        // some sanity checks:
        assert!(old
            .balances
            .iter()
            .all(|(currency_id, _)| new.balances.contains_key(currency_id)));
        assert!(new
            .balances
            .iter()
            .all(|(currency_id, _)| old.balances.contains_key(currency_id)));
        assert!(old
            .balances
            .iter()
            .chain(new.balances.iter())
            .all(
                |(currency_id, AccountData { free, locked })| free.currency() == *currency_id
                    && locked.currency() == *currency_id
            ));

        // Clear collateral currencies:
        for (_currency_id, balance) in old.balances.iter() {
            balance.free.transfer(&account_id, &account_of(FAUCET)).unwrap();
            balance.locked.burn_from(&account_id).unwrap();
        }

        for (_, balance) in new.balances.iter() {
            // set free balance:
            balance.free.transfer(&account_of(FAUCET), &account_id).unwrap();

            // set locked balance:
            balance.locked.transfer(&account_of(FAUCET), &account_id).unwrap();
            balance.locked.lock_on(&account_id).unwrap();
        }

        // sanity check:
        assert_eq!(Self::get(id), new);

        new
    }
}

#[derive(Debug, Clone)]
pub struct FeePool {
    pub vault_rewards: BTreeMap<CurrencyId, Amount<Runtime>>,
}

impl FeePool {
    pub fn rewards_for(&mut self, vault_id: &VaultId) -> &mut Amount<Runtime> {
        self.vault_rewards.get_mut(&vault_id.wrapped_currency()).unwrap()
    }
}
impl Default for FeePool {
    fn default() -> Self {
        Self {
            vault_rewards: iter_wrapped_currencies()
                .map(|currency_id| (currency_id, Amount::zero(currency_id)))
                .collect(),
        }
    }
}
fn abs_difference<T: std::ops::Sub<Output = T> + PartialOrd>(x: T, y: T) -> T {
    if x < y {
        y - x
    } else {
        x - y
    }
}

impl PartialEq for FeePool {
    fn eq(&self, rhs: &Self) -> bool {
        assert!(self
            .vault_rewards
            .iter()
            .all(|(currency_id, _)| rhs.vault_rewards.contains_key(currency_id)));
        assert!(rhs
            .vault_rewards
            .iter()
            .all(|(currency_id, _)| self.vault_rewards.contains_key(currency_id)));
        self.vault_rewards.iter().all(|(currency_id, lhs)| {
            abs_difference(lhs.clone(), rhs.vault_rewards.get(currency_id).unwrap().clone())
                <= Amount::new(1, *currency_id)
        })
    }
}

impl FeePool {
    pub fn get() -> Self {
        Self {
            vault_rewards: iter_wrapped_currencies()
                .map(|currency_id| {
                    (
                        currency_id,
                        Amount::new(
                            VaultRewardsPallet::get_total_rewards(currency_id)
                                .unwrap()
                                .try_into()
                                .unwrap(),
                            currency_id,
                        ),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CoreVaultData {
    pub to_be_issued: Amount<Runtime>,
    pub issued: Amount<Runtime>,
    pub to_be_redeemed: Amount<Runtime>,
    pub backing_collateral: Amount<Runtime>,
    pub griefing_collateral: Amount<Runtime>,
    pub liquidated_collateral: Amount<Runtime>,
    // note: we use BTreeMap such that the debug print output is sorted, for easier diffing
    pub free_balance: BTreeMap<CurrencyId, Amount<Runtime>>,
    pub to_be_replaced: Amount<Runtime>,
    pub replace_collateral: Amount<Runtime>,
}

impl CoreVaultData {
    pub fn get_default(vault_id: &VaultId) -> Self {
        Self {
            to_be_issued: vault_id.wrapped(0),
            issued: vault_id.wrapped(0),
            to_be_redeemed: vault_id.wrapped(0),
            to_be_replaced: vault_id.wrapped(0),
            griefing_collateral: griefing(0),
            replace_collateral: griefing(0),
            backing_collateral: Amount::new(0, vault_id.collateral_currency()),
            free_balance: iter_all_currencies().map(|x| (x, Amount::new(0, x))).collect(),
            liquidated_collateral: Amount::new(0, vault_id.collateral_currency()),
        }
    }
}

impl CoreVaultData {
    pub fn vault(vault_id: VaultId) -> Self {
        let vault = VaultRegistryPallet::get_vault_from_id(&vault_id).unwrap();
        Self {
            to_be_issued: Amount::new(vault.to_be_issued_tokens, vault_id.wrapped_currency()),
            issued: Amount::new(vault.issued_tokens, vault_id.wrapped_currency()),
            to_be_redeemed: Amount::new(vault.to_be_redeemed_tokens, vault_id.wrapped_currency()),
            to_be_replaced: Amount::new(vault.to_be_replaced_tokens, vault_id.wrapped_currency()),
            backing_collateral: CurrencySource::<Runtime>::Collateral(vault_id.clone())
                .current_balance(vault_id.currencies.collateral)
                .unwrap(),
            griefing_collateral: CurrencySource::<Runtime>::ActiveReplaceCollateral(vault_id.clone())
                .current_balance(<Runtime as vault_registry::Config>::GetGriefingCollateralCurrencyId::get())
                .unwrap()
                + CurrencySource::<Runtime>::AvailableReplaceCollateral(vault_id.clone())
                    .current_balance(vault_id.currencies.collateral)
                    .unwrap(),
            liquidated_collateral: Amount::new(vault.liquidated_collateral, vault_id.currencies.collateral),
            free_balance: iter_all_currencies()
                .map(|currency_id| {
                    (
                        currency_id,
                        CurrencySource::<Runtime>::FreeBalance(vault_id.account_id.clone())
                            .current_balance(currency_id)
                            .unwrap(),
                    )
                })
                .collect(),
            replace_collateral: Amount::new(
                vault.replace_collateral,
                <Runtime as vault_registry::Config>::GetGriefingCollateralCurrencyId::get(),
            ),
        }
    }

    pub fn collateral_currency(&self) -> CurrencyId {
        self.backing_collateral.currency()
    }

    pub fn force_to(vault_id: &VaultId, state: CoreVaultData) {
        VaultRegistryPallet::collateral_integrity_check();

        // check that all have same currency
        assert_eq!(vault_id.wrapped_currency(), state.to_be_issued.currency());
        assert_eq!(vault_id.wrapped_currency(), state.issued.currency());
        assert_eq!(vault_id.wrapped_currency(), state.to_be_redeemed.currency());
        assert_eq!(vault_id.wrapped_currency(), state.to_be_replaced.currency());

        // replace collateral is part of griefing collateral, so it needs to smaller or equal
        assert!(state.griefing_collateral >= state.replace_collateral);
        assert!(state.to_be_replaced + state.to_be_redeemed <= state.issued);

        // register vault if not yet registered
        try_register_vault(Amount::new(100, state.collateral_currency()), &vault_id);

        // todo: check that currency did not change
        let currency_id = VaultRegistryPallet::get_vault_from_id(&vault_id)
            .unwrap()
            .id
            .collateral_currency();
        VaultRegistryPallet::collateral_integrity_check();

        // temporarily give vault a lot of backing collateral so we can set issued & to-be-issued to whatever we want
        VaultRegistryPallet::transfer_funds(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Collateral(vault_id.clone()),
            &Amount::new(FUND_LIMIT_CEILING / 10, currency_id),
        )
        .unwrap();
        VaultRegistryPallet::collateral_integrity_check();

        let current = CoreVaultData::vault(vault_id.clone());

        // set all token types to 0
        assert_ok!(VaultRegistryPallet::decrease_to_be_issued_tokens(
            &vault_id,
            &current.to_be_issued
        ));
        assert_ok!(VaultRegistryPallet::decrease_to_be_redeemed_tokens(
            &vault_id,
            &current.to_be_redeemed
        ));
        assert_ok!(VaultRegistryPallet::try_increase_to_be_redeemed_tokens(
            &vault_id,
            &current.issued
        ));
        assert_ok!(VaultRegistryPallet::decrease_tokens(
            &vault_id,
            &account_of(DUMMY),
            &current.issued,
        ));
        assert_ok!(VaultRegistryPallet::decrease_to_be_replaced_tokens(
            &vault_id,
            &current.to_be_replaced,
        ));

        // set to-be-issued
        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &vault_id,
            &state.to_be_issued
        ));
        // set issued (2 steps)
        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &vault_id,
            &state.issued
        ));
        assert_ok!(VaultRegistryPallet::issue_tokens(&vault_id, &state.issued));
        // set to-be-redeemed
        assert_ok!(VaultRegistryPallet::try_increase_to_be_redeemed_tokens(
            &vault_id,
            &state.to_be_redeemed
        ));
        // set to-be-replaced:
        assert_ok!(VaultRegistryPallet::try_increase_to_be_replaced_tokens(
            &vault_id,
            &state.to_be_replaced,
        ));
        VaultRegistryPallet::collateral_integrity_check();

        // clear all balances
        for currency_id in iter_all_currencies() {
            VaultRegistryPallet::transfer_funds(
                CurrencySource::FreeBalance(vault_id.account_id.clone()),
                CurrencySource::FreeBalance(account_of(FAUCET)),
                &CurrencySource::<Runtime>::FreeBalance(vault_id.account_id.clone())
                    .current_balance(currency_id)
                    .unwrap(),
            )
            .unwrap();

            VaultRegistryPallet::transfer_funds(
                CurrencySource::FreeBalance(account_of(FAUCET)),
                CurrencySource::FreeBalance(vault_id.account_id.clone()),
                &state
                    .free_balance
                    .get(&currency_id)
                    .copied()
                    .unwrap_or(Amount::zero(currency_id)),
            )
            .unwrap();
            VaultRegistryPallet::collateral_integrity_check();
        }
        VaultRegistryPallet::collateral_integrity_check();

        VaultRegistryPallet::transfer_funds(
            CurrencySource::ActiveReplaceCollateral(vault_id.clone()),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            &CurrencySource::<Runtime>::ActiveReplaceCollateral(vault_id.clone())
                .current_balance(<Runtime as vault_registry::Config>::GetGriefingCollateralCurrencyId::get())
                .unwrap(),
        )
        .unwrap();
        VaultRegistryPallet::collateral_integrity_check();

        VaultRegistryPallet::transfer_funds(
            CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            &CurrencySource::<Runtime>::AvailableReplaceCollateral(vault_id.clone())
                .current_balance(<Runtime as vault_registry::Config>::GetGriefingCollateralCurrencyId::get())
                .unwrap(),
        )
        .unwrap();

        VaultRegistryPallet::collateral_integrity_check();

        // vault's backing collateral was temporarily increased - reset to 0
        VaultRegistryPallet::transfer_funds(
            CurrencySource::Collateral(vault_id.clone()),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            &CurrencySource::<Runtime>::Collateral(vault_id.clone())
                .current_balance(currency_id)
                .unwrap(),
        )
        .unwrap();

        // set backing and griefing collateral
        VaultRegistryPallet::transfer_funds(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Collateral(vault_id.clone()),
            &state.backing_collateral,
        )
        .unwrap();
        VaultRegistryPallet::transfer_funds(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
            &state.replace_collateral,
        )
        .unwrap();

        VaultRegistryPallet::transfer_funds(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::ActiveReplaceCollateral(vault_id.clone()),
            &(state.griefing_collateral - state.replace_collateral),
        )
        .unwrap();

        VaultRegistryPallet::collateral_integrity_check();

        // check that we achieved the desired state
        assert_eq!(CoreVaultData::vault(vault_id.clone()), state);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SingleLiquidationVault {
    pub to_be_issued: Amount<Runtime>,
    pub issued: Amount<Runtime>,
    pub to_be_redeemed: Amount<Runtime>,
    pub collateral: Amount<Runtime>,
}

impl SingleLiquidationVault {
    fn zero(currency_pair: &DefaultVaultCurrencyPair<Runtime>) -> Self {
        Self {
            to_be_issued: Amount::new(0, currency_pair.wrapped),
            issued: Amount::new(0, currency_pair.wrapped),
            to_be_redeemed: Amount::new(0, currency_pair.wrapped),
            collateral: Amount::new(0, currency_pair.collateral),
        }
    }
    fn get_default(currency_pair: &DefaultVaultCurrencyPair<Runtime>) -> Self {
        Self {
            to_be_issued: Amount::new(123124, currency_pair.wrapped),
            issued: Amount::new(2131231, currency_pair.wrapped),
            to_be_redeemed: Amount::new(12323, currency_pair.wrapped),
            collateral: Amount::new(2451241, currency_pair.collateral),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct LiquidationVaultData {
    // note: we use BTreeMap such that the debug print output is sorted, for easier diffing
    liquidation_vaults: BTreeMap<DefaultVaultCurrencyPair<Runtime>, SingleLiquidationVault>,
}

impl LiquidationVaultData {
    pub fn get() -> Self {
        let liquidation_vaults = iter_currency_pairs()
            .map(|currency_pair| {
                let vault = VaultRegistryPallet::get_liquidation_vault(&currency_pair);
                let data = SingleLiquidationVault {
                    to_be_issued: Amount::new(vault.to_be_issued_tokens, currency_pair.wrapped),
                    issued: Amount::new(vault.issued_tokens, currency_pair.wrapped),
                    to_be_redeemed: Amount::new(vault.to_be_redeemed_tokens, currency_pair.wrapped),
                    collateral: CurrencySource::<Runtime>::LiquidationVault(currency_pair.clone())
                        .current_balance(currency_pair.collateral)
                        .unwrap(),
                };
                (currency_pair, data)
            })
            .collect();
        Self { liquidation_vaults }
    }

    pub fn with_currency(&mut self, currency_pair: &DefaultVaultCurrencyPair<Runtime>) -> &mut SingleLiquidationVault {
        self.liquidation_vaults.get_mut(currency_pair).unwrap()
    }

    pub fn get_default(currency_pair: &DefaultVaultCurrencyPair<Runtime>) -> Self {
        let mut ret = Self {
            liquidation_vaults: BTreeMap::new(),
        };
        for pair in iter_currency_pairs() {
            if &pair == currency_pair {
                ret.liquidation_vaults
                    .insert(pair.clone(), SingleLiquidationVault::zero(&pair));
            } else {
                ret.liquidation_vaults
                    .insert(pair.clone(), SingleLiquidationVault::get_default(&pair));
            }
        }
        ret
    }

    pub fn force_to(self) {
        let current = Self::get();

        for (currency_pair, target) in self.liquidation_vaults.iter() {
            let current = &current.liquidation_vaults[currency_pair];
            let mut liquidation_vault = VaultRegistryPallet::get_rich_liquidation_vault(&currency_pair);
            liquidation_vault
                .increase_issued(&(target.issued - current.issued))
                .unwrap();
            liquidation_vault
                .increase_to_be_issued(&(target.to_be_issued - current.to_be_issued))
                .unwrap();
            liquidation_vault
                .increase_to_be_redeemed(&(target.to_be_redeemed - current.to_be_redeemed))
                .unwrap();
            VaultRegistryPallet::transfer_funds(
                CurrencySource::FreeBalance(account_of(FAUCET)),
                CurrencySource::LiquidationVault(currency_pair.clone()),
                &target.collateral,
            )
            .unwrap();
        }
        let result = Self::get();
        assert_eq!(result, self);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CoreNominatorData {
    pub collateral_to_be_withdrawn: Amount<Runtime>,
}

// impl Default for CoreNominatorData {
//     fn default() -> Self {
//         CoreNominatorData {
//             collateral_to_be_withdrawn: 0,
//         }
//     }
// }

impl CoreNominatorData {}

#[derive(Debug, PartialEq, Clone)]
pub struct ParachainState {
    user: UserData,
    vault: CoreVaultData,
    liquidation_vault: LiquidationVaultData,
    fee_pool: FeePool,
}

impl ParachainState {
    pub fn get_default(vault_id: &VaultId) -> Self {
        Self {
            user: default_user_state(),
            vault: default_vault_state(vault_id),
            liquidation_vault: default_liquidation_vault_state(&vault_id.currencies),
            fee_pool: Default::default(),
        }
    }
}

impl ParachainState {
    pub fn get(vault_id: &VaultId) -> Self {
        Self {
            user: UserData::get(ALICE),
            vault: CoreVaultData::vault(vault_id.clone()),
            liquidation_vault: LiquidationVaultData::get(),
            fee_pool: FeePool::get(),
        }
    }

    pub fn with_changes(
        &self,
        f: impl FnOnce(&mut UserData, &mut CoreVaultData, &mut LiquidationVaultData, &mut FeePool),
    ) -> Self {
        let mut state = self.clone();
        f(
            &mut state.user,
            &mut state.vault,
            &mut state.liquidation_vault,
            &mut state.fee_pool,
        );
        state
    }
}

// todo: merge with ParachainState
#[derive(Debug, PartialEq, Clone)]
pub struct ParachainTwoVaultState {
    pub vault1: CoreVaultData,
    pub vault2: CoreVaultData,
    pub liquidation_vault: LiquidationVaultData,
}

impl ParachainTwoVaultState {
    pub fn get_default(old_vault_id: &VaultId, new_vault_id: &VaultId) -> Self {
        Self {
            vault1: default_vault_state(old_vault_id),
            vault2: default_vault_state(new_vault_id),
            liquidation_vault: default_liquidation_vault_state(&old_vault_id.currencies),
        }
    }
}

impl ParachainTwoVaultState {
    pub fn get(old_vault_id: &VaultId, new_vault_id: &VaultId) -> Self {
        Self {
            vault1: CoreVaultData::vault(old_vault_id.clone()),
            vault2: CoreVaultData::vault(new_vault_id.clone()),
            liquidation_vault: LiquidationVaultData::get(),
        }
    }

    pub fn with_changes(
        &self,
        f: impl FnOnce(&mut CoreVaultData, &mut CoreVaultData, &mut LiquidationVaultData),
    ) -> Self {
        let mut state = self.clone();
        f(&mut state.vault1, &mut state.vault2, &mut state.liquidation_vault);
        state
    }
}

pub fn liquidate_vault(vault_id: &VaultId) {
    assert_ok!(OraclePallet::_set_exchange_rate(
        vault_id.currencies.collateral,
        FixedU128::checked_from_integer(10_000_000_000).unwrap()
    ));
    assert_ok!(VaultRegistryPallet::liquidate_vault(&vault_id));
    assert_ok!(OraclePallet::_set_exchange_rate(
        vault_id.currencies.collateral,
        FixedU128::checked_from_integer(1).unwrap()
    ));
}

pub fn set_default_thresholds() {
    let secure = FixedU128::checked_from_rational(150, 100).unwrap();
    let premium = FixedU128::checked_from_rational(135, 100).unwrap();
    let liquidation = FixedU128::checked_from_rational(110, 100).unwrap();

    for collateral_id in iter_collateral_currencies() {
        for wrapped_id in iter_wrapped_currencies() {
            let currency_pair = VaultCurrencyPair {
                collateral: collateral_id,
                wrapped: wrapped_id,
            };
            VaultRegistryPallet::_set_secure_collateral_threshold(currency_pair.clone(), secure);
            VaultRegistryPallet::_set_premium_redeem_threshold(currency_pair.clone(), premium);
            VaultRegistryPallet::_set_liquidation_collateral_threshold(currency_pair.clone(), liquidation);
        }
    }
}

pub fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

pub fn register_vault_with_public_key(vault_id: &VaultId, collateral: Amount<Runtime>, public_key: BtcPublicKey) {
    if VaultRegistryPallet::get_bitcoin_public_key(&vault_id.account_id).is_err() {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_public_key { public_key })
                .dispatch(origin_of(vault_id.account_id.clone()))
        );
    }
    assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault {
        currency_pair: vault_id.currencies.clone(),
        collateral: collateral.amount(),
    })
    .dispatch(origin_of(vault_id.account_id.clone())));
}

pub fn register_vault(vault_id: &VaultId, collateral: Amount<Runtime>) {
    register_vault_with_public_key(vault_id, collateral, dummy_public_key());
}

pub fn get_register_vault_result(vault_id: &VaultId, collateral: Amount<Runtime>) -> DispatchResultWithPostInfo {
    assert_eq!(vault_id.collateral_currency(), collateral.currency());
    if VaultRegistryPallet::get_bitcoin_public_key(&vault_id.account_id).is_err() {
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_public_key {
            public_key: dummy_public_key()
        })
        .dispatch(origin_of(vault_id.account_id.clone())));
    }
    Call::VaultRegistry(VaultRegistryCall::register_vault {
        currency_pair: vault_id.currencies.clone(),
        collateral: collateral.amount(),
    })
    .dispatch(origin_of(vault_id.account_id.clone()))
}

pub fn try_register_vault(collateral: Amount<Runtime>, vault_id: &VaultId) {
    if VaultRegistryPallet::get_vault_from_id(vault_id).is_err() {
        register_vault(&vault_id, collateral);
    };
}

pub fn force_issue_tokens(user: [u8; 32], vault: [u8; 32], collateral: Amount<Runtime>, tokens: Amount<Runtime>) {
    // register the vault
    register_vault(&default_vault_id_of(vault), collateral);

    // increase to be issued tokens
    assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
        &default_vault_id_of(vault),
        &tokens
    ));

    // issue tokens
    assert_ok!(VaultRegistryPallet::issue_tokens(&default_vault_id_of(vault), &tokens));

    // mint tokens to the user
    assert_ok!(tokens.mint_to(&user.into()));
}

pub fn required_collateral_for_issue(issued_tokens: Amount<Runtime>, currency_id: CurrencyId) -> Amount<Runtime> {
    let fee_amount_btc = FeePallet::get_issue_fee(&issued_tokens).unwrap();
    let total_amount_btc = issued_tokens + fee_amount_btc;
    VaultRegistryPallet::get_required_collateral_for_wrapped(&total_amount_btc, currency_id).unwrap()
}

pub fn assert_store_main_chain_header_event(block_height: u32, block_hash: H256Le, relayer_id: AccountId) {
    let store_event = Event::BTCRelay(BTCRelayEvent::StoreMainChainHeader {
        block_height,
        block_hash,
        relayer_id,
    });

    // store only main chain header
    SystemPallet::assert_has_event(store_event);
    //TODO: add checks for actual chain state
}

pub fn assert_store_fork_header_event(chain_id: u32, fork_height: u32, block_hash: H256Le, relayer_id: AccountId) {
    let store_event = Event::BTCRelay(BTCRelayEvent::StoreForkHeader {
        chain_id,
        fork_height,
        block_hash,
        relayer_id,
    });

    // store only fork header
    SystemPallet::assert_has_event(store_event);
    //TODO: add checks for actual chain state
}

pub fn assert_fork_ahead_of_main_chain_event(main_chain_height: u32, fork_height: u32, fork_id: u32) {
    let store_event = Event::BTCRelay(BTCRelayEvent::ForkAheadOfMainChain {
        main_chain_height,
        fork_height,
        fork_id,
    });

    // store only fork header
    SystemPallet::assert_has_event(store_event);
    //TODO: add checks for actual chain state
}

pub fn assert_chain_reorg_event(new_chain_tip_hash: H256Le, new_chain_tip_height: u32, fork_depth: u32) {
    let store_event = Event::BTCRelay(BTCRelayEvent::ChainReorg {
        new_chain_tip_hash,
        new_chain_tip_height,
        fork_depth,
    });

    // ensure that chain reorg happened
    SystemPallet::assert_has_event(store_event);
    //TODO: add checks for actual chain state
}

pub fn mine_blocks(blocks: u32) {
    let start_height = BTCRelayPallet::get_best_block_height();
    TransactionGenerator::new().with_confirmations(blocks).mine();
    let end_height = BTCRelayPallet::get_best_block_height();
    // sanity check
    assert_eq!(end_height, start_height + blocks);
}

#[derive(Default, Clone, Debug)]
pub struct TransactionGenerator {
    coinbase_destination: BtcAddress,
    inputs: Vec<(Transaction, u32, Option<BtcPublicKey>)>,
    outputs: Vec<(BtcAddress, Balance)>,
    return_data: Vec<H256>,
    script: Vec<u8>,
    confirmations: u32,
    relayer: Option<[u8; 32]>,
}

impl TransactionGenerator {
    pub fn new() -> Self {
        Self {
            relayer: None,
            coinbase_destination: BtcAddress::P2PKH(H160::from([0; 20])),
            confirmations: 7,
            outputs: vec![(BtcAddress::P2PKH(H160::from([0; 20])), 100)],
            script: vec![
                0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210, 186, 21, 187, 98, 38,
                255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232, 168, 2, 32, 72, 126, 179, 207, 142, 8, 99,
                8, 32, 78, 244, 166, 106, 160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3,
                120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247, 165, 78, 111, 80, 79, 50,
                200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51, 33, 2, 128, 200, 184, 235, 148,
                25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7, 84, 210, 85, 156, 238, 77, 97, 188, 240,
                162, 197, 105, 62, 82, 174,
            ],
            return_data: vec![],
            ..Default::default()
        }
    }

    pub fn with_inputs(&mut self, inputs: Vec<(Transaction, u32, Option<BtcPublicKey>)>) -> &mut Self {
        self.inputs = inputs;
        self
    }
    pub fn with_outputs(&mut self, outputs: Vec<(BtcAddress, Amount<Runtime>)>) -> &mut Self {
        self.outputs = outputs
            .iter()
            .map(|output| {
                let (address, amount) = output;
                (*address, amount.amount())
            })
            .collect();
        self
    }

    pub fn with_coinbase_destination(&mut self, coinbase_destination: BtcAddress) -> &mut Self {
        self.coinbase_destination = coinbase_destination;
        self
    }

    pub fn with_op_return(&mut self, op_returns: Vec<H256>) -> &mut Self {
        self.return_data = op_returns;
        self
    }
    pub fn with_script(&mut self, script: &[u8]) -> &mut Self {
        self.script = script.to_vec();
        self
    }
    pub fn with_confirmations(&mut self, confirmations: u32) -> &mut Self {
        self.confirmations = confirmations;
        self
    }
    pub fn with_relayer(&mut self, relayer: Option<[u8; 32]>) -> &mut Self {
        self.relayer = relayer;
        self
    }
    pub fn mine(&self) -> (H256Le, u32, Vec<u8>, Vec<u8>, Transaction) {
        let mut height = BTCRelayPallet::get_best_block_height() + 1;
        let extra_confirmations = self.confirmations - 1;

        let mut transaction_builder = TransactionBuilder::new();
        transaction_builder.with_version(2);

        if self.inputs.len() == 0 {
            // initialize BTC Relay with one block
            let init_block = BlockBuilder::new()
                .with_version(4)
                .with_coinbase(&self.coinbase_destination, 50, 3)
                .with_timestamp(1588813835)
                .mine(U256::from(2).pow(254.into()))
                .unwrap();

            let raw_init_block_header = RawBlockHeader::from_bytes(&init_block.header.try_format().unwrap())
                .expect("could not serialize block header");
            let init_block_header = BTCRelayPallet::parse_raw_block_header(&raw_init_block_header).unwrap();

            match BTCRelayPallet::initialize(account_of(ALICE), init_block_header, height) {
                Ok(_) => {}
                Err(e) if e == BTCRelayError::AlreadyInitialized.into() => {}
                _ => panic!("Failed to initialize btc relay"),
            }

            height = BTCRelayPallet::get_best_block_height() + 1;

            transaction_builder.add_input(
                TransactionInputBuilder::new()
                    .with_script(&self.script)
                    .with_source(TransactionInputSource::FromOutput(init_block.transactions[0].hash(), 0))
                    .build(),
            );
        }

        for input in self.inputs.clone().into_iter() {
            let (transaction, output_index, public_key) = input;
            let tmp_script_sig;
            let script = match public_key {
                Some(val) => {
                    tmp_script_sig = val.to_p2pkh_script_sig(vec![1; 32]);
                    tmp_script_sig.as_bytes()
                }
                None => &self.script,
            };
            transaction_builder.add_input(
                TransactionInputBuilder::new()
                    .with_script(script)
                    .with_source(TransactionInputSource::FromOutput(
                        transaction.hash(),
                        output_index.clone(),
                    ))
                    .build(),
            );
        }

        for output in self.outputs.iter() {
            let (address, amount) = output;
            transaction_builder.add_output(TransactionOutput::payment(amount.clone() as i64, &address));
        }

        for op_return_data in self.return_data.iter() {
            transaction_builder.add_output(TransactionOutput::op_return(0, op_return_data.as_bytes()));
        }

        let transaction = transaction_builder.build();

        let prev_hash = BTCRelayPallet::get_best_block();
        let block = BlockBuilder::new()
            .with_previous_hash(prev_hash)
            .with_version(4)
            .with_coinbase(&self.coinbase_destination, 50, 3)
            .with_timestamp(1588814835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let raw_block_header =
            RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).expect("could not serialize block header");

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&[tx_id]).unwrap();
        let bytes_proof = proof.try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        self.relay(height, &block, raw_block_header);

        // Mine six new blocks to get over required confirmations
        let mut prev_block_hash = block.header.hash;
        let mut timestamp = 1588814835;
        for _ in 0..extra_confirmations {
            height += 1;
            timestamp += 1000;
            let conf_block = BlockBuilder::new()
                .with_previous_hash(prev_block_hash)
                .with_version(4)
                .with_coinbase(&self.coinbase_destination, 50, 3)
                .with_timestamp(timestamp)
                .mine(U256::from(2).pow(254.into()))
                .unwrap();

            let raw_conf_block_header = RawBlockHeader::from_bytes(&conf_block.header.try_format().unwrap())
                .expect("could not serialize block header");
            self.relay(height, &conf_block, raw_conf_block_header);

            prev_block_hash = conf_block.header.hash;
        }

        (tx_id, tx_block_height, bytes_proof, raw_tx, transaction)
    }

    fn relay(&self, height: u32, block: &Block, raw_block_header: RawBlockHeader) {
        if let Some(relayer) = self.relayer {
            assert_ok!(Call::Relay(RelayCall::store_block_header {
                raw_block_header: raw_block_header
            })
            .dispatch(origin_of(account_of(relayer))));
            assert_store_main_chain_header_event(height, raw_block_header.hash(), account_of(relayer));
        } else {
            // bypass staked relayer module
            let block_header = BTCRelayPallet::parse_raw_block_header(&raw_block_header).unwrap();
            assert_ok!(BTCRelayPallet::store_block_header(&account_of(ALICE), block_header));
            assert_store_main_chain_header_event(height, block.header.hash, account_of(ALICE));
        }
    }
}

pub fn register_addresses_and_mine_transaction(
    vault_id: DefaultVaultId<Runtime>,
    signer: BtcPublicKey,
    inputs: Vec<(Transaction, u32, Option<BtcPublicKey>)>,
    outputs: Vec<(BtcAddress, Amount<Runtime>)>,
    return_data: Vec<H256>,
) -> (H256Le, u32, Vec<u8>, Vec<u8>, Transaction) {
    register_vault_address(vault_id.clone(), signer.clone());
    for (_, _, public_key) in inputs.iter() {
        if let Some(key) = public_key {
            register_vault_address(vault_id.clone(), key.clone());
        }
    }
    generate_transaction_and_mine(signer, inputs, outputs, return_data)
}

pub fn register_vault_address(vault_id: DefaultVaultId<Runtime>, from: BtcPublicKey) {
    assert_ok!(VaultRegistryPallet::insert_vault_deposit_address(
        vault_id,
        BtcAddress::P2PKH(from.to_hash())
    ));
}

pub fn generate_transaction_and_mine(
    signer: BtcPublicKey,
    inputs: Vec<(Transaction, u32, Option<BtcPublicKey>)>,
    outputs: Vec<(BtcAddress, Amount<Runtime>)>,
    return_data: Vec<H256>,
) -> (H256Le, u32, Vec<u8>, Vec<u8>, Transaction) {
    let (tx_id, height, proof, raw_tx, tx) = TransactionGenerator::new()
        .with_script(signer.to_p2pkh_script_sig(vec![1; 32]).as_bytes())
        .with_inputs(inputs)
        .with_outputs(outputs)
        .with_op_return(return_data)
        .mine();
    (tx_id, height, proof, raw_tx, tx)
}

pub struct ExtBuilder {
    test_externalities: sp_io::TestExternalities,
}

impl ExtBuilder {
    pub fn build() -> Self {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        let balances = vec![
            (account_of(ALICE), INITIAL_BALANCE),
            (account_of(BOB), INITIAL_BALANCE),
            (account_of(CAROL), INITIAL_BALANCE),
            (account_of(DAVE), INITIAL_BALANCE),
            (account_of(EVE), INITIAL_BALANCE),
            (account_of(FRANK), INITIAL_BALANCE),
            (account_of(GRACE), INITIAL_BALANCE),
            (account_of(FAUCET), 1 << 60),
        ];

        let balances = balances
            .into_iter()
            .flat_map(|(account, balance)| {
                iter_collateral_currencies()
                    .chain(iter_native_currencies())
                    .map(move |currency| (account.clone(), currency, balance))
            })
            .chain(iter_wrapped_currencies().map(move |currency| (account_of(FAUCET), currency, 1 << 60)))
            .collect();

        orml_tokens::GenesisConfig::<Runtime> { balances }
            .assimilate_storage(&mut storage)
            .unwrap();

        oracle::GenesisConfig::<Runtime> {
            authorized_oracles: vec![(account_of(BOB), BOB.to_vec())],
            max_delay: 3600000, // one hour
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_sudo::GenesisConfig::<Runtime> {
            key: Some(account_of(ALICE)),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig::<Runtime> {
            bitcoin_confirmations: CONFIRMATIONS,
            parachain_confirmations: CONFIRMATIONS,
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Runtime> {
            minimum_collateral_vault: vec![(Token(DOT), 0), (Token(KSM), 0)],
            punishment_delay: 8,
            system_collateral_ceiling: iter_currency_pairs().map(|pair| (pair, FUND_LIMIT_CEILING)).collect(),
            secure_collateral_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(150, 100).unwrap()))
                .collect(),
            premium_redeem_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(150, 100).unwrap()))
                .collect(),
            liquidation_collateral_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(110, 100).unwrap()))
                .collect(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        issue::GenesisConfig::<Runtime> {
            issue_period: 10,
            issue_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        redeem::GenesisConfig::<Runtime> {
            redeem_transaction_size: 400,
            redeem_period: 10,
            redeem_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        replace::GenesisConfig::<Runtime> {
            replace_period: 10,
            replace_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        fee::GenesisConfig::<Runtime> {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),    // 5%
            theft_fee_max: 10000000,                                         // 0.1 BTC
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        refund::GenesisConfig::<Runtime> {
            refund_btc_dust_value: 3,
            refund_transaction_size: 401,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_collective::GenesisConfig::<Runtime, TechnicalCommitteeInstance> {
            members: vec![account_of(ALICE)],
            phantom: Default::default(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        supply::GenesisConfig::<Runtime> {
            initial_supply: token_distribution::INITIAL_ALLOCATION,
            start_height: YEARS * 5,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        Self {
            test_externalities: sp_io::TestExternalities::from(storage),
        }
    }

    /// do setup common to all integration tests, then execute the callback
    pub fn execute_with<R>(self, execute: impl FnOnce() -> R) -> R {
        self.execute_without_relay_init(|| {
            // initialize btc relay
            let _ = TransactionGenerator::new().with_confirmations(7).mine();

            assert_ok!(Call::Oracle(OracleCall::insert_authorized_oracle {
                account_id: account_of(ALICE),
                name: vec![]
            })
            .dispatch(root()));
            assert_ok!(Call::Oracle(OracleCall::feed_values {
                values: vec![
                    (OracleKey::ExchangeRate(DEFAULT_COLLATERAL_CURRENCY), FixedU128::from(1)),
                    (OracleKey::ExchangeRate(DEFAULT_GRIEFING_CURRENCY), FixedU128::from(1)),
                    (OracleKey::FeeEstimation, FixedU128::from(3)),
                ]
            })
            .dispatch(origin_of(account_of(ALICE))));
            OraclePallet::begin_block(0);

            let ret = execute();
            VaultRegistryPallet::total_user_vault_collateral_integrity_check();
            ret
        })
    }

    /// used for btc-relay test
    pub fn execute_without_relay_init<R>(mut self, execute: impl FnOnce() -> R) -> R {
        self.test_externalities.execute_with(|| {
            SystemPallet::set_block_number(1); // required to be able to dispatch functions
            SecurityPallet::set_active_block_number(1);

            assert_ok!(OraclePallet::_set_exchange_rate(
                DEFAULT_COLLATERAL_CURRENCY,
                FixedU128::one()
            ));
            set_default_thresholds();

            let ret = execute();
            VaultRegistryPallet::total_user_vault_collateral_integrity_check();
            ret
        })
    }
}

pub const fn wrapped(amount: Balance) -> Amount<Runtime> {
    Amount::new(amount, DEFAULT_WRAPPED_CURRENCY)
}

pub const fn griefing(amount: Balance) -> Amount<Runtime> {
    Amount::new(amount, DEFAULT_GRIEFING_CURRENCY)
}
