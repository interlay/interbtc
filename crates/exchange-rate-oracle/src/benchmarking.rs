use super::{Pallet as ExchangeRateOracle, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

benchmarks! {
    set_exchange_rate {
        let origin: T::AccountId = account("origin", 0, 0);
        <AuthorizedOracles<T>>::insert(origin.clone(), Vec::<u8>::new());
        let rate = UnsignedFixedPoint::<T>::checked_from_rational(1, 1).unwrap();
    }: _(RawOrigin::Signed(origin), rate)
    verify {
        assert_eq!(ExchangeRate::<T>::get(), UnsignedFixedPoint::<T>::checked_from_rational(1, 1).unwrap());
    }

    set_btc_tx_fees_per_byte {
        let u in 0 .. 1000u32;
        let origin: T::AccountId = account("origin", 0, 0);
        <AuthorizedOracles<T>>::insert(origin.clone(), Vec::<u8>::new());
    }: _(RawOrigin::Signed(origin), 1 * u, 2 * u, 3 * u)
    verify {
        let readback = SatoshiPerBytes::<T>::get();

        assert_eq!(readback.fast, 1 * u);
        assert_eq!(readback.half, 2 * u);
        assert_eq!(readback.hour, 3 * u);
    }

    insert_authorized_oracle {
        let origin: T::AccountId = account("origin", 0, 0);
    }: _(RawOrigin::Root, origin.clone(), "Origin".as_bytes().to_vec())
    verify {
        assert_eq!(ExchangeRateOracle::<T>::is_authorized(&origin), true);
    }

    remove_authorized_oracle {
        let origin: T::AccountId = account("origin", 0, 0);
        ExchangeRateOracle::<T>::insert_oracle(origin.clone(), "Origin".as_bytes().to_vec());
    }: _(RawOrigin::Root, origin.clone())
    verify {
        assert_eq!(ExchangeRateOracle::<T>::is_authorized(&origin), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_set_exchange_rate::<Test>());
            assert_ok!(test_benchmark_set_btc_tx_fees_per_byte::<Test>());
            assert_ok!(test_benchmark_insert_authorized_oracle::<Test>());
            assert_ok!(test_benchmark_remove_authorized_oracle::<Test>());
        });
    }
}
