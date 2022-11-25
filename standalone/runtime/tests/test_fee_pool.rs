mod mock;
use currency::Amount;
use interbtc_runtime_standalone::UnsignedFixedPoint;
use mock::{
    assert_eq,
    issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder},
    nomination_testing_utils::*,
    reward_testing_utils::{BasicRewardPool, IdealRewardPool, StakeHolder},
    *,
};
use rand::Rng;
use std::collections::BTreeMap;
use vault_registry::DefaultVaultId;

const VAULT_2: [u8; 32] = DAVE;

const REWARD_CURRENCY: CurrencyId = Token(INTR);

fn default_nomination(currency_id: CurrencyId) -> Amount<Runtime> {
    Amount::new(DEFAULT_NOMINATION, currency_id)
}

// assert that a and b differ by at most 1
macro_rules! assert_eq_modulo_rounding {
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
            for currency_id in iter_collateral_currencies() {
                assert_ok!(OraclePallet::_set_exchange_rate(
                    currency_id,
                    FixedU128::from_float(0.1)
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
}

fn withdraw_rewards(vault_id: &VaultId, nominator_id: &AccountId) {
    withdraw_vault_global_pool_rewards(vault_id);
    withdraw_local_pool_rewards(vault_id, nominator_id);
}

fn withdraw_vault_global_pool_rewards(vault_id: &VaultId) {
    assert_ok!(RuntimeCall::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(vault_id.account_id.clone())));
}

fn withdraw_local_pool_rewards(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    let amount = VaultStakingPallet::compute_reward(REWARD_CURRENCY, vault_id, nominator_id).unwrap();
    assert_ok!(RuntimeCall::Fee(FeeCall::withdraw_rewards {
        vault_id: vault_id.clone(),
        index: None
    })
    .dispatch(origin_of(nominator_id.clone())));
    amount
}

fn get_vault_global_pool_rewards(vault_id: &VaultId) -> i128 {
    // todo: withdraw from capacity?
    // VaultRewardsPallet::compute_reward(&vault_id.collateral_currency(), vault_id, REWARD_CURRENCY).unwrap()
    let reward =
        CapacityRewardsPallet::withdraw_reward(&(), &vault_id.collateral_currency(), vault_id.collateral_currency())
            .unwrap();
    VaultRewardsPallet::distribute_reward(
        &vault_id.collateral_currency(),
        vault_id.collateral_currency(),
        reward.into(),
    )
    .unwrap();
    VaultRewardsPallet::compute_reward(&vault_id.collateral_currency(), vault_id, REWARD_CURRENCY).unwrap()
}

fn get_local_pool_rewards(vault_id: &VaultId, nominator_id: &AccountId) -> i128 {
    staking::Pallet::<Runtime>::compute_reward(REWARD_CURRENCY, vault_id, nominator_id).unwrap()
}

fn get_vault_issued_tokens(vault_id: &VaultId) -> Amount<Runtime> {
    wrapped(VaultRegistryPallet::get_vault_from_id(vault_id).unwrap().issued_tokens)
}

fn get_vault_collateral(vault_id: &VaultId) -> Amount<Runtime> {
    VaultRegistryPallet::compute_collateral(vault_id)
        .unwrap()
        .try_into()
        .unwrap()
}

fn issue_with_vault(vault_id: &VaultId, request_amount: Amount<Runtime>) {
    let (issue_id, _) = RequestIssueBuilder::new(vault_id, request_amount)
        .with_vault(vault_id.clone())
        .request();
    ExecuteIssueBuilder::new(issue_id).assert_execute();
}

// #[test]
// fn test_vault_fee_pool_withdrawal() {
//     test_with(|vault_id_1| {
//         let vault_id_2 = VaultId {
//             account_id: account_of(VAULT2),
//             ..vault_id_1.clone()
//         };
//         let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
//         let vault_2_stake_id = StakeHolder::Vault(vault_id_2.clone());
// 
//         let rewards1 = Amount::new(1000, REWARD_CURRENCY);
//         let rewards2 = Amount::new(5000, REWARD_CURRENCY);
//         distribute_rewards(rewards1);
// 
//         let mut vault_2 = default_vault_state(&vault_id_2);
//         vault_2.backing_collateral = vault_2.backing_collateral * 4;
//         CoreVaultData::force_to(&vault_id_2, vault_2.clone());
// 
//         distribute_rewards(rewards2);
// 
//         let mut reward_pool = BasicRewardPool::default();
//         reward_pool
//             .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
//             .distribute(rewards1.amount() as f64)
//             .deposit_stake(&vault_2_stake_id, get_vault_issued_tokens(&vault_id_2).amount() as f64)
//             .distribute(rewards2.amount() as f64);
// 
//         withdraw_rewards(&vault_id_1, &vault_id_1.account_id);
//         withdraw_rewards(&vault_id_2, &vault_id_2.account_id);
// 
//         assert_eq!(
//             ParachainState::get(&vault_id_1),
//             ParachainState::get_default(&vault_id_1).with_changes(|_, vault, _, _| {
//                 let reward = Amount::new(reward_pool.compute_reward(&vault_1_stake_id) as u128, REWARD_CURRENCY);
//                 *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += reward;
//             })
//         );
//         assert_eq!(
//             CoreVaultData::vault(vault_id_2.clone()),
//             vault_2.with_changes(|vault| {
//                 let reward = Amount::new(reward_pool.compute_reward(&vault_2_stake_id) as u128, REWARD_CURRENCY);
//                 *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += reward;
//             })
//         );
//     });
// }
// 
// fn commission_for(amount: Amount<Runtime>) -> Amount<Runtime> {
//     (amount * 3) / 4 // tests take 75% commission
// }
// 
// fn nominator_share(amount: Amount<Runtime>) -> Amount<Runtime> {
//     amount / 4 // tests take 75% commission
// }
// 
// #[test]
// fn test_new_nomination_withdraws_global_reward() {
//     test_with(|vault_id_1| {
//         let currency_id = vault_id_1.collateral_currency();
//         let rewards = Amount::new(1000, REWARD_CURRENCY);
//         distribute_rewards(rewards);
// 
//         assert_eq_modulo_rounding!(get_vault_global_pool_rewards(&vault_id_1), rewards.amount() as i128,);
// 
//         assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));
// 
//         // operator receives the commission directly
//         assert_eq!(
//             ParachainState::get(&vault_id_1),
//             ParachainState::get_default(&vault_id_1).with_changes(|user, vault, _, _| {
//                 (*user.balances.get_mut(&vault_id_1.collateral_currency()).unwrap()).free -=
//                     default_nomination(currency_id);
//                 vault.backing_collateral += default_nomination(currency_id);
//                 *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += commission_for(rewards);
//             })
//         );
// 
//         // Vault rewards are withdrawn when a new nominator joins
//         assert_eq_modulo_rounding!(get_vault_global_pool_rewards(&vault_id_1), 0 as i128);
//         assert_eq_modulo_rounding!(
//             get_local_pool_rewards(&vault_id_1, &vault_id_1.account_id),
//             nominator_share(rewards).amount() as i128
//         );
//     });
// }
// 
fn distribute_rewards(amount: Amount<Runtime>) {
    // mint the tokens
    amount.mint_to(&FeePallet::fee_pool_account_id()).unwrap();

    // distribute
    FeePallet::distribute_rewards(&amount).unwrap();
}
// 
// #[test]
// fn test_fee_nomination() {
//     test_with(|vault_id| {
//         let mut global_reward_pool = BasicRewardPool::default();
//         let mut local_reward_pool = BasicRewardPool::default();
// 
//         let vault_1_stake_id = StakeHolder::Vault(vault_id.clone());
//         let user_stake_id = StakeHolder::Nominator(account_of(USER));
// 
//         let vault_issued_tokens_1 = default_vault_state(&vault_id).issued;
//         let operator_stake = default_vault_state(&vault_id).backing_collateral;
//         let nominator_stake = operator_stake / 4;
// 
//         let rewards1 = Amount::new(1000, REWARD_CURRENCY);
//         let rewards2 = Amount::new(2000, REWARD_CURRENCY);
// 
//         // perform the actions to test
//         {
//             // step 1: reward without nominator
//             distribute_rewards(rewards1);
// 
//             // step 2: add a nominator
//             assert_nominate_collateral(&vault_id, account_of(USER), nominator_stake);
// 
//             // step 3: reward with nominator
//             distribute_rewards(rewards2);
//         }
// 
//         // reproduce the expected effected of the above on the feel pools..
//         {
//             // step 0: set stake prior to getting rewards
//             global_reward_pool.deposit_stake(&vault_1_stake_id, vault_issued_tokens_1.amount() as f64);
//             local_reward_pool.deposit_stake(&vault_1_stake_id, operator_stake.amount() as f64);
// 
//             // step 1: distribute without nominator
//             global_reward_pool.distribute(rewards1.amount() as f64);
// 
//             // step 2: add a nominator. Internally this withdraws rewards from the global pool
//             let reward = global_reward_pool.compute_reward(&vault_1_stake_id) * NOMINATOR_SHARE;
//             global_reward_pool.withdraw_reward(&vault_1_stake_id);
//             local_reward_pool
//                 .distribute(reward)
//                 .deposit_stake(&user_stake_id, nominator_stake.amount() as f64);
// 
//             // step 3: reward with nominator
//             global_reward_pool.distribute(rewards2.amount() as f64);
//             local_reward_pool.distribute(global_reward_pool.compute_reward(&vault_1_stake_id) * NOMINATOR_SHARE);
//         }
// 
//         // now check
//         assert_eq_modulo_rounding!(
//             get_vault_global_pool_rewards(&vault_id),
//             global_reward_pool.compute_reward(&vault_1_stake_id) as i128,
//         );
// 
//         FeePallet::distribute_vault_rewards(&vault_id, REWARD_CURRENCY).unwrap();
// 
//         assert_eq_modulo_rounding!(get_vault_global_pool_rewards(&vault_id), 0 as i128);
// 
//         assert_eq_modulo_rounding!(
//             withdraw_local_pool_rewards(&vault_id, &vault_id.account_id),
//             local_reward_pool.compute_reward(&vault_1_stake_id) as i128,
//         );
// 
//         assert_eq_modulo_rounding!(
//             withdraw_local_pool_rewards(&vault_id, &account_of(USER)),
//             local_reward_pool.compute_reward(&user_stake_id) as i128,
//         );
//     });
// }
// 
// #[test]
// fn test_fee_nomination_slashing() {
//     test_with(|vault_id_1| {
//         let currency_id = vault_id_1.collateral_currency();
//         let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
//         let user_stake_id = StakeHolder::Nominator(account_of(USER));
// 
//         let rewards = Amount::new(1000, REWARD_CURRENCY);
//         distribute_rewards(rewards);
// 
//         assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));
// 
//         // Slash the vault and its nominator
//         VaultRegistryPallet::transfer_funds(
//             CurrencySource::Collateral(vault_id_1.clone()),
//             CurrencySource::FreeBalance(account_of(VAULT_2)),
//             &default_nomination(currency_id),
//         )
//         .unwrap();
// 
//         distribute_rewards(rewards);
// 
//         let mut global_reward_pool = BasicRewardPool::default();
//         global_reward_pool
//             .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
//             .distribute(rewards.amount() as f64);
// 
//         let vault_collateral = get_vault_collateral(&vault_id_1).amount() as f64;
//         let nominator_collateral = DEFAULT_NOMINATION as f64;
//         let slashed_amount = DEFAULT_NOMINATION as f64;
// 
//         let mut local_reward_pool = BasicRewardPool::default();
//         local_reward_pool
//             .deposit_stake(&vault_1_stake_id, vault_collateral)
//             .deposit_stake(&user_stake_id, nominator_collateral)
//             .withdraw_stake(
//                 &vault_1_stake_id,
//                 slashed_amount * vault_collateral / (vault_collateral + nominator_collateral),
//             )
//             .withdraw_stake(
//                 &user_stake_id,
//                 slashed_amount * nominator_collateral / (vault_collateral + nominator_collateral),
//             )
//             .distribute(global_reward_pool.compute_reward(&vault_1_stake_id));
// 
//         assert_eq_modulo_rounding!(
//             get_vault_global_pool_rewards(&vault_id_1),
//             local_reward_pool.compute_reward(&vault_1_stake_id) as i128
//                 + local_reward_pool.compute_reward(&user_stake_id) as i128,
//         );
//     });
// }
// 
// #[test]
// fn test_fee_nomination_withdrawal() {
//     test_with(|vault_id_1| {
//         let currency_id = vault_id_1.collateral_currency();
//         let vault_1_stake_id = StakeHolder::Vault(vault_id_1.clone());
//         let user_stake_id = StakeHolder::Nominator(account_of(USER));
// 
//         let rewards = Amount::new(1000, REWARD_CURRENCY);
// 
//         SecurityPallet::set_active_block_number(1);
//         enable_nomination();
//         assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
// 
//         issue_with_vault(&vault_id_1, wrapped(10000));
// 
//         assert_nominate_collateral(&vault_id_1, account_of(USER), default_nomination(currency_id));
//         assert_withdraw_nominator_collateral(account_of(USER), &vault_id_1, default_nomination(currency_id) / 2);
//         distribute_rewards(rewards);
// 
//         let mut global_reward_pool = BasicRewardPool::default();
//         global_reward_pool
//             .deposit_stake(&vault_1_stake_id, get_vault_issued_tokens(&vault_id_1).amount() as f64)
//             .distribute(rewards.amount() as f64);
// 
//         let mut local_reward_pool = BasicRewardPool::default();
//         local_reward_pool
//             .deposit_stake(&vault_1_stake_id, get_vault_collateral(&vault_id_1).amount() as f64)
//             .deposit_stake(&user_stake_id, DEFAULT_NOMINATION as f64)
//             .distribute(global_reward_pool.compute_reward(&user_stake_id))
//             .withdraw_reward(&vault_1_stake_id)
//             .withdraw_stake(&user_stake_id, (DEFAULT_NOMINATION / 2) as f64)
//             .distribute(global_reward_pool.compute_reward(&vault_1_stake_id));
// 
//         assert_eq_modulo_rounding!(
//             get_vault_global_pool_rewards(&vault_id_1),
//             local_reward_pool.compute_reward(&vault_1_stake_id) as i128
//                 + local_reward_pool.compute_reward(&user_stake_id) as i128,
//         );
//     });
// }
// 
// #[test]
// fn integration_test_fee_with_parachain_shutdown_fails() {
//     test_with(|vault_id_1| {
//         SecurityPallet::set_status(StatusCode::Shutdown);
//         assert_noop!(
//             RuntimeCall::Fee(FeeCall::withdraw_rewards {
//                 vault_id: vault_id_1.clone(),
//                 index: None
//             })
//             .dispatch(origin_of(vault_id_1.account_id)),
//             SystemError::CallFiltered
//         );
//     })
// }
// 
// mod reward_amount_tests {
//     use super::{assert_eq, *};
// 
//     fn test_with_2_vaults<R>(execute: impl Fn(VaultId, VaultId) -> R) {
//         test_with(|vault_id_1| {
//             let vault_id_2 = PrimitiveVaultId {
//                 account_id: account_of(VAULT2),
//                 ..vault_id_1.clone()
//             };
//             CoreVaultData::force_to(&vault_id_2, default_vault_state(&vault_id_2));
// 
//             execute(vault_id_1, vault_id_2)
//         });
//     }
// 
//     #[test]
//     fn test_fee_nomination() {
//         test_with_2_vaults(|vault_id, _vault_id_2| {
//             let reward = Amount::new(1000, REWARD_CURRENCY);
//             distribute_rewards(reward);
//             withdraw_rewards(&vault_id, &vault_id.account_id);
// 
//             assert_eq!(
//                 ParachainState::get(&vault_id),
//                 ParachainState::get_default(&vault_id).with_changes(|_, vault, _, _| {
//                     // two vaults with equal issuance -> 50% reward
//                     *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += reward / 2;
//                 })
//             );
// 
//             // same amount of collateral as operator
//             let nominator_collateral = default_vault_state(&vault_id).backing_collateral;
//             assert_nominate_collateral(&vault_id, account_of(USER), nominator_collateral);
// 
//             let state_1 = ParachainState::get(&vault_id);
// 
//             distribute_rewards(reward);
//             withdraw_rewards(&vault_id, &vault_id.account_id);
//             withdraw_rewards(&vault_id, &account_of(USER));
// 
//             assert_eq!(
//                 ParachainState::get(&vault_id),
//                 state_1.with_changes(|user, vault, _, _| {
//                     // two vaults with equal issuance -> 50% reward goes to the vault
//                     // the operator takes 75% commission and gets 50% of the remaining since
//                     // their collateral is the same as the nominator collateral
//                     let commission = ((reward / 2) * 3) / 4; // 75% commission
//                     let reward_from_collateral = ((reward / 2) - commission) / 2;
// 
//                     *vault.free_balance.get_mut(&REWARD_CURRENCY).unwrap() += commission + reward_from_collateral;
//                     (*user.balances.get_mut(&REWARD_CURRENCY).unwrap()).free += reward_from_collateral;
//                 })
//             );
//         });
//     }
// }

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
                assert_ok!(RuntimeCall::Tokens(TokensCall::set_balance {
                    who: nominator.clone(),
                    currency_id,
                    new_free: max_collateral,
                    new_reserved: 0,
                })
                .dispatch(root()));
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

            assert_ok!(
                RuntimeCall::VaultRegistry(VaultRegistryCall::set_custom_secure_threshold {
                    currency_pair: vault_id.currencies.clone(),
                    custom_threshold: Some(FixedU128::from_float(3.0)),
                })
                .dispatch(origin_of(vault_id.account_id.clone()))
            );
            let threshold = VaultRegistryPallet::get_vault_secure_threshold(&vault_id).unwrap();
            reference_pool.set_secure_threshold(&vault_id, threshold);

            OraclePallet::_set_exchange_rate(vault_id.collateral_currency(), FixedU128::one()).unwrap();
            reference_pool.set_exchange_rate(vault_id.collateral_currency(), FixedU128::one());
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
                    OraclePallet::_set_exchange_rate(currency_id, exchange_rate.clone()).unwrap();
                    reference_pool.set_exchange_rate(currency_id, exchange_rate.clone());
                }
                Action::DistributeRewards => {
                    let amount = rng.gen_range(0..10_000_000_000);
                    distribute_rewards(Amount::new(amount, REWARD_CURRENCY));
                    reference_pool.distribute_reward(amount);
                }
            };
        }
        for ((vault_id, nominator_id), _) in reference_pool.nominations() {
            withdraw_vault_global_pool_rewards(&vault_id);
            let reward = withdraw_local_pool_rewards(&vault_id, &nominator_id);
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
