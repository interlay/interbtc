use frame_support::traits::GenesisBuild;
pub use interlay_runtime_parachain::{xcm_config::*, *};
use polkadot_primitives::v2::{BlockNumber, MAX_CODE_SIZE, MAX_POV_SIZE};
use polkadot_runtime_parachains::configuration::HostConfiguration;
pub use primitives::{
    CurrencyId::Token,
    TokenSymbol::{DOT, INTR},
};
use sp_runtime::traits::AccountIdConversion;
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub const INTERLAY_PARA_ID: u32 = 2032;
pub const SIBLING_PARA_ID: u32 = 2001;

decl_test_relay_chain! {
    pub struct PolkadotNet {
        Runtime = polkadot_runtime::Runtime,
        XcmConfig = polkadot_runtime::xcm_config::XcmConfig,
        new_ext = polkadot_ext(),
    }
}

decl_test_parachain! {
    pub struct Interlay {
        Runtime = Runtime,
        Origin = Origin,
        XcmpMessageHandler = interlay_runtime_parachain::XcmpQueue,
        DmpMessageHandler = interlay_runtime_parachain::DmpQueue,
        new_ext = para_ext(INTERLAY_PARA_ID),
    }
}

decl_test_parachain! {
    pub struct Sibling {
        Runtime = testnet_interlay_runtime_parachain::Runtime,
        Origin = testnet_interlay_runtime_parachain::Origin,
        XcmpMessageHandler = testnet_interlay_runtime_parachain::XcmpQueue,
        DmpMessageHandler = testnet_interlay_runtime_parachain::DmpQueue,
        new_ext = para_ext(SIBLING_PARA_ID),
    }
}

// note: can't use SIBLING_PARA_ID and INTERLAY_PARA_ID in this macro - we are forced to use raw numbers
decl_test_network! {
    pub struct TestNet {
        relay_chain = PolkadotNet,
        parachains = vec![
            (2032, Interlay),
            (2001, Sibling),
        ],
    }
}

fn default_parachains_host_configuration() -> HostConfiguration<BlockNumber> {
    HostConfiguration {
        max_code_size: MAX_CODE_SIZE,
        max_pov_size: MAX_POV_SIZE,
        max_head_data_size: 20_480,
        max_upward_queue_count: 10,
        max_upward_queue_size: 51_200,
        max_upward_message_size: 51_200,
        max_upward_message_num_per_candidate: 10,
        hrmp_max_message_num_per_candidate: 10,
        validation_upgrade_cooldown: 14_400,
        validation_upgrade_delay: 600,
        max_downward_message_size: 51_200,
        ump_service_total_weight: 100_000_000_000,
        hrmp_max_parachain_outbound_channels: 10,
        hrmp_max_parathread_outbound_channels: 0,
        hrmp_sender_deposit: 100_000_000_000,
        hrmp_recipient_deposit: 100_000_000_000,
        hrmp_channel_max_capacity: 1_000,
        hrmp_channel_max_total_size: 102_400,
        hrmp_max_parachain_inbound_channels: 10,
        hrmp_max_parathread_inbound_channels: 0,
        hrmp_channel_max_message_size: 102_400,
        code_retention_period: 14_400,
        parathread_cores: 0,
        parathread_retries: 0,
        group_rotation_frequency: 10,
        chain_availability_period: 10,
        thread_availability_period: 10,
        scheduling_lookahead: 1,
        max_validators_per_core: Some(5),
        max_validators: Some(200),
        dispute_period: 6,
        dispute_post_conclusion_acceptance_period: 600,
        dispute_max_spam_slots: 2,
        dispute_conclusion_by_time_out_period: 600,
        no_show_slots: 2,
        n_delay_tranches: 89,
        zeroth_delay_tranche_width: 0,
        needed_approvals: 30,
        relay_vrf_modulo_samples: 40,
        ump_max_individual_weight: 20_000_000_000,
        pvf_checking_enabled: false,
        pvf_voting_ttl: 2,
        minimum_validation_upgrade_delay: 20,
    }
}

pub fn polkadot_ext() -> sp_io::TestExternalities {
    use polkadot_parachain::primitives::{HeadData, ValidationCode};
    use polkadot_runtime::{Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(AccountId::from(ALICE), 100_000_000 * DOT.one())],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    // register a parachain so that we can test opening hrmp channel
    let fake_para = polkadot_runtime_parachains::paras::ParaGenesisArgs {
        genesis_head: HeadData(vec![]),
        parachain: true,
        validation_code: ValidationCode(vec![0]),
    };
    <polkadot_runtime_parachains::paras::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
        &polkadot_runtime_parachains::paras::GenesisConfig {
            paras: vec![
                (INTERLAY_PARA_ID.into(), fake_para.clone()),
                (SIBLING_PARA_ID.into(), fake_para.clone()),
            ],
        },
        &mut t,
    )
    .unwrap();

    polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
        config: default_parachains_host_configuration(),
    }
    .assimilate_storage(&mut t)
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

pub fn para_ext(parachain_id: u32) -> sp_io::TestExternalities {
    ExtBuilder::default()
        .balances(vec![(AccountId::from(ALICE), Token(DOT), 10 * DOT.one())])
        .parachain_id(parachain_id)
        .build()
}

#[allow(dead_code)]
pub const DEFAULT: [u8; 32] = [0u8; 32];
#[allow(dead_code)]
pub const ALICE: [u8; 32] = [4u8; 32];
#[allow(dead_code)]
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

pub(crate) fn interlay_sovereign_account_on_polkadot() -> AccountId {
    polkadot_parachain::primitives::Id::from(INTERLAY_PARA_ID).into_account_truncating()
}

pub(crate) fn sibling_sovereign_account_on_polkadot() -> AccountId {
    polkadot_parachain::primitives::Id::from(SIBLING_PARA_ID).into_account_truncating()
}
