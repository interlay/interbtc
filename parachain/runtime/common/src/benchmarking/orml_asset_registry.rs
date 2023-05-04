use frame_benchmarking::v2::benchmarks;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_asset_registry::AssetMetadata;
use primitives::CustomMetadata;
use sp_std::{vec, vec::Vec};
use xcm::{
    latest::MultiLocation,
    opaque::lts::{Junction::*, Junctions::*},
};

pub struct Pallet<T: Config>(orml_asset_registry::Pallet<T>);
pub trait Config: orml_asset_registry::Config<CustomMetadata = CustomMetadata, Balance = u128, AssetId = u32> {}

#[benchmarks]
pub mod benchmarks {
    use super::{Config, Pallet, *};
    use orml_asset_registry::Call;

    fn longest_vec() -> Vec<u8> {
        // there is no fixed upperbound, but all actions are root-only so an assumed upperbound of 128 will do
        vec![b'a', 128]
    }

    fn longest_multilocation() -> MultiLocation {
        let key = GeneralKey {
            length: 32,
            data: [0; 32],
        };
        MultiLocation::new(1, X8(key, key, key, key, key, key, key, key))
    }

    fn get_asset_metadata() -> AssetMetadata<u128, CustomMetadata> {
        AssetMetadata {
            decimals: 12,
            name: longest_vec(),
            symbol: longest_vec(),
            existential_deposit: 0,
            location: Some(longest_multilocation().into()),
            additional: CustomMetadata {
                fee_per_second: 1_000_000_000_000,
                coingecko_id: longest_vec(),
            },
        }
    }

    #[benchmark]
    fn register_asset() {
        let metadata = get_asset_metadata();

        #[extrinsic_call]
        register_asset(RawOrigin::Root, metadata, None);
    }

    #[benchmark]
    fn update_asset() {
        let metadata = get_asset_metadata();

        assert_ok!(orml_asset_registry::Pallet::<T>::register_asset(
            RawOrigin::Root.into(),
            metadata,
            None,
        ));

        // update values, and make sure to change the actual values in case there is some optimization
        // somewhere in the codepath
        let key = GeneralKey {
            length: 32,
            data: [1; 32],
        };
        let location = MultiLocation::new(1, X8(key, key, key, key, key, key, key, key));
        #[extrinsic_call]
        update_asset(
            RawOrigin::Root,
            1,
            Some(123),
            Some(vec![b'b', 128]),
            Some(vec![b'b', 128]),
            Some(1234),
            Some(Some(location.into())),
            Some(CustomMetadata {
                fee_per_second: 123,
                coingecko_id: vec![b'b', 128],
            }),
        );
    }

    #[benchmark]
    fn set_asset_location() {
        #[block]
        {
            // todo: remove this benchmark when this unused item is removed from the weight type upstream
        }
    }
}
