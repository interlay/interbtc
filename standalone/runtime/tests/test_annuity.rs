mod mock;

use codec::Encode;
use frame_support::traits::{schedule::MaybeHashed, Currency, OnInitialize};
use mock::{assert_eq, *};
use sp_runtime::Permill;

type EscrowAnnuityPallet = annuity::Pallet<Runtime, EscrowAnnuityInstance>;

type VaultAnnuityPallet = annuity::Pallet<Runtime, VaultAnnuityInstance>;
type VaultAnnuityEvent = annuity::Event<Runtime, VaultAnnuityInstance>;

type SupplyPallet = supply::Pallet<Runtime>;

fn get_last_reward() -> Balance {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::VaultAnnuity(VaultAnnuityEvent::BlockReward(reward)) = record.event {
                Some(reward)
            } else {
                None
            }
        })
        .expect("nothing was rewarded")
}

#[test]
fn integration_test_annuity() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: VaultAnnuityPallet::account_id(),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            new_free: 10_000_000_000_000,
            new_reserved: 0,
        })
        .dispatch(root()));
        VaultAnnuityPallet::update_reward_per_block();
        VaultAnnuityPallet::on_initialize(1);

        let emission_period = <Runtime as annuity::Config<VaultAnnuityInstance>>::EmissionPeriod::get() as u128;
        let expected_reward = 10_000_000_000_000 / emission_period as u128;
        for i in 1..1000 {
            VaultAnnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}

#[test]
fn rewards_are_not_distributed_if_annuity_has_no_balance() {
    ExtBuilder::build().execute_with(|| {
        VaultAnnuityPallet::update_reward_per_block();
        VaultAnnuityPallet::on_initialize(1);

        let expected_reward = 0;
        for i in 1..1000 {
            VaultAnnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}

#[test]
fn should_distribute_vault_rewards_from_supply() {
    ExtBuilder::build().execute_with(|| {
        // full distribution is minted on genesis
        assert_eq!(
            NativeCurrency::total_balance(&SupplyPallet::account_id()),
            token_distribution::INITIAL_ALLOCATION
        );

        // distribute the four year supply (300 million INTR) for the vault block rewards
        let total_rewards = Permill::from_percent(30) * token_distribution::INITIAL_ALLOCATION;
        // NOTE: start height cannot be the current height or in the past
        let start_height = SystemPallet::block_number() + 1;
        assert_ok!(Call::Utility(UtilityCall::batch {
            calls: vec![
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 1".encode(),
                    when: start_height + YEARS * 0,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: VaultAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(40) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 2".encode(),
                    when: start_height + YEARS * 1,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: VaultAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(30) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 3".encode(),
                    when: start_height + YEARS * 2,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: VaultAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(20) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 4".encode(),
                    when: start_height + YEARS * 3,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: VaultAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(10) * total_rewards,
                    }))),
                })
            ],
        })
        .dispatch(root()));

        // Year 1: 120 million INTR are distributed to the vault annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 0);
        assert_eq!(
            NativeCurrency::total_balance(&VaultAnnuityPallet::account_id()),
            Permill::from_percent(40) * total_rewards
        );

        // Year 2: 90 million INTR are distributed to the vault annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 1);
        assert_eq!(
            NativeCurrency::total_balance(&VaultAnnuityPallet::account_id()),
            Permill::from_percent(70) * total_rewards
        );

        // Year 3: 60 million INTR are distributed to the vault annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 2);
        assert_eq!(
            NativeCurrency::total_balance(&VaultAnnuityPallet::account_id()),
            Permill::from_percent(90) * total_rewards
        );

        // Year 4: 30 million INTR are distributed to the vault annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 3);
        assert_eq!(
            NativeCurrency::total_balance(&VaultAnnuityPallet::account_id()),
            Permill::from_percent(100) * total_rewards
        );
    })
}

#[test]
fn should_distribute_escrow_rewards_from_supply() {
    ExtBuilder::build().execute_with(|| {
        // full distribution is minted on genesis
        assert_eq!(
            NativeCurrency::total_balance(&SupplyPallet::account_id()),
            token_distribution::INITIAL_ALLOCATION
        );

        // distribute the four year supply (50 million INTR) for the stake to vote rewards
        let total_rewards = Permill::from_percent(5) * token_distribution::INITIAL_ALLOCATION;
        // NOTE: start height cannot be the current height or in the past
        let start_height = SystemPallet::block_number() + 1;
        assert_ok!(Call::Utility(UtilityCall::batch {
            calls: vec![
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 1".encode(),
                    when: start_height + YEARS * 0,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: EscrowAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(25) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 2".encode(),
                    when: start_height + YEARS * 1,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: EscrowAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(25) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 3".encode(),
                    when: start_height + YEARS * 2,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: EscrowAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(25) * total_rewards,
                    }))),
                }),
                Call::Scheduler(SchedulerCall::schedule_named {
                    id: "Year 4".encode(),
                    when: start_height + YEARS * 3,
                    maybe_periodic: None,
                    priority: 63,
                    call: Box::new(MaybeHashed::Value(Call::Tokens(TokensCall::force_transfer {
                        source: SupplyPallet::account_id(),
                        dest: EscrowAnnuityPallet::account_id(),
                        currency_id: DEFAULT_NATIVE_CURRENCY,
                        amount: Permill::from_percent(25) * total_rewards,
                    }))),
                })
            ],
        })
        .dispatch(root()));

        // Year 1: 12,500,000 INTR are distributed to the stake-to-vote annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 0);
        assert_eq!(
            NativeCurrency::total_balance(&EscrowAnnuityPallet::account_id()),
            Permill::from_percent(25) * total_rewards
        );

        // Year 2: 12,500,000 INTR are distributed to the stake-to-vote annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 1);
        assert_eq!(
            NativeCurrency::total_balance(&EscrowAnnuityPallet::account_id()),
            Permill::from_percent(50) * total_rewards
        );

        // Year 3: 12,500,000 INTR are distributed to the stake-to-vote annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 2);
        assert_eq!(
            NativeCurrency::total_balance(&EscrowAnnuityPallet::account_id()),
            Permill::from_percent(75) * total_rewards
        );

        // Year 4: 12,500,000 INTR are distributed to the stake-to-vote annuity pallet
        SchedulerPallet::on_initialize(start_height + YEARS * 3);
        assert_eq!(
            NativeCurrency::total_balance(&EscrowAnnuityPallet::account_id()),
            Permill::from_percent(100) * total_rewards
        );
    })
}
