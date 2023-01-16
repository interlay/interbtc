mod mock;
use currency::Amount;
use interbtc_runtime_standalone::UnsignedFixedPoint;
use mock::{
    assert_eq, loans_testing_utils::mint_lend_tokens, nomination_testing_utils::*,
    reward_testing_utils::IdealRewardPool, *,
};
use rand::Rng;
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
fn integration_test_fee_with_parachain_shutdown_fails() {
    test_with(|vault_id_1| {
        SecurityPallet::set_status(StatusCode::Shutdown);
        assert_noop!(
            RuntimeCall::Fee(FeeCall::withdraw_rewards {
                vault_id: vault_id_1.clone(),
                index: None
            })
            .dispatch(origin_of(vault_id_1.account_id)),
            SystemError::CallFiltered
        );
    })
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
}

impl Action {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Action {
        match rng.gen_range(0..7) {
            0 => Self::DepositNominationCollateral,
            1 => Self::WithdrawNominationCollateral,
            2 => Self::DepositVaultCollateral,
            3 => Self::WithdrawVaultCollateral,
            4 => Self::SetSecureThreshold,
            5 => Self::DistributeRewards,
            6 => Self::SetExchangeRate,
            _ => unreachable!(),
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

fn do_random_nomination_sequence() {
    test_with(|vault_id| {
        let mut rng = rand::thread_rng();

        let max_collateral = 1000;

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
                        new_free: max_collateral,
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
                let collateral_currency = match rng.gen_bool(0.5) {
                    false => token1,
                    true => token2,
                };
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

        let mut actual_rewards = BTreeMap::new();
        for _ in 0..50 {
            // 50 random actions.
            match Action::random(&mut rng) {
                Action::DepositNominationCollateral => {
                    let nominator = &nominators[rng.gen_range(0..nominators.len())];
                    let vault = &vaults[rng.gen_range(0..vaults.len())];
                    let current_stake = reference_pool.get_nominator_collateral(nominator, vault.collateral_currency());
                    let amount = rng.gen_range(0..max_collateral - current_stake);
                    reference_pool.deposit_nominator_collateral(&(vault.clone(), nominator.clone()), amount);
                    assert_nominate_collateral(
                        vault,
                        nominator.clone(),
                        Amount::new(amount, vault.collateral_currency()),
                    );
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
                        amount,
                    );

                    if max_amount == 0 {
                        continue;
                    }

                    let amount = rng.gen_range(0..max_amount);

                    reference_pool.withdraw_nominator_collateral(&(vault_id.clone(), nominator_id.clone()), amount);
                    assert_withdraw_nominator_collateral(
                        nominator_id,
                        &vault_id,
                        Amount::new(amount, vault_id.collateral_currency()),
                    );
                }
                Action::DepositVaultCollateral => {
                    let vault_id = &vaults[rng.gen_range(0..vaults.len())];
                    let max_amount =
                        CoreVaultData::vault(vault_id.clone()).free_balance[&vault_id.collateral_currency()].amount();
                    let amount = rng.gen_range(0..max_amount);
                    assert_ok!(RuntimeCall::Nomination(NominationCall::deposit_collateral {
                        vault_id: vault_id.clone(),
                        amount,
                    })
                    .dispatch(origin_of(vault_id.account_id.clone())));
                    reference_pool
                        .deposit_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), amount);
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
                    let amount = rng.gen_range(0..max_amount);
                    assert_ok!(RuntimeCall::Nomination(NominationCall::withdraw_collateral {
                        vault_id: vault_id.clone(),
                        index: None,
                        amount,
                    })
                    .dispatch(origin_of(vault_id.account_id.clone())));
                    reference_pool
                        .withdraw_nominator_collateral(&(vault_id.clone(), vault_id.account_id.clone()), amount);
                }
                Action::SetSecureThreshold => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let threshold = FixedU128::from_float(rng.gen_range(2.0..5.0));

                    assert_ok!(
                        RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                            currency_pair: vault_id.currencies.clone(),
                            custom_threshold: Some(threshold),
                        })
                        .dispatch(origin_of(vault_id.account_id.clone()))
                    );

                    reference_pool.set_secure_threshold(&vault_id, threshold);
                }
                Action::SetExchangeRate => {
                    let vault_id = vaults[rng.gen_range(0..vaults.len())].clone();
                    let currency_id = vault_id.collateral_currency();
                    let exchange_rate = FixedU128::from_float(rng.gen_range(0.5..5.0));
                    if currency_id.is_lend_token() {
                        let underlying_id = LoansPallet::underlying_id(vault_id.collateral_currency()).unwrap();
                        let lend_token_rate = exchange_rate.mul(LoansPallet::exchange_rate(underlying_id));
                        // Only need to set the exchange rate of the underlying currency in the oracle pallet
                        OraclePallet::_set_exchange_rate(underlying_id, exchange_rate.clone()).unwrap();
                        // The reference pool must store both exchange rates explicitly
                        reference_pool.set_exchange_rate(currency_id, lend_token_rate);
                        reference_pool.set_exchange_rate(underlying_id, exchange_rate.clone());
                    } else {
                        OraclePallet::_set_exchange_rate(currency_id, exchange_rate.clone()).unwrap();
                        reference_pool.set_exchange_rate(currency_id, exchange_rate.clone());
                        if let Ok(lend_token_id) = LoansPallet::lend_token_id(currency_id.clone()) {
                            let lend_token_rate = exchange_rate.div(LoansPallet::exchange_rate(currency_id.clone()));
                            reference_pool.set_exchange_rate(lend_token_id, lend_token_rate);
                        }
                    }
                }
                Action::DistributeRewards => {
                    let amount = rng.gen_range(0..10_000_000_000);
                    distribute_rewards(Amount::new(amount, REWARD_CURRENCY));
                    reference_pool.distribute_reward(amount);
                }
            };
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

        assert!(abs_difference(total_reference_pool, total_actually_received) < 1000);

        // check the rewards of all stakeholders
        for (nominator_id, expected_reward) in reference_pool.rewards() {
            // vaults had some initial free balance in the reward currency - compensate for that..
            let actual_reward = if vaults.iter().any(|x| x.account_id == nominator_id) {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount() - 200_000
            } else {
                currency::get_free_balance::<Runtime>(REWARD_CURRENCY, &nominator_id).amount()
            };
            // ensure the difference is small, but allow some rounding errors..
            if abs_difference(actual_reward, expected_reward) > expected_reward / 10_000 + 10 {
                assert_eq!(actual_reward, expected_reward);
            }
        }
    })
}
