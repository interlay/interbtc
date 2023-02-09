mod mock;

use mock::{assert_eq, *};
use orml_traits::currency::MultiCurrency;
use sp_core::H256;
use vault_registry::DefaultVaultId;

// todo: change ALICE to an AccountId32 at some point
const ALEX: AccountId32 = AccountId32::new(ALICE);
const BEN: AccountId32 = AccountId32::new(BOB);

#[test]
fn stable_pool_with_unequal_tokens() {
    ExtBuilder::build().execute_with(|| {
        let currency_a = ForeignAsset(0);
        let currency_b = ForeignAsset(1);
        let currency_c = ForeignAsset(2);

        RuntimeCall::DexStable(DexStableCall::create_base_pool {
            currency_ids: vec![currency_a, currency_b, currency_c],
            currency_decimals: vec![12, 12, 12],
            a: 200,
            fee: 100_000_000,
            admin_fee: 0,
            admin_fee_receiver: BEN,
            lp_currency_symbol: "A+B+C".into(),
        })
        .assert_sudo_dispatch();

        TokensPallet::set_balance(root(), ALEX, currency_a, 1_000_000_000_000, 0).unwrap();
        TokensPallet::set_balance(root(), ALEX, currency_b, 100_000_000_000, 0).unwrap();
        TokensPallet::set_balance(root(), ALEX, currency_c, 100_000_000_000, 0).unwrap();

        RuntimeCall::DexStable(DexStableCall::add_liquidity {
            pool_id: 0,
            amounts: vec![1_000_000_000_000, 100_000_000_000, 100_000_000_000],
            min_mint_amount: 0,
            to: ALEX,
            deadline: 999,
        })
        .assert_dispatch(origin_of(ALEX));

        TokensPallet::set_balance(root(), ALEX, currency_a, 50_000, 0).unwrap();

        RuntimeCall::DexStable(DexStableCall::swap {
            poo_id: 0,
            from_index: 0,
            to_index: 1,
            in_amount: 50_000,
            min_out_amount: 0,
            to: ALEX,
            deadline: 99,
        })
        .assert_dispatch(origin_of(ALEX));

        // receive almost the same as in_amount, even though there is 10 times as much of the in-
        // currency than the out currency
        assert_eq!(TokensPallet::free_balance(currency_b, &ALEX), 44_598);
    })
}
