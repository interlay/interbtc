mod mock;

use frame_support::traits::OnInitialize;
use mock::{assert_eq, *};

type AnuityPallet = annuity::Pallet<Runtime, annuity::Instance1>;
type AnuityEvent = annuity::Event<Runtime, annuity::Instance1>;

const NATIVE_CURRENCY_ID: CurrencyId = Token(INTR);

fn get_last_reward() -> u128 {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::VaultAnnuity(AnuityEvent::BlockReward(reward)) = record.event {
                Some(reward)
            } else {
                None
            }
        })
        .expect("nothing was rewarded")
}

#[test]
fn integration_test_annuitiy() {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: AnuityPallet::annuity_pallet_id(),
            currency_id: NATIVE_CURRENCY_ID,
            new_free: 10_000_000_000_000,
            new_reserved: 0,
        })
        .dispatch(root()));
        AnuityPallet::update_reward_per_block();
        AnuityPallet::on_initialize(1);

        let emission_period = <Runtime as annuity::Config<annuity::Instance1>>::EmissionPeriod::get() as u128;
        let expected_reward = 10_000_000_000_000 / emission_period as u128;
        for i in 1..1000 {
            AnuityPallet::on_initialize(i);
            assert_eq!(get_last_reward(), expected_reward);
        }
    })
}
