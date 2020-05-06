pub use btc_parachain_runtime::{AccountId, Event, Runtime};
pub use btc_relay::H256Le;
pub use frame_support::{assert_err, assert_ok};
pub use mocktopus::mocking::*;
pub use security::StatusCode;
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use x_core::Error;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];

pub fn origin_of(account_id: AccountId) -> <Runtime as system::Trait>::Origin {
    <Runtime as system::Trait>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

pub type SecurityModule = security::Module<Runtime>;
pub type SystemModule = system::Module<Runtime>;

pub type VaultRegistryCall = vault_registry::Call<Runtime>;
pub type OracleCall = exchange_rate_oracle::Call<Runtime>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        balances::GenesisConfig::<Runtime, balances::Instance1> {
            balances: vec![(account_of(ALICE), 1_000_000), (account_of(BOB), 1_000_000)],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        balances::GenesisConfig::<Runtime, balances::Instance2> {
            balances: vec![(account_of(ALICE), 500_000), (account_of(BOB), 500_000)],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        exchange_rate_oracle::GenesisConfig::<Runtime> {
            admin: account_of(BOB),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}
