use super::*;
use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite, Linear};
use frame_system::RawOrigin;
use sp_std::vec;

#[allow(unused)]
use super::Pallet as ClientsInfo;

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn set_current_client_release(n: Linear<0, 255>, u: Linear<0, 255>) {
        let name = BoundedVec::try_from(vec![0; n as usize]).unwrap();
        let uri = BoundedVec::try_from(vec![0; u as usize]).unwrap();
        let client_release = ClientRelease {
            uri,
            checksum: Default::default(),
        };

        #[extrinsic_call]
        _(RawOrigin::Root, name.clone(), client_release.clone());

        assert_eq!(CurrentClientReleases::<T>::get(name), Some(client_release));
    }

    #[benchmark]
    fn set_pending_client_release(n: Linear<0, 255>, u: Linear<0, 255>) {
        let name = BoundedVec::try_from(vec![0; n as usize]).unwrap();
        let uri = BoundedVec::try_from(vec![0; u as usize]).unwrap();
        let client_release = ClientRelease {
            uri,
            checksum: Default::default(),
        };

        #[extrinsic_call]
        _(RawOrigin::Root, name.clone(), client_release.clone());

        assert_eq!(PendingClientReleases::<T>::get(name), Some(client_release));
    }

    impl_benchmark_test_suite!(ClientsInfo, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
