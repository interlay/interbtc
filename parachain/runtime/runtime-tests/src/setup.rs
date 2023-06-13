pub use codec::Encode;
use frame_support::traits::GenesisBuild;
pub use frame_support::{assert_noop, assert_ok, traits::Currency};
pub use frame_system::RawOrigin;
pub use orml_traits::{location::RelativeLocations, Change, GetByKey, MultiCurrency};
pub use sp_core::H160;
pub use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, BlakeTwo256, Convert, Hash, Zero},
    DispatchError, DispatchResult, FixedPointNumber, MultiAddress, Perbill, Permill,
};
pub use xcm::latest::prelude::*;
pub use xcm_emulator::XcmExecutor;

#[cfg(feature = "with-kintsugi-runtime")]
pub use kintsugi_imports::*;
#[cfg(feature = "with-kintsugi-runtime")]
mod kintsugi_imports {
    pub use frame_support::{parameter_types, weights::Weight};
    pub use kintsugi_runtime_parachain::{xcm_config::*, *};
    pub use sp_runtime::{traits::AccountIdConversion, FixedPointNumber};

    pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(KSM);
    pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(KBTC);
    pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(KINT);
    pub const DEFAULT_GRIEFING_CURRENCY: CurrencyId = DEFAULT_NATIVE_CURRENCY;
}

#[cfg(feature = "with-interlay-runtime")]
pub use interlay_imports::*;
#[cfg(feature = "with-interlay-runtime")]
mod interlay_imports {
    pub use frame_support::{parameter_types, weights::Weight};
    pub use interlay_runtime_parachain::{xcm_config::*, *};
    pub use sp_runtime::{traits::AccountIdConversion, FixedPointNumber};

    pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(DOT);
    pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);
    pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(INTR);
    pub const DEFAULT_GRIEFING_CURRENCY: CurrencyId = DEFAULT_NATIVE_CURRENCY;
}

pub const DEFAULT: [u8; 32] = [0u8; 32];

pub const ALICE: [u8; 32] = [4u8; 32];
pub const BOB: [u8; 32] = [5u8; 32];

pub struct ExtBuilder {
    balances: Vec<(AccountId, CurrencyId, Balance)>,
    parachain_id: u32,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![],
            parachain_id: 2000,
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    #[allow(dead_code)]
    pub fn parachain_id(mut self, parachain_id: u32) -> Self {
        self.parachain_id = parachain_id;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        let native_currency_id = GetNativeCurrencyId::get();

        orml_tokens::GenesisConfig::<Runtime> {
            balances: self
                .balances
                .into_iter()
                .filter(|(_, currency_id, _)| *currency_id != native_currency_id)
                .collect::<Vec<_>>(),
        }
        .assimilate_storage(&mut t)
        .unwrap();

        <parachain_info::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
            &parachain_info::GenesisConfig {
                parachain_id: self.parachain_id.into(),
            },
            &mut t,
        )
        .unwrap();

        <pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
            &pallet_xcm::GenesisConfig {
                safe_xcm_version: Some(2),
            },
            &mut t,
        )
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
