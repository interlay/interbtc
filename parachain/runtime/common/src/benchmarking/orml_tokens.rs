use frame_benchmarking::v2::{account, benchmarks, whitelisted_caller};
use frame_support::{assert_ok, traits::Get};
use frame_system::RawOrigin;
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::{CurrencyId, Rate, Ratio};
use sp_runtime::{traits::StaticLookup, FixedPointNumber};
use sp_std::prelude::*;
// use orml_traits::MultiCurrency;

const SEED: u32 = 0;

pub struct Pallet<T: Config>(orml_tokens::Pallet<T>);
pub trait Config: orml_tokens::Config + currency::Config<CurrencyId = CurrencyId> + loans::Config {}
pub fn lookup_of_account<T: Config>(
    who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
    <T as frame_system::Config>::Lookup::unlookup(who)
}

use loans::{InterestRateModel, JumpModel, Market, MarketState};
struct Tokens {
    underlying: CurrencyId,
    lend_token: CurrencyId,
}
fn setup_lending<T: Config>() -> Tokens {
    let lend_token = CurrencyId::LendToken(1);
    let market = Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        state: MarketState::Pending,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        lend_token_id: lend_token,
    };

    let underlying = <T as currency::Config>::GetNativeCurrencyId::get();
    assert_ok!(loans::Pallet::<T>::add_market(
        RawOrigin::Root.into(),
        underlying,
        market
    ));
    assert_ok!(loans::Pallet::<T>::activate_market(RawOrigin::Root.into(), underlying));

    // rewards need to come from non-root:
    let non_root: T::AccountId = account("non-root", 0, 0);

    let reward_amount = 100000000;
    assert_ok!(<orml_tokens::Pallet<T>>::deposit(underlying, &non_root, reward_amount));
    assert_ok!(loans::Pallet::<T>::add_reward(
        RawOrigin::Signed(non_root).into(),
        reward_amount
    ));
    assert_ok!(loans::Pallet::<T>::update_market_reward_speed(
        RawOrigin::Root.into(),
        underlying,
        Some(123),
        Some(234)
    ));

    Tokens { underlying, lend_token }
}

struct Transfer<T: Config> {
    from: T::AccountId,
    to: T::AccountId,
    amount: <T as orml_tokens::Config>::Balance,
}
fn setup_transfer<T: Config>(tokens: &Tokens) -> Transfer<T> {
    let from: T::AccountId = whitelisted_caller();
    let to: T::AccountId = account("to", 0, SEED);
    let amount = 10u32.into();

    assert_ok!(orml_tokens::Pallet::<T>::deposit(tokens.lend_token, &from, amount));

    // worst case: send qTokens to an account that already has collateral
    assert_ok!(orml_tokens::Pallet::<T>::deposit(tokens.lend_token, &to, amount));
    assert_ok!(loans::Pallet::<T>::deposit_all_collateral(
        RawOrigin::Signed(to.clone()).into(),
        tokens.underlying
    ));

    Transfer { from, to, amount }
}
#[benchmarks]
pub mod benchmarks {
    use super::{Config, Pallet, *};
    use orml_tokens::Call;

    #[benchmark]
    fn transfer() {
        let tokens = setup_lending::<T>();
        let Transfer { from, to, amount } = setup_transfer::<T>(&tokens);

        #[extrinsic_call]
        transfer(
            RawOrigin::Signed(from),
            lookup_of_account::<T>(to.clone()),
            tokens.lend_token,
            amount,
        );

        // check that it got reserved
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiCurrency<T::AccountId>>::free_balance(tokens.lend_token, &to),
            0
        );
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(
                tokens.lend_token,
                &to
            ),
            2 * amount
        );
    }

    #[benchmark]
    fn transfer_all() {
        let tokens = setup_lending::<T>();
        let Transfer { from, to, amount } = setup_transfer::<T>(&tokens);

        #[extrinsic_call]
        transfer_all(
            RawOrigin::Signed(from),
            lookup_of_account::<T>(to.clone()),
            tokens.lend_token,
            false,
        );

        // check that it got reserved
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiCurrency<T::AccountId>>::free_balance(tokens.lend_token, &to),
            0
        );
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(
                tokens.lend_token,
                &to
            ),
            2 * amount
        );
    }

    #[benchmark]
    fn transfer_keep_alive() {
        let tokens = setup_lending::<T>();
        let Transfer { from, to, amount } = setup_transfer::<T>(&tokens);

        #[extrinsic_call]
        transfer_keep_alive(
            RawOrigin::Signed(from),
            lookup_of_account::<T>(to.clone()),
            tokens.lend_token,
            amount,
        );

        // check that it got reserved
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiCurrency<T::AccountId>>::free_balance(tokens.lend_token, &to),
            0
        );
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(
                tokens.lend_token,
                &to
            ),
            2 * amount
        );
    }

    #[benchmark]
    fn force_transfer() {
        let tokens = setup_lending::<T>();
        let Transfer { from, to, amount } = setup_transfer::<T>(&tokens);

        #[extrinsic_call]
        force_transfer(
            RawOrigin::Root,
            lookup_of_account::<T>(from),
            lookup_of_account::<T>(to.clone()),
            tokens.lend_token,
            amount,
        );

        // check that it got reserved
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiCurrency<T::AccountId>>::free_balance(tokens.lend_token, &to),
            0
        );
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(
                tokens.lend_token,
                &to
            ),
            2 * amount
        );
    }

    #[benchmark]
    fn set_balance() {
        let account: T::AccountId = whitelisted_caller();
        let free = 123;
        let reserved = 234;
        let token = CurrencyId::ForeignAsset(1);

        #[extrinsic_call]
        set_balance(
            RawOrigin::Root,
            lookup_of_account::<T>(account.clone()),
            token,
            free,
            reserved,
        );

        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiCurrency<T::AccountId>>::free_balance(token, &account),
            free
        );
        assert_eq!(
            <orml_tokens::Pallet::<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(token, &account),
            reserved
        );
    }
}
