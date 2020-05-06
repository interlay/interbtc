#[cfg(test)]

mod tests {
    use btc_parachain_runtime::{AccountId, Event, Runtime};
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

            balances::GenesisConfig::<Runtime, balances::Instance1> {
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

    //pub type IssueEvent = issue::Event<Runtime>;
    //pub type RedeemEvent = redeem::Event<Runtime>;
    pub type ReplaceEvent = replace::Event<Runtime>;

    pub type IssueCall = issue::Call<Runtime>;
    pub type VaultRegistryCall = vault_registry::Call<Runtime>;
    pub type OracleCall = exchange_rate_oracle::Call<Runtime>;
    pub type RedeemCall = redeem::Call<Runtime>;
    pub type ReplaceCall = replace::Call<Runtime>;

    #[test]
    fn replace_request() {
        ExtBuilder::build().execute_with(|| {
            SystemModule::set_block_number(1);
            let amount = 1000;
            let timeout = 10;
            let griefing_collateral = 200;

            assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));
            assert_ok!(VaultRegistryCall::register_vault(amount, H160([0; 20]))
                .dispatch(origin_of(account_of(BOB))));
            assert_ok!(
                ReplaceCall::request_replace(amount, timeout, griefing_collateral)
                    .dispatch(origin_of(account_of(BOB)))
            );

            let events = SystemModule::events();
            let record = events.iter().find(|record| match record.event {
                Event::replace(ReplaceEvent::RequestReplace(_, _, _, _)) => true,
                _ => false,
            });
            let _id = if let Event::replace(ReplaceEvent::RequestReplace(id, _, _, _)) =
                record.unwrap().event.clone()
            {
                id
            } else {
                panic!("request replace event not found")
            };
        });
    }
}
