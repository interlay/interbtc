use frame_benchmarking::v2::{account, benchmarks, whitelisted_caller};
use frame_support::{
    assert_ok,
    traits::{Currency, Get},
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use orml_traits::MultiCurrency;
use orml_vesting::VestingSchedule;
use primitives::CurrencyId;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

const SEED: u32 = 0;

pub struct Pallet<T: Config>(orml_vesting::Pallet<T>);
pub trait Config: orml_vesting::Config + currency::Config<CurrencyId = CurrencyId> {}
pub fn lookup_of_account<T: Config>(
    who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
    <T as frame_system::Config>::Lookup::unlookup(who)
}

pub(crate) type BalanceOf<T> =
    <<T as orml_vesting::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub(crate) type VestingScheduleOf<T> = VestingSchedule<BlockNumberFor<T>, BalanceOf<T>>;

fn dummy_schedule<T: Config>() -> VestingScheduleOf<T> {
    VestingSchedule {
        start: 0u32.into(),
        period: 10u32.into(),
        period_count: 2,
        per_period: 10u32.into(),
    }
}

struct Transfer<T: Config> {
    from: T::AccountId,
    to: T::AccountId,
}

fn setup_transfer<T: Config>() -> Transfer<T> {
    let from: T::AccountId = whitelisted_caller();
    let to: T::AccountId = account("to", 0, SEED);
    let amount = 1000000000u32.into();
    let native_currency = <T as currency::Config>::GetNativeCurrencyId::get();
    assert_ok!(orml_tokens::Pallet::<T>::deposit(native_currency, &from, amount));

    Transfer { from, to }
}

#[benchmarks]
pub mod benchmarks {
    use super::{Config, Pallet, *};
    use orml_vesting::Call;

    #[benchmark]
    fn claim(n: Linear<0, 1>) {
        let setup = setup_transfer::<T>();
        let schedule = dummy_schedule::<T>();
        for _ in 0..n {
            assert_ok!(orml_vesting::Pallet::<T>::vested_transfer(
                RawOrigin::Signed(setup.from.clone()).into(),
                lookup_of_account::<T>(setup.to.clone()),
                schedule.clone()
            ));
        }

        #[extrinsic_call]
        claim(RawOrigin::Signed(setup.to));
    }

    #[benchmark]
    fn vested_transfer() {
        let setup = setup_transfer::<T>();
        let schedule = dummy_schedule::<T>();
        #[extrinsic_call]
        vested_transfer(
            RawOrigin::Signed(setup.from),
            lookup_of_account::<T>(setup.to),
            schedule,
        );
    }

    #[benchmark]
    fn update_vesting_schedules(n: Linear<0, 1>) {
        let setup = setup_transfer::<T>();
        let schedule = dummy_schedule::<T>();
        assert_ok!(orml_vesting::Pallet::<T>::vested_transfer(
            RawOrigin::Signed(setup.from).into(),
            lookup_of_account::<T>(setup.to.clone()),
            schedule.clone()
        ));
        let new_schedule = VestingSchedule {
            per_period: 1u32.into(),
            ..schedule
        };
        #[extrinsic_call]
        update_vesting_schedules(
            RawOrigin::Root,
            lookup_of_account::<T>(setup.to.clone()),
            vec![new_schedule; n as usize],
        );
    }
}
