use crate::setup::*;
use frame_support::traits::GenesisBuild;
use polkadot_primitives::v2::{BlockNumber, MAX_CODE_SIZE, MAX_POV_SIZE};
use polkadot_runtime_parachains::configuration::HostConfiguration;
use primitives::CurrencyId::Token;
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

decl_test_relay_chain! {
    pub struct KusamaNet {
        Runtime = kusama_runtime::Runtime,
        XcmConfig = kusama_runtime::xcm_config::XcmConfig,
        new_ext = kusama_ext(),
    }
}

decl_test_parachain! {
    pub struct Kintsugi {
        Runtime = Runtime,
        Origin = Origin,
        XcmpMessageHandler = kintsugi_runtime_parachain::XcmpQueue,
        DmpMessageHandler = kintsugi_runtime_parachain::DmpQueue,
        new_ext = para_ext(KINTSUGI_PARA_ID),
    }
}

decl_test_parachain! {
    pub struct Sibling {
        Runtime = testnet_kintsugi_runtime_parachain::Runtime,
        Origin = testnet_kintsugi_runtime_parachain::Origin,
        XcmpMessageHandler = testnet_kintsugi_runtime_parachain::XcmpQueue,
        DmpMessageHandler = testnet_kintsugi_runtime_parachain::DmpQueue,
        new_ext = para_ext(SIBLING_PARA_ID),
    }
}

// note: can't use SIBLING_PARA_ID and KINTSUGI_PARA_ID in this macro - we are forced to use raw numbers
decl_test_network! {
    pub struct TestNet {
        relay_chain = KusamaNet,
        parachains = vec![
            (2092, Kintsugi),
            (2001, Sibling),
        ],
    }
}

fn default_parachains_host_configuration() -> HostConfiguration<BlockNumber> {
    HostConfiguration {
        minimum_validation_upgrade_delay: 5,
        validation_upgrade_cooldown: 5u32,
        validation_upgrade_delay: 5,
        code_retention_period: 1200,
        max_code_size: MAX_CODE_SIZE,
        max_pov_size: MAX_POV_SIZE,
        max_head_data_size: 32 * 1024,
        group_rotation_frequency: 20,
        chain_availability_period: 4,
        thread_availability_period: 4,
        max_upward_queue_count: 8,
        max_upward_queue_size: 1024 * 1024,
        max_downward_message_size: 1024,
        ump_service_total_weight: 4 * 1_000_000_000,
        max_upward_message_size: 50 * 1024,
        max_upward_message_num_per_candidate: 5,
        hrmp_sender_deposit: 5_000_000_000_000,
        hrmp_recipient_deposit: 5_000_000_000_000,
        hrmp_channel_max_capacity: 1000,
        hrmp_channel_max_total_size: 8 * 1024,
        hrmp_max_parachain_inbound_channels: 4,
        hrmp_max_parathread_inbound_channels: 4,
        hrmp_channel_max_message_size: 102400,
        hrmp_max_parachain_outbound_channels: 4,
        hrmp_max_parathread_outbound_channels: 4,
        hrmp_max_message_num_per_candidate: 5,
        dispute_period: 6,
        no_show_slots: 2,
        n_delay_tranches: 25,
        needed_approvals: 2,
        relay_vrf_modulo_samples: 2,
        zeroth_delay_tranche_width: 0,
        ..Default::default()
    }
}

pub fn kusama_ext() -> sp_io::TestExternalities {
    use kusama_runtime::{Runtime, System};
    use polkadot_parachain::primitives::{HeadData, ValidationCode};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (AccountId::from(ALICE), 2002 * KSM.one()),
            // (ParaId::from(KINTSUGI_PARA_ID).into_account(), 2 * KSM.one()),
        ],
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
                (KINTSUGI_PARA_ID.into(), fake_para.clone()),
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
        .balances(vec![
            (AccountId::from(ALICE), Token(KSM), 10 * KSM.one()),
            // (kintsugi_runtime_parachain::TreasuryAccount::get(), Token(KSM), KSM.one()),
        ])
        .parachain_id(parachain_id)
        .build()
}
