mod mock;

use mock::{assert_eq, *};
use reward::Rewards;
use sp_core::H256;

#[test]
fn integration_test_individual_balance_and_total_supply() {
    ExtBuilder::build().execute_with(|| {
        let span = <Runtime as escrow::Config>::Span::get();
        let current_height = SystemPallet::block_number();
        let amount_1 = 1000_000_000_000_000;

        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            new_free: amount_1,
            new_reserved: 0,
        })
        .dispatch(root()));

        assert_ok!(Call::Escrow(EscrowCall::create_lock {
            amount: amount_1,
            unlock_height: current_height + span
        })
        .dispatch(origin_of(account_of(ALICE))));

        let height_to_check = current_height + 4 * span / 10;

        assert_eq!(
            EscrowPallet::balance_at(&account_of(ALICE), Some(height_to_check)),
            EscrowPallet::total_supply(Some(height_to_check))
        );

        let amount_2 = 600_000_000_000_000;

        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(BOB),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            new_free: amount_2,
            new_reserved: 0,
        })
        .dispatch(root()));

        assert_ok!(Call::Escrow(EscrowCall::create_lock {
            amount: amount_2,
            unlock_height: current_height + span
        })
        .dispatch(origin_of(account_of(BOB))));

        assert_eq!(EscrowPallet::total_supply(Some(height_to_check)), 4615308283904);
    });
}

fn ensure_reward_stake_is_escrow_balance(height: BlockNumber) {
    assert_ok!(
        <EscrowRewardsPallet as Rewards<AccountId, Balance, CurrencyId>>::get_stake(&account_of(ALICE)),
        EscrowPallet::balance_at(&account_of(ALICE), Some(height))
    );
}

#[test]
fn integration_test_escrow_reward_stake() {
    ExtBuilder::build().execute_with(|| {
        let max_period = <Runtime as escrow::Config>::MaxPeriod::get();
        let current_height = SystemPallet::block_number();
        let create_lock_amount = 100_000_000_000;
        let increase_amount = 100_000;
        let new_free = create_lock_amount + increase_amount;

        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            new_free,
            new_reserved: 0,
        })
        .dispatch(root()));

        let unlock_height = current_height + max_period;
        assert_ok!(Call::Escrow(EscrowCall::create_lock {
            amount: create_lock_amount,
            unlock_height,
        })
        .dispatch(origin_of(account_of(ALICE))));
        ensure_reward_stake_is_escrow_balance(current_height);

        assert_ok!(Call::Escrow(EscrowCall::increase_amount {
            amount: increase_amount
        })
        .dispatch(origin_of(account_of(ALICE))));
        ensure_reward_stake_is_escrow_balance(current_height);

        SystemPallet::set_block_number(unlock_height / 2);
        let current_height = SystemPallet::block_number();

        assert_ok!(Call::Escrow(EscrowCall::increase_unlock_height {
            unlock_height: current_height + max_period
        })
        .dispatch(origin_of(account_of(ALICE))));
        ensure_reward_stake_is_escrow_balance(current_height);
    });
}
