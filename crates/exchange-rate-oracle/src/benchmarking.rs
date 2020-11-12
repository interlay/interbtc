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

    set_btc_tx_fees_per_byte {
        let u in 0 .. 1000u32;
        let origin: T::AccountId = account("origin", 0, 0);
        <AuthorizedOracle<T>>::set(origin.clone());
    }: _(RawOrigin::Signed(origin), 1 * u, 2 * u, 3 * u)
    verify {
        let readback = SatoshiPerBytes::get();

        assert_eq!(readback.fast, 1 * u);
        assert_eq!(readback.half, 2 * u);
        assert_eq!(readback.hour, 3 * u);
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

    #[test]
    fn test_set_btc_tx_fees_per_byte() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_set_btc_tx_fees_per_byte::<Test>());
        });
    }
}
