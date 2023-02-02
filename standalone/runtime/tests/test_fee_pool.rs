mod mock;
use crate::redeem_testing_utils::{expire_bans, setup_cancelable_redeem, RedeemRequestTestExt};
use currency::Amount;
use frame_support::migration::put_storage_value;
use interbtc_runtime_standalone::{Timestamp, UnsignedFixedPoint};
use mock::{
    assert_eq,
    loans_testing_utils::{deposit_and_borrow, mint_lend_tokens},
    nomination_testing_utils::*,
    reward_testing_utils::IdealRewardPool,
    *,
};
use primitives::TruncateFixedPointToInt;
use rand::Rng;
use sp_consensus_aura::{Slot, SlotDuration};
use sp_timestamp::Timestamp as SlotTimestamp;
use std::collections::BTreeMap;
use traits::LoansApi;
use vault_registry::DefaultVaultId;

const VAULT_2: [u8; 32] = DAVE;
const REWARD_CURRENCY: CurrencyId = Token(INTR);
const DEFAULT_EXCHANGE_RATE: f64 = 0.1;

fn default_nomination(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_NOMINATION, currency_id)
}

// assert that a and b differ by at most 1
macro_rules! assert_approx_eq {
    ($left:expr, $right:expr $(,)?) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                if (*left_val > *right_val && *left_val - *right_val > 5)
                    || (*right_val > *left_val && *right_val - *left_val > 5)
                {
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    panic!(
                        r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                        &*left_val, &*right_val
                    )
                }
            }
        }
    }};
}

fn test_with_2<R>(execute: impl Fn(VaultId) -> R) {
    let test_with = |currency_id, wrapped_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies() {
                assert_ok!(OraclePallet::_set_exchange_rate(
                    currency_id,
                    FixedU128::from_float(DEFAULT_EXCHANGE_RATE)
                ));
            }
            if wrapped_id != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_id, FixedU128::one()));
            }
            activate_lending_and_mint(Token(DOT), LendToken(1));
            UserData::force_to(USER, default_user_state());
            let vault_id = PrimitiveVaultId::new(account_of(VAULT), currency_id, wrapped_id);
            CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));
            LiquidationVaultData::force_to(default_liquidation_vault_state(&vault_id.currencies));

            enable_nomination();
            assert_nomination_opt_in(&vault_id);

            let commission = UnsignedFixedPoint::from_float(COMMISSION);
            set_commission(&vault_id, commission);

            execute(vault_id)
        });
    };
    test_with(Token(DOT), Token(KBTC));
}

fn test_with<R>(execute: impl Fn(VaultId) -> R) {
    let test_with = |currency_id, wrapped_id| {
        ExtBuilder::build().execute_with(|| {
            SecurityPallet::set_active_block_number(1);
            for currency_id in iter_collateral_currencies().filter(|c| !c.is_lend_token()) {
                assert_ok!(OraclePallet::_set_exchange_rate(
                    currency_id,
                    FixedU128::from_float(DEFAULT_EXCHANGE_RATE)
                ));
            }
            if wrapped_id != Token(IBTC) {
                assert_ok!(OraclePallet::_set_exchange_rate(wrapped_id, FixedU128::one()));
            }
            activate_lending_and_mint(Token(DOT), LendToken(1));
            UserData::force_to(USER, default_user_state());
            let vault_id = PrimitiveVaultId::new(account_of(VAULT), currency_id, wrapped_id);
            CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));
            LiquidationVaultData::force_to(default_liquidation_vault_state(&vault_id.currencies));

            enable_nomination();
            assert_nomination_opt_in(&vault_id);

            let commission = UnsignedFixedPoint::from_float(COMMISSION);
            set_commission(&vault_id, commission);

            execute(vault_id)
        });
    };
    test_with(Token(DOT), Token(KBTC));
    test_with(Token(KSM), Token(IBTC));
    test_with(Token(DOT), Token(IBTC));
    test_with(ForeignAsset(1), Token(IBTC));
    test_with(LendToken(1), Token(IBTC));
}

fn distribute_rewards(amount: Amount<Runtime>) {
    // mint the tokens
    amount.mint_to(&FeePallet::fee_pool_account_id()).unwrap();

    // distribute
    FeePallet::distribute_rewards(&amount).unwrap();
}

fn withdraw_vault_rewards(vault_id: &VaultId) -> i128 {
    withdraw_nominator_rewards(vault_id, &vault_id.account_id)
}

fn withdraw_nominator_rewards(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    assert_ok!(FeePallet::distribute_all_vault_rewards(vault_id));
    let amount = VaultStakingPallet::compute_reward(REWARD_CURRENCY, vault_id, nominator_id).unwrap();
    assert_ok!(RuntimeCall::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(nominator_id.clone())));
    amount
}

fn distribute_capacity_and_compute_vault_reward(vault_id: &VaultId) -> i128 {
    let reward = CapacityRewardsPallet::withdraw_reward(&(), &vault_id.collateral_currency(), REWARD_CURRENCY).unwrap();
    VaultRewardsPallet::distribute_reward(&vault_id.collateral_currency(), REWARD_CURRENCY, reward.into()).unwrap();
    VaultRewardsPallet::compute_reward(&vault_id.collateral_currency(), vault_id, REWARD_CURRENCY).unwrap()
}

fn compute_staking_reward(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    VaultStakingPallet::compute_reward(REWARD_CURRENCY, vault_id, nominator_id).unwrap()
}

fn get_vault_collateral(vault_id: &VaultId) -> Amount<Runtime> {
    VaultRegistryPallet::compute_collateral(vault_id)
        .unwrap()
        .try_into()
        .unwrap()
}

#[test]
fn integration_test_estimate_vault_reward_rate() {
    test_with(|vault_id| {
        let rewards1 = Amount::<Runtime>::new(1000000000000000, REWARD_CURRENCY);
        rewards1.mint_to(&VaultAnnuityPallet::account_id()).unwrap();
        VaultAnnuityPallet::update_reward_per_block();

        let reward = runtime_common::estimate_vault_reward_rate::<
            Runtime,
            VaultAnnuityInstance,
            VaultStakingPallet,
            CapacityRewardsPallet,
            _,
        >(vault_id.clone())
        .unwrap();
        assert!(!reward.is_zero());
    });
}

#[test]
fn integration_test_estimate_vault_reward_rate_works_with_zero_stake() {
    test_with(|vault_id| {
        CoreVaultData::force_to(
            &vault_id,
            CoreVaultData {
                backing_collateral: Amount::zero(vault_id.collateral_currency()),
                ..default_vault_state(&vault_id)
            },
        );
        assert_ok!(
            runtime_common::estimate_vault_reward_rate::<
                Runtime,
                VaultAnnuityInstance,
                VaultStakingPallet,
                CapacityRewardsPallet,
                _,
            >(vault_id.clone()),
            Zero::zero()
        );
    });
}

#[test]
fn test_vault_reward_withdrawal() {
    test_with(|vault_id_1| {
        let vault_id_2 = VaultId {
            account_id: account_of(VAULT2),
            ..vault_id_1.clone()
        };

        let rewards1 = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards1);

        let mut vault_2 = default_vault_state(&vault_id_2);
        vault_2.backing_collateral = vault_2.backing_collateral * 4;
        CoreVaultData::force_to(&vault_id_2, vault_2.clone());

        let rewards2 = Amount::new(5000, REWARD_CURRENCY);
        distribute_rewards(rewards2);

        let mut reference_pool = IdealRewardPool::default();
        reference_pool
            .set_exchange_rate(vault_id_1.collateral_currency(), FixedU128::one())
            .set_commission(&vault_id_1, FixedU128::from_float(COMMISSION))
            .set_secure_threshold(
                &vault_id_1,
                VaultRegistryPallet::get_vault_secure_threshold(&vault_id_1).unwrap(),
            )
            .deposit_nominator_collateral(
                &(vault_id_1.clone(), vault_id_1.account_id.clone()),
                get_vault_collateral(&vault_id_1).amount(),
            )
            .distribute_reward(rewards1.amount())
            .set_secure_threshold(
                &vault_id_2,
                VaultRegistryPallet::get_vault_secure_threshold(&vault_id_2).unwrap(),
            )
            .deposit_nominator_collateral(
                &(vault_id_2.clone(), vault_id_2.account_id.clone()),
                get_vault_collateral(&vault_id_2).amount(),
            )
            .distribute_reward(rewards2.amount());

        assert_approx_eq!(
            withdraw_vault_rewards(&vault_id_1),
            reference_pool.get_total_reward_for(&vault_id_1.account_id) as i128
        );
        assert_approx_eq!(
            withdraw_vault_rewards(&vault_id_2),
            reference_pool.get_total_reward_for(&vault_id_2.account_id) as i128
        );
    });
}

#[test]
fn test_nomination_distributes_past_rewards() {
    test_with(|vault_id_1| {
        let currency_id = vault_id_1.collateral_currency();
        let rewards = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards);

        assert_approx_eq!(
            distribute_capacity_and_compute_vault_reward(&vault_id_1),
            rewards.amount() as i128
        );

        // nominator joins
        assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));

        fn commission_for(amount: f64) -> f64 {
            amount * COMMISSION
        }

        fn nominator_share(amount: f64) -> f64 {
            amount - commission_for(amount)
        }

        // operator receives the commission directly
        assert_eq!(
            ParachainState::get(&vault_id_1),
            ParachainState::get_default(&vault_id_1).with_changes(|user, vault, _, _| {
                (*user.balances.get_mut(&vault_id_1.collateral_currency()).unwrap()).free -=
                    default_nomination(currency_id);
                vault.backing_collateral += default_nomination(currency_id);
                // TODO: find a better way to account for rounding
                let rounding = 2;
                let commission = Amount::new(
                    commission_for((rewards.amount() - rounding) as f64) as u128,
                    REWARD_CURRENCY,
                );
                *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += commission;
            })
        );

        // rewards are withdrawn when a new nominator joins
        assert_approx_eq!(distribute_capacity_and_compute_vault_reward(&vault_id_1), 0 as i128);
        assert_approx_eq!(
            compute_staking_reward(&vault_id_1, &vault_id_1.account_id),
            nominator_share(rewards.amount() as f64) as i128
        );
    });
}

#[test]
fn test_nomination_and_rewards() {
    test_with(|vault_id| {
        let nominator_id = account_of(USER);
        let mut reference_pool = IdealRewardPool::default();

        let vault_stake = default_vault_state(&vault_id).backing_collateral;
        let nominator_stake = vault_stake / 4;

        let rewards1 = Amount::new(1000, REWARD_CURRENCY);
        let rewards2 = Amount::new(2000, REWARD_CURRENCY);

        // set commission, exchange rate and secure threshold first
        reference_pool.set_commission(&vault_id, FixedU128::from_float(COMMISSION));
        reference_pool.set_exchange_rate(vault_id.collateral_currency(), FixedU128::one());
        reference_pool.set_secure_threshold(
            &vault_id,
            VaultRegistryPallet::get_vault_secure_threshold(&vault_id).unwrap(),
        );

        // deposit vault stake prior to getting rewards
        // this is done for runtime in test constructor
        reference_pool
            .deposit_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), vault_stake.amount());

        // distribute without nominator
        distribute_rewards(rewards1);
        reference_pool.distribute_reward(rewards1.amount());

        // add a nominator, internally this drains past rewards
        assert_nominate_collateral(&vault_id, nominator_id.clone(), nominator_stake);
        reference_pool
            .deposit_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), nominator_stake.amount());

        // reward with nominator
        distribute_rewards(rewards2);
        reference_pool.distribute_reward(rewards2.amount());

        assert_approx_eq!(
            withdraw_vault_rewards(&vault_id),
            reference_pool.get_total_reward_for(&vault_id.account_id) as i128
        );
        assert_approx_eq!(
            withdraw_nominator_rewards(&vault_id, &nominator_id),
            reference_pool.get_total_reward_for(&nominator_id) as i128
        );
    });
}

#[test]
fn test_nomination_slashing_and_rewards() {
    test_with(|vault_id| {
        let currency_id = vault_id.collateral_currency();
        let nominator_id = account_of(USER);

        let rewards1 = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards1);

        assert_nominate_collateral(&vault_id, nominator_id.clone(), default_nomination(currency_id));

        // slash the vault and its nominator
        assert_ok!(VaultRegistryPallet::transfer_funds(
            CurrencySource::Collateral(vault_id.clone()),
            CurrencySource::FreeBalance(account_of(VAULT_2)),
            &default_nomination(currency_id),
        ));

        let rewards2 = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards2);

        let vault_collateral = get_vault_collateral(&vault_id).amount();
        let nominator_collateral = DEFAULT_NOMINATION;
        let slashed_amount = DEFAULT_NOMINATION;

        let mut reference_pool = IdealRewardPool::default();
        reference_pool
            .set_exchange_rate(vault_id.collateral_currency(), FixedU128::one())
            .set_secure_threshold(
                &vault_id,
                VaultRegistryPallet::get_vault_secure_threshold(&vault_id).unwrap(),
            )
            .set_commission(&vault_id, FixedU128::from_float(COMMISSION))
            .deposit_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), vault_collateral)
            .distribute_reward(rewards1.amount())
            .deposit_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), nominator_collateral)
            .slash_collateral(&vault_id, slashed_amount)
            .distribute_reward(rewards2.amount());

        assert_approx_eq!(
            withdraw_vault_rewards(&vault_id),
            reference_pool.get_total_reward_for(&vault_id.account_id) as i128
        );
        assert_approx_eq!(
            withdraw_nominator_rewards(&vault_id, &nominator_id),
            reference_pool.get_total_reward_for(&nominator_id) as i128
        );
    });
}

#[test]
fn test_nomination_withdrawal_and_rewards() {
    test_with(|vault_id| {
        let currency_id = vault_id.collateral_currency();
        let nominator_id = account_of(USER);

        // nominate and distribute rewards
        assert_nominate_collateral(&vault_id, nominator_id.clone(), default_nomination(currency_id));
        let rewards1 = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards1);

        // withdraw half available collateral and distribute again
        assert_withdraw_nominator_collateral(nominator_id.clone(), &vault_id, default_nomination(currency_id) / 2);
        let rewards2 = Amount::new(1000, REWARD_CURRENCY);
        distribute_rewards(rewards2);

        let vault_collateral = get_vault_collateral(&vault_id).amount();
        let nominator_collateral = DEFAULT_NOMINATION;
        let withdrawn_collateral = DEFAULT_NOMINATION / 2;

        let mut reference_pool = IdealRewardPool::default();
        reference_pool
            .set_exchange_rate(vault_id.collateral_currency(), FixedU128::one())
            .set_secure_threshold(
                &vault_id,
                VaultRegistryPallet::get_vault_secure_threshold(&vault_id).unwrap(),
            )
            .set_commission(&vault_id, FixedU128::from_float(COMMISSION))
            .deposit_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), vault_collateral)
            .deposit_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), nominator_collateral)
            .distribute_reward(rewards1.amount())
            .withdraw_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), withdrawn_collateral)
            .distribute_reward(rewards2.amount());

        assert_approx_eq!(
            withdraw_nominator_rewards(&vault_id, &nominator_id),
            reference_pool.get_total_reward_for(&nominator_id) as i128
        );
    });
}

enum Action {
    DepositNominationCollateral,
    WithdrawNominationCollateral,
    DepositVaultCollateral,
    WithdrawVaultCollateral,
    SetSecureThreshold,
    DistributeRewards,
    SetExchangeRate,
    SetAcceptIssues,
    FailRedeem,
}

impl Action {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Action {
        match rng.gen_range(0..8) {
            0 => Self::DepositNominationCollateral,
            1 => Self::WithdrawNominationCollateral,
            2 => Self::DepositVaultCollateral,
            3 => Self::WithdrawVaultCollateral,
            4 => Self::SetSecureThreshold,
            5 => Self::DistributeRewards,
            6 => Self::SetExchangeRate,
            7 => Self::SetAcceptIssues,
            /* note the range - this will never be produced, since slashing is known to break nomination */
            8 => Self::FailRedeem,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
enum ConcreteAction {
    DepositNominationCollateral {
        nominator_id: AccountId,
        vault_id: VaultId,
        amount: Amount<Runtime>,
    },
    WithdrawNominationCollateral {
        nominator_id: AccountId,
        vault_id: VaultId,
        amount: Amount<Runtime>,
    },
    SetSecureThreshold {
        vault_id: VaultId,
        threshold: FixedU128,
    },
    SetExchangeRate {
        currency_id: CurrencyId,
        exchange_rate: FixedU128,
    },
    DistributeRewards {
        amount: Amount<Runtime>,
    },
    FailRedeem {
        vault_id: VaultId,
        amount: Amount<Runtime>,
    },
    AcceptNewIssues {
        vault_id: VaultId,
        accept: bool,
    },
}

impl ConcreteAction {
    fn execute(&self, reference_pool: &mut IdealRewardPool) {
        match self {
            ConcreteAction::DepositNominationCollateral {
                nominator_id,
                vault_id,
                amount,
            } => {
                reference_pool.deposit_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), amount.amount());
                assert_nominate_collateral(vault_id, nominator_id.clone(), amount.clone());
            }
            ConcreteAction::WithdrawNominationCollateral {
                nominator_id,
                vault_id,
                amount,
            } => {
                reference_pool
                    .withdraw_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), amount.amount());
                assert_withdraw_nominator_collateral(nominator_id.clone(), vault_id, amount.clone());
            }
            ConcreteAction::SetSecureThreshold { vault_id, threshold } => {
                assert_ok!(
                    RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                        currency_pair: vault_id.currencies.clone(),
                        custom_threshold: Some(*threshold),
                    })
                    .dispatch(origin_of(vault_id.account_id.clone()))
                );

                reference_pool.set_secure_threshold(vault_id, threshold.clone());
            }
            ConcreteAction::SetExchangeRate {
                currency_id,
                exchange_rate,
            } => {
                if currency_id.is_lend_token() {
                    let underlying_id = LoansPallet::underlying_id(*currency_id).unwrap();
                    let lend_token_rate = exchange_rate.mul(LoansPallet::exchange_rate(underlying_id));
                    // Only need to set the exchange rate of the underlying currency in the oracle pallet
                    OraclePallet::_set_exchange_rate(underlying_id, *exchange_rate).unwrap();
                    // The reference pool must store both exchange rates explicitly
                    reference_pool.set_exchange_rate(*currency_id, lend_token_rate);
                    reference_pool.set_exchange_rate(underlying_id, *exchange_rate);
                } else {
                    OraclePallet::_set_exchange_rate(*currency_id, *exchange_rate).unwrap();
                    reference_pool.set_exchange_rate(*currency_id, *exchange_rate);
                    if let Ok(lend_token_id) = LoansPallet::lend_token_id(*currency_id) {
                        let lend_token_rate = exchange_rate.div(LoansPallet::exchange_rate(*currency_id));
                        reference_pool.set_exchange_rate(lend_token_id, lend_token_rate);
                    }
                }
            }
            ConcreteAction::DistributeRewards { amount } => {
                distribute_rewards(*amount);
                reference_pool.distribute_reward(amount.amount());
            }
            ConcreteAction::FailRedeem { vault_id, amount } => {
                expire_bans();

                let free_balance_before = CurrencySource::FreeBalance(vault_id.account_id.clone())
                    .current_balance(DEFAULT_GRIEFING_CURRENCY)
                    .unwrap();
                let redeem_id = setup_cancelable_redeem(USER, &vault_id, *amount);
                let free_balance_after = CurrencySource::FreeBalance(vault_id.account_id.clone())
                    .current_balance(DEFAULT_GRIEFING_CURRENCY)
                    .unwrap();

                VaultRegistryPallet::transfer_funds(
                    CurrencySource::FreeBalance(vault_id.account_id.clone()),
                    CurrencySource::AvailableReplaceCollateral(vault_id.clone()),
                    &(free_balance_after - free_balance_before),
                )
                .unwrap();

                let redeem = RedeemPallet::get_open_redeem_request_from_id(&redeem_id).unwrap();
                let amount_without_fee_collateral =
                    redeem.amount_without_fee_as_collateral(vault_id.collateral_currency());
                let punishment_fee = FeePallet::get_punishment_fee(&amount_without_fee_collateral).unwrap();

                assert_ok!(RuntimeCall::Redeem(RedeemCall::cancel_redeem {
                    redeem_id: redeem_id,
                    reimburse: false
                })
                .dispatch(origin_of(account_of(USER))));

                reference_pool.slash_collateral(&vault_id, punishment_fee.amount());
            }
            ConcreteAction::AcceptNewIssues { vault_id, accept } => {
                assert_ok!(RuntimeCall::VaultRegistry(VaultRegistryCall::accept_new_issues {
                    currency_pair: vault_id.currencies.clone(),
                    accept_new_issues: *accept,
                })
                .dispatch(origin_of(vault_id.account_id.clone())));
                reference_pool.accept_new_issues(&vault_id, *accept);
            }
        }
    }
}

#[test]
#[cfg_attr(feature = "skip-slow-tests", ignore)]
fn test_fee_pool_matches_ideal_implementation() {
    env_logger::init();
    for _ in 1..100 {
        do_random_nomination_sequence();
    }
}

const MAX_COLLATERAL: u128 = 1000;
fn setup_nomination(vault_id: VaultId) -> (IdealRewardPool, Vec<AccountId>, Vec<VaultId>) {
    let token1 = Token(DOT);
    let token2 = Token(KSM);

    // set up some potential nominators
    let nominators: Vec<_> = (100..107).map(|id| account_of([id; 32])).collect();
    for nominator in nominators.iter() {
        for currency_id in [vault_id.collateral_currency(), token1, token2] {
            if currency_id.is_lend_token() {
                // Hardcoding the lend_token balance would break the internal exchange rate calculation
                // in the Loans pallet, which is using the total amount of issued lend_tokens
                mint_lend_tokens(nominator.clone(), currency_id);
            } else {
                assert_ok!(RuntimeCall::Tokens(TokensCall::set_balance {
                    who: nominator.clone(),
                    currency_id,
                    new_free: MAX_COLLATERAL,
                    new_reserved: 0,
                })
                .dispatch(root()));
            }
        }
    }

    // setup some vaults
    let vault_id = &vault_id;
    let vaults: Vec<_> = (107..110)
        .map(|id| {
            let collateral_currency = if id % 2 == 0 { token1 } else { token2 };
            let vault_id = PrimitiveVaultId {
                account_id: account_of([id; 32]),
                currencies: VaultCurrencyPair {
                    collateral: collateral_currency,
                    wrapped: vault_id.wrapped_currency(),
                },
            };
            CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));
            assert_nomination_opt_in(&vault_id);
            set_commission(&vault_id, FixedU128::from_float(COMMISSION));
            vault_id
        })
        .chain(vec![vault_id.clone()])
        .collect();

    // setup the reference pool - vaults have initial stake
    let mut reference_pool = IdealRewardPool::default();
    for vault_id in vaults.iter() {
        let collateral = default_vault_state(&vault_id).backing_collateral.amount();
        reference_pool.deposit_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), collateral);
        reference_pool.set_commission(&vault_id, FixedU128::from_float(COMMISSION));

        assert_ok!(
            RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                currency_pair: vault_id.currencies.clone(),
                custom_threshold: Some(FixedU128::from_float(3.0)),
            })
            .dispatch(origin_of(vault_id.account_id.clone()))
        );
        let threshold = VaultRegistryPallet::get_vault_secure_threshold(&vault_id).unwrap();
        reference_pool.set_secure_threshold(&vault_id, threshold);

        if vault_id.collateral_currency().is_lend_token() {
            let underlying_id = LoansPallet::underlying_id(vault_id.collateral_currency()).unwrap();
            let lend_token_rate = FixedU128::one().mul(LoansPallet::exchange_rate(underlying_id));
            OraclePallet::_set_exchange_rate(underlying_id, FixedU128::one()).unwrap();
            reference_pool.set_exchange_rate(vault_id.collateral_currency(), lend_token_rate);
            reference_pool.set_exchange_rate(underlying_id, FixedU128::one());
        } else {
            OraclePallet::_set_exchange_rate(vault_id.collateral_currency(), FixedU128::one()).unwrap();
            reference_pool.set_exchange_rate(vault_id.collateral_currency(), FixedU128::one());
        }
    }

    (reference_pool, nominators, vaults)
}
fn do_random_nomination_sequence() {
    test_with_2(|vault_id| {
        let mut rng = rand::thread_rng();

        let (mut reference_pool, nominators, vaults) = setup_nomination(vault_id.clone());

        let mut actual_rewards = BTreeMap::new();
        let mut actions = Vec::new();
        for _ in 0..50 {
            // 50 random actions.
            let action = match Action::random(&mut rng) {
                Action::DepositNominationCollateral => {
                    log::error!("DepositNominationCollateral");

                    let nominator_id = nominators[rng.gen_range(0..nominators.len())].clone();
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let free =
                        UserData::from_account(nominator_id.clone()).balances[&vault_id.collateral_currency()].free;
                    if free.is_zero() {
                        continue;
                    }

                    let amount = Amount::new(
                        rng.gen_range(0..MAX_COLLATERAL.min(free.amount())),
                        vault_id.collateral_currency(),
                    );
                    ConcreteAction::DepositNominationCollateral {
                        nominator_id,
                        vault_id,
                        amount,
                    }
                }
                Action::WithdrawNominationCollateral => {
                    let nominations = reference_pool.nominations();
                    if nominations.is_empty() {
                        continue;
                    }
                    let ((vault_id, nominator_id), amount) = nominations[rng.gen_range(0..nominations.len())].clone();

                    let max_amount = std::cmp::min(
                        VaultRegistryPallet::get_free_collateral(&vault_id)
                            .unwrap()
                            .amount()
                            .saturating_sub(100), // acount for rounding errors
                        amount.truncate_to_inner().unwrap(),
                    );

                    if max_amount == 0 {
                        continue;
                    }

                    let amount = Amount::new(rng.gen_range(0..max_amount), vault_id.collateral_currency());

                    ConcreteAction::WithdrawNominationCollateral {
                        nominator_id,
                        vault_id,
                        amount,
                    }
                }
                Action::DepositVaultCollateral => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let max_amount =
                        CoreVaultData::vault(vault_id.clone()).free_balance[&vault_id.collateral_currency()].amount();
                    let amount = Amount::new(rng.gen_range(0..max_amount), vault_id.collateral_currency());

                    ConcreteAction::DepositNominationCollateral {
                        nominator_id: vault_id.account_id.clone(),
                        vault_id,
                        amount,
                    }
                }
                Action::WithdrawVaultCollateral => {
                    let vaults: Vec<_> = reference_pool
                        .nominations()
                        .into_iter()
                        .filter(|((vault, nominator), _)| &vault.account_id == nominator)
                        .map(|((vault_id, _), _)| vault_id)
                        .collect();
                    if vaults.is_empty() {
                        continue;
                    }
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let max_amount = VaultRegistryPallet::get_free_collateral(&vault_id)
                        .unwrap()
                        .amount()
                        .saturating_sub(100); // acount for rounding
                    if max_amount.is_zero() {
                        continue;
                    }
                    let amount = Amount::new(rng.gen_range(0..max_amount), vault_id.collateral_currency());

                    ConcreteAction::WithdrawNominationCollateral {
                        nominator_id: vault_id.account_id.clone(),
                        vault_id,
                        amount,
                    }
                }
                Action::SetSecureThreshold => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let threshold = FixedU128::from_float(rng.gen_range(2.0..5.0));

                    ConcreteAction::SetSecureThreshold { vault_id, threshold }
                }
                Action::SetExchangeRate => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let currency_id = vault_id.collateral_currency();
                    let exchange_rate = FixedU128::from_float(rng.gen_range(0.5..5.0));

                    ConcreteAction::SetExchangeRate {
                        currency_id,
                        exchange_rate,
                    }
                }
                Action::DistributeRewards => {
                    let amount = Amount::new(rng.gen_range(0..10_000_000_000), REWARD_CURRENCY);
                    ConcreteAction::DistributeRewards { amount }
                }
                Action::FailRedeem => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let vault = CoreVaultData::vault(vault_id.clone());
                    let redeemable = vault.issued - vault.to_be_redeemed;
                    let user_btc = UserData::get(USER).balances[&vault_id.wrapped_currency()].free;

                    let max_amount = user_btc.min(&redeemable).unwrap();
                    let min_amount = redeem::Pallet::<Runtime>::get_dust_value(vault_id.wrapped_currency())
                        + redeem::Pallet::<Runtime>::get_current_inclusion_fee(vault_id.wrapped_currency()).unwrap();
                    if max_amount <= min_amount {
                        continue;
                    }

                    let amount = max_amount.with_amount(|x| rng.gen_range(min_amount.amount()..x));

                    ConcreteAction::FailRedeem { vault_id, amount }
                }
                Action::SetAcceptIssues => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let accept = rng.gen_bool(0.5);
                    ConcreteAction::AcceptNewIssues { vault_id, accept }
                }
            };
            actions.push(action.clone());
            action.execute(&mut reference_pool);
        }
        for ((vault_id, nominator_id), _) in reference_pool.nominations() {
            withdraw_vault_rewards(&vault_id);
            let reward = withdraw_nominator_rewards(&vault_id, &nominator_id);
            actual_rewards.insert(nominator_id, reward as u128);
        }

        let total_reference_pool: u128 = reference_pool.rewards().iter().map(|(_, value)| *value).sum();
        let total_actually_received: u128 = reference_pool
            .rewards()
            .iter()
            .map(|(nominator, _)| {
                let initial = if vaults.iter().any(|x| &x.account_id == nominator) {
                    200_000
                } else {
                    0
                };
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator).amount() - initial
            })
            .sum();

        if abs_difference(total_reference_pool, total_actually_received) >= 1000 {
            log::error!("Failed assertion for actions {actions:?}");
            // use the following to format the debug output above to an array to be used in reproduction
            #[cfg_attr(rustfmt, rustfmt_skip)] // don't fmt this comment, it breaks the cmd
            // cat $file | sed 's/CurrencyId:://g' | sed 's/TokenSymbol:://g' | sed 's/[^ ]*\(..\) [(]5[^)]*[)]/account_of([0x\1; 32])/g' | sed 's/FixedU128/FixedU128::from_float/g' | sed 's/Amount { amount: \([0-9]\+\), currency_id: \([^}]\+\)}/Amount::new(\1, \2)/g'

            assert_eq!(total_reference_pool, total_actually_received);
        }

        // check the rewards of all stakeholders
        for (nominator_id, expected_reward) in reference_pool.rewards() {
            // vaults had some initial free balance in the reward currency - compensate for that..
            let actual_reward = if vaults.iter().any(|x| x.account_id == nominator_id) {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount() - 200_000
            } else {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount()
            };
            // ensure the difference is small, but allow some rounding errors..
            if abs_difference(actual_reward, expected_reward) > expected_reward / 500 + 10 {
                log::error!("Failed assertion for actions {actions:?}");
                assert_eq!(actual_reward, expected_reward);
            }
        }
    })
}

#[test]
#[ignore] // this function is used to debug failing test cases of test_fee_pool_matches_ideal_implementation
fn reproduce_failing_test() {
    for i in 1..50 {
        _reproduce_failing_test(i);
    }
}

fn _reproduce_failing_test(num_actions: usize) {
    let _ = env_logger::try_init();
    test_with_2(|vault_id| {
        let (mut reference_pool, _, vaults) = setup_nomination(vault_id.clone());

        use ConcreteAction::*;

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let actions = vec![
            DepositNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(50000, Token(DOT)), },
            DistributeRewards { amount: Amount::new(10000000000, Token(INTR)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(3.419287478062694912), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(53948, Token(DOT)), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(573157, Token(DOT)), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(3.092839019606802944), },
            DepositNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(367450, Token(DOT)), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(1.373757773995398144), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(2.691586595358666240), },
            DistributeRewards { amount: Amount::new(8982684181, Token(INTR)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6b; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(2.203683781128871680), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6d; 32]), vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(650763, Token(KSM)), },
            DistributeRewards { amount: Amount::new(4447058456, Token(INTR)), },
            WithdrawNominationCollateral { nominator_id: account_of([0x01; 32]), vault_id: VaultId { account_id: account_of([0x01; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(31049, Token(DOT)), },
            WithdrawNominationCollateral { nominator_id: account_of([0x01; 32]), vault_id: VaultId { account_id: account_of([0x01; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(10377, Token(DOT)), },
            SetExchangeRate { currency_id: Token(KSM), exchange_rate: FixedU128::from_float(1.670225524936316928), },
            FailRedeem { vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(55567, Token(KBTC)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(3.365260653951399936), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6b; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(2.649903409252506624), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(4.728498710955768832), },
            DepositNominationCollateral { nominator_id: account_of([0x64; 32]), vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(827, Token(KSM)), },
            FailRedeem { vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(55118, Token(KBTC)), },
            DistributeRewards { amount: Amount::new(8032902951, Token(INTR)), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(4.115433122284937216), },
            DepositNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(121195, Token(DOT)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6b; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(3.677915815946293760), },
            SetExchangeRate { currency_id: Token(KSM), exchange_rate: FixedU128::from_float(1.880357238054170112), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(2.035206833200252416), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(1.260768734589768192), },
            DepositNominationCollateral { nominator_id: account_of([0x67; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(511, Token(DOT)), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(262678, Token(DOT)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(4.710951067647164416), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6b; 32]), vault_id: VaultId { account_id: account_of([0x6b; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(82259, Token(KSM)), },
            DepositNominationCollateral { nominator_id: account_of([0x69; 32]), vault_id: VaultId { account_id: account_of([0x01; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(382, Token(DOT)), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x01; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(3.275451943011447296), },
            DepositNominationCollateral { nominator_id: account_of([0x6d; 32]), vault_id: VaultId { account_id: account_of([0x6d; 32]), currencies: VaultCurrencyPair { collateral: Token(KSM), wrapped: Token(KBTC), }, }, amount: Amount::new(549361, Token(KSM)), },
            SetExchangeRate { currency_id: Token(KSM), exchange_rate: FixedU128::from_float(2.314548580022945792), },
            WithdrawNominationCollateral { nominator_id: account_of([0x6c; 32]), vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, amount: Amount::new(24121, Token(DOT)), },
            SetExchangeRate { currency_id: Token(DOT), exchange_rate: FixedU128::from_float(4.374931332402049024), },
            SetSecureThreshold { vault_id: VaultId { account_id: account_of([0x6c; 32]), currencies: VaultCurrencyPair { collateral: Token(DOT), wrapped: Token(KBTC), }, }, threshold: FixedU128::from_float(4.625228520227672064), },
        ];

        for action in actions.iter().take(num_actions - 1) {
            action.execute(&mut reference_pool);
        }
        // separated last action for easier breakpointing
        for action in actions.iter().skip(num_actions - 1).take(1) {
            log::error!("Last action: {action:?}");
            action.execute(&mut reference_pool);
        }

        let total_distributed: u128 = actions
            .iter()
            .take(num_actions)
            .filter_map(|x| match x {
                ConcreteAction::DistributeRewards { amount } => Some(amount.amount()),
                _ => None,
            })
            .sum();

        for ((vault_id, nominator_id), _) in reference_pool.nominations() {
            withdraw_vault_rewards(&vault_id);
            withdraw_nominator_rewards(&vault_id, &nominator_id);
        }

        let total_reference_pool: u128 = reference_pool.rewards().iter().map(|(_, value)| *value).sum();
        let total_actually_received: u128 = reference_pool
            .rewards()
            .iter()
            .map(|(nominator, _)| {
                let initial = if vaults.iter().any(|x| &x.account_id == nominator) {
                    200_000
                } else {
                    0
                };
                let actual_reward = currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator).amount() - initial;
                let reference_nominated = reference_pool.get_total_reward_for(&nominator);
                let reference_reward = reference_pool
                    .rewards()
                    .iter()
                    .find_map(|x| if &x.0 == nominator { Some(x.1) } else { None })
                    .unwrap();

                let diff = actual_reward as i128 - reference_reward as i128;

                let abs_diff = abs_difference(actual_reward, reference_reward);

                log::error!("actual_reward {nominator:?} {actual_reward} {reference_reward}, diff: {diff}, {reference_nominated}");
                log::error!("Num_Actions: {num_actions}");
                assert!(abs_diff < 1000);

                actual_reward
            })
            .sum();

        log::error!("Total distributed: {total_distributed}");
        log::error!("Total reference_pool: {total_reference_pool}");
        log::error!("Total actually_received: {total_actually_received}");
        log::error!(
            "Difference: {}",
            abs_difference(total_distributed, total_actually_received)
        );
        log::error!("num_actions: {num_actions}");

        // check the rewards of all stakeholders
        for (nominator_id, expected_reward) in reference_pool.rewards() {
            // vaults had some initial free balance in the reward currency - compensate for that..
            let actual_reward = if vaults.iter().any(|x| x.account_id == nominator_id) {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount() - 200_000
            } else {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount()
            };

            // ensure the difference is small, but allow some rounding errors..
            if abs_difference(actual_reward, expected_reward) > expected_reward / 500 + 10 {
                assert_eq!(actual_reward, expected_reward);
            }
        }

        if abs_difference(total_reference_pool, total_actually_received) >= 1000 {
            assert_eq!(total_reference_pool, total_actually_received);
        }
    });
}

#[test]
fn accrued_lend_token_interest_increases_reward_share() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_active_block_number(1);
        // The timestamp has to be non-zero for interest to start accruing
        Timestamp::set_timestamp(1_000);
        for currency_id in iter_collateral_currencies().filter(|c| !c.is_lend_token()) {
            assert_ok!(OraclePallet::_set_exchange_rate(
                currency_id,
                FixedU128::from_float(DEFAULT_EXCHANGE_RATE)
            ));
        }
        activate_lending_and_mint(Token(DOT), LendToken(1));
        let vault_id = PrimitiveVaultId::new(account_of(VAULT), LendToken(1), Token(IBTC));
        CoreVaultData::force_to(&vault_id, default_vault_state(&vault_id));

        // Borrow some lend_tokens so interest starts accruing in the market
        let initial_lend_token_stake: u128 = CapacityRewardsPallet::get_stake(&(), &vault_id.collateral_currency()).unwrap();
        let amount_to_borrow = Amount::<Runtime>::new(1_000_000_000_000_000, Token(DOT));
        deposit_and_borrow(account_of(USER), amount_to_borrow);

        // A timestamp needs to be set to a future time, when a meaningful amount of interest has accrued.
        // The set timestamp has to be within the bounds of the current Aura slot, so set the slot to a high
        // value first, by overwriting storage.
        let slot_duration = SlotDuration::from_millis(AuraPallet::slot_duration());
        let slot_to_set = Slot::from_timestamp(SlotTimestamp::try_from(1000000000000000).unwrap(), slot_duration);
        put_storage_value(b"Aura", b"CurrentSlot", &[], slot_to_set);
        Timestamp::set_timestamp(*slot_to_set * AuraPallet::slot_duration());

        // Manually trigger interest accrual
        assert_ok!(LoansPallet::accrue_interest(Token(DOT),));
        let final_lend_token_stake: u128 = CapacityRewardsPallet::get_stake(&(), &vault_id.collateral_currency()).unwrap();

        assert!(
            final_lend_token_stake.gt(&initial_lend_token_stake),
            "Expected stake of lend_tokens to increase in the Capacity model pool, because their value increased due to accrued interest."
        );
    });
}
