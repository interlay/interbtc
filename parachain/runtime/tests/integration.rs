#[cfg(test)]

mod tests {
    use btc_parachain_runtime::{AccountId, Event, Runtime};
    use btc_relay::H256Le;
    use frame_support::assert_ok;
    use sp_core::H160;
    use sp_runtime::traits::Dispatchable;

    const ALICE: [u8; 32] = [0u8; 32];
    const BOB: [u8; 32] = [1u8; 32];

    pub const ALICE_BALANCE: u128 = 1_000_000;
    pub const BOB_BALANCE: u128 = 1_000_000;

    pub fn origin_of(account_id: AccountId) -> <Runtime as system::Trait>::Origin {
        <Runtime as system::Trait>::Origin::signed(account_id)
    }

    pub fn account_of(address: [u8; 32]) -> AccountId {
        AccountId::from(address)
    }

    pub struct ExtBuilder;

    impl ExtBuilder {
        pub fn build() -> sp_io::TestExternalities {
            let mut storage = system::GenesisConfig::default()
                .build_storage::<Runtime>()
                .unwrap();

            balances::GenesisConfig::<Runtime> {
                balances: vec![
                    (account_of(ALICE), ALICE_BALANCE),
                    (account_of(BOB), BOB_BALANCE),
                ],
            }
            .assimilate_storage(&mut storage)
            .unwrap();

            exchange_rate_oracle::GenesisConfig::<Runtime> {
                admin: account_of(BOB),
            }
            .assimilate_storage(&mut storage)
            .unwrap();

            storage.into()
        }
    }

    pub type SystemModule = system::Module<Runtime>;

    pub type EventIssue = issue::Event<Runtime>;

    pub type CallIssue = issue::Call<Runtime>;
    pub type CallVaultRegistry = vault_registry::Call<Runtime>;
    pub type CallOracle = exchange_rate_oracle::Call<Runtime>;

    #[test]
    fn happy_path() {
        ExtBuilder::build().execute_with(|| {
            SystemModule::set_block_number(1);

            assert_ok!(CallOracle::set_exchange_rate(100).dispatch(origin_of(account_of(BOB))));

            assert_ok!(CallVaultRegistry::register_vault(1000, H160([0; 20]))
                .dispatch(origin_of(account_of(BOB))));

            assert_ok!(CallIssue::request_issue(1000, account_of(BOB), 100)
                .dispatch(origin_of(account_of(ALICE))));

            let events = SystemModule::events();
            let record = events.iter().find(|record| match record.event {
                Event::issue(EventIssue::RequestIssue(_, _, _, _, _)) => true,
                _ => false,
            });
            let id = if let Event::issue(EventIssue::RequestIssue(id, _, _, _, _)) =
                record.unwrap().event
            {
                id
            } else {
                panic!("request issue event not found")
            };

            // SystemModule::set_block_number(5);

            // assert_ok!(CallIssue::execute_issue(
            //     id,
            //     H256Le::zero(),
            //     0,
            //     vec![0u8; 32],
            //     vec![0u8; 32]
            // )
            // .dispatch(origin_of(account_of(ALICE))));
        });
    }
}
