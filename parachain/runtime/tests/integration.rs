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

<<<<<<< HEAD
            balances::GenesisConfig::<Runtime> {
=======
            balances::GenesisConfig::<Runtime, balances::Instance1> {
>>>>>>> e481c70... parachain integration tests
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

<<<<<<< HEAD
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
=======
    pub type IssueEvent = issue::Event<Runtime>;
    pub type RedeemEvent = redeem::Event<Runtime>;

    pub type IssueCall = issue::Call<Runtime>;
    pub type VaultRegistryCall = vault_registry::Call<Runtime>;
    pub type OracleCall = exchange_rate_oracle::Call<Runtime>;
    pub type RedeemCall = redeem::Call<Runtime>;

    #[test]
    fn issue_polka_btc() {
        ExtBuilder::build().execute_with(|| {
            SystemModule::set_block_number(1);

            assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));

            assert_ok!(VaultRegistryCall::register_vault(1000, H160([0; 20]))
                .dispatch(origin_of(account_of(BOB))));

            assert_ok!(IssueCall::request_issue(1000, account_of(BOB), 100)
>>>>>>> e481c70... parachain integration tests
                .dispatch(origin_of(account_of(ALICE))));

            let events = SystemModule::events();
            let record = events.iter().find(|record| match record.event {
<<<<<<< HEAD
                Event::issue(EventIssue::RequestIssue(_, _, _, _, _)) => true,
                _ => false,
            });
            let id = if let Event::issue(EventIssue::RequestIssue(id, _, _, _, _)) =
=======
                Event::issue(IssueEvent::RequestIssue(_, _, _, _, _)) => true,
                _ => false,
            });
            let id = if let Event::issue(IssueEvent::RequestIssue(id, _, _, _, _)) =
>>>>>>> e481c70... parachain integration tests
                record.unwrap().event
            {
                id
            } else {
                panic!("request issue event not found")
            };

            // SystemModule::set_block_number(5);

<<<<<<< HEAD
            // assert_ok!(CallIssue::execute_issue(
=======
            // assert_ok!(IssueCall::execute_issue(
>>>>>>> e481c70... parachain integration tests
            //     id,
            //     H256Le::zero(),
            //     0,
            //     vec![0u8; 32],
            //     vec![0u8; 32]
            // )
            // .dispatch(origin_of(account_of(ALICE))));
        });
    }
<<<<<<< HEAD
=======

    #[test]
    fn redeem_polka_btc() {
        ExtBuilder::build().execute_with(|| {
            SystemModule::set_block_number(1);

            assert_ok!(OracleCall::set_exchange_rate(1).dispatch(origin_of(account_of(BOB))));

            assert_ok!(VaultRegistryCall::register_vault(10000, H160([0; 20]))
                .dispatch(origin_of(account_of(BOB))));

            assert_ok!(
                vault_registry::Module::<Runtime>::_increase_to_be_issued_tokens(
                    &account_of(BOB),
                    1000,
                ),
                H160([0; 20])
            );

            assert_ok!(vault_registry::Module::<Runtime>::_issue_tokens(
                &account_of(BOB),
                1000
            ));

            assert_ok!(
                RedeemCall::request_redeem(1000, H160([0; 20]), account_of(BOB))
                    .dispatch(origin_of(account_of(ALICE)))
            );

            let events = SystemModule::events();
            let record = events.iter().find(|record| match record.event {
                Event::redeem(RedeemEvent::RequestRedeem(_, _, _, _, _)) => true,
                _ => false,
            });
            let id = if let Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _)) =
                record.unwrap().event
            {
                id
            } else {
                panic!("request redeem event not found")
            };
        });
    }
>>>>>>> e481c70... parachain integration tests
}
