mod mock;

use mock::{assert_eq, *};
use primitives::VaultCurrencyPair;
use sp_core::H256;

#[test]
fn integration_test_individual_balance_and_total_supply() {
    ExtBuilder::build().execute_with(|| {
        let span = <Runtime as escrow::Config>::Span::get();
        let current_height = SystemPallet::block_number();
        let amount_1 = 1000_000_000_000_000;

        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(ALICE),
            currency_id: Token(INTR),
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
            currency_id: Token(INTR),
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
