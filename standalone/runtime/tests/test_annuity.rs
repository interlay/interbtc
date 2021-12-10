mod mock;

use frame_support::traits::OnInitialize;
use mock::{assert_eq, *};

type AnnuityPallet = annuity::Pallet<Runtime, VaultAnnuityInstance>;
type AnnuityEvent = annuity::Event<Runtime, VaultAnnuityInstance>;

const NATIVE_CURRENCY_ID: CurrencyId = Token(INTR);

fn get_last_reward() -> u128 {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::VaultAnnuity(AnnuityEvent::BlockReward(reward)) = record.event {
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
            who: AnnuityPallet::account_id(),
            currency_id: NATIVE_CURRENCY_ID,
            new_free: 10_000_000_000_000,
            new_reserved: 0,
        })
        .dispatch(root()));
        AnnuityPallet::update_reward_per_block();
        AnnuityPallet::on_initialize(1);

        let emission_period = <Runtime as annuity::Config<VaultAnnuityInstance>>::EmissionPeriod::get() as u128;
        let expected_reward = 10_000_000_000_000 / emission_period as u128;
        for i in 1..1000 {
            AnnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}

#[test]
fn rewards_are_not_distributed_if_annuity_has_no_balance() {
    ExtBuilder::build().execute_with(|| {
        AnnuityPallet::update_reward_per_block();
        AnnuityPallet::on_initialize(1);

        let expected_reward = 0;
        for i in 1..1000 {
            AnnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}

#[test]
fn supply_vault_and_stake_to_vote_rewards_via_governance_from_supply_pallet() {
    ExtBuilder::build().execute_with(|| {
        // TODO: verify that initial supply is the token total supply

        // TODO: distribute the four year supply (300 million INTR) for the vault block rewards to
        // the vault annuity pallet via a scheduled governance proposal such that:
        // Year 1: 120 million INTR are distributed to the vault annuity pallet
        // Year 2: 90 million INTR are distributed to the vault annuity pallet
        // Year 3: 60 million INTR are distributed to the vault annuity pallet
        // Year 4: 30 million INTR are distributed to the vault annuity pallet
        // The funds should come from the supply pallet and not be newly minted

        // TODO: distribute the four year supply (50 million INTR) for the stake to vote rewards to
        // the stake to vote annuity pallet via a scheduled governance proposal such that:
        // Year 1: 20 million INTR are distributed to the stake-to-vote annuity pallet
        // Year 2: 15 million INTR are distributed to the stake-to-vote annuity pallet
        // Year 3: 10 million INTR are distributed to the stake-to-vote annuity pallet
        // Year 4: 5 million INTR are distributed to the stake-to-vote annuity pallet
        // The funds should come from the supply pallet and not be newly minted

        // TODO: initialize the annuity pallets
        // TODO: verify the block rewards to vaults in year 1, 2, 3, and 4
        // TODO: verify the block rewards for stake to vote in year 1, 2, 3, and 4
        AnnuityPallet::update_reward_per_block();
        AnnuityPallet::on_initialize(1);

        let expected_reward = 0;
        for i in 1..1000 {
            AnnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}
