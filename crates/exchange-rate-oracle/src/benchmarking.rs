use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks! {
    _ {}

    set_exchange_rate {
        let u in 0 .. 1000;
        let origin: T::AccountId = account("origin", 0, 0);
        <AuthorizedOracle<T>>::set(origin.clone());
    }: _(RawOrigin::Signed(origin), u.into())
    verify {
        assert_eq!(ExchangeRate::get(), u.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_set_exchange_rate() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_set_exchange_rate::<Test>());
        });
    }
}
