extern crate hex;

pub use bitcoin::formatter::Formattable;
pub use bitcoin::types::*;
pub use btc_parachain_runtime::{AccountId, Call, Event, Runtime};
pub use btc_relay::BtcAddress;
pub use frame_support::{assert_err, assert_ok};
pub use mocktopus::mocking::*;
use primitive_types::{H256, U256};
pub use security::StatusCode;
pub use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use sp_std::convert::TryInto;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CLAIRE: [u8; 32] = [2u8; 32];

pub const LIQUIDATION_VAULT: [u8; 32] = [3u8; 32];
pub const FEE_POOL: [u8; 32] = [4u8; 32];
pub const MAINTAINER: [u8; 32] = [5u8; 32];

pub const CONFIRMATIONS: u32 = 6;

pub type BTCRelayCall = btc_relay::Call<Runtime>;
pub type BTCRelayEvent = btc_relay::Event;

pub fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Trait>::Origin {
    <Runtime as frame_system::Trait>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

#[allow(dead_code)]
pub fn set_default_thresholds() {
    let secure = 200_000; // 200%
    let auction = 150_000; // 150%
    let premium = 120_000; // 120%
    let liquidation = 110_000; // 110%

    VaultRegistryModule::_set_secure_collateral_threshold(secure);
    VaultRegistryModule::_set_auction_collateral_threshold(auction);
    VaultRegistryModule::_set_premium_redeem_threshold(premium);
    VaultRegistryModule::_set_liquidation_collateral_threshold(liquidation);
}

#[allow(dead_code)]
pub fn force_issue_tokens(
    user: [u8; 32],
    vault: [u8; 32],
    collateral: u128,
    tokens: u128,
    btc_address: BtcAddress,
) {
    // register the vault
    assert_ok!(
        Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, btc_address))
            .dispatch(origin_of(account_of(vault)))
    );

    // increase to be issued tokens
    VaultRegistryModule::_increase_to_be_issued_tokens(&account_of(vault), tokens).unwrap();

    // issue tokens
    assert_ok!(VaultRegistryModule::_issue_tokens(
        &account_of(vault),
        tokens
    ));

    // mint tokens to the user
    treasury::Module::<Runtime>::mint(user.into(), tokens);
}

pub fn assert_store_main_chain_header_event(height: u32, hash: H256Le) {
    let store_event = Event::btc_relay(BTCRelayEvent::StoreMainChainHeader(height, hash));
    let events = SystemModule::events();

    // store only main chain header
    assert!(events.iter().any(|a| a.event == store_event));
}

#[allow(dead_code)]
pub fn generate_transaction_and_mine(
    address: BtcAddress,
    amount: u128,
    return_data: H256,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    generate_transaction_and_mine_with_script_sig(address, amount, return_data, &vec![])
}

#[allow(dead_code)]
pub fn generate_transaction_and_mine_with_script_sig(
    address: BtcAddress,
    amount: u128,
    return_data: H256,
    script: &[u8],
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let mut height = 1;
    let confirmations = 6;

    // initialize BTC Relay with one block
    let init_block = BlockBuilder::new()
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()));

    let init_block_hash = init_block.header.hash();
    let raw_init_block_header = RawBlockHeader::from_bytes(&init_block.header.format())
        .expect("could not serialize block header");

    assert_ok!(Call::BTCRelay(BTCRelayCall::initialize(
        raw_init_block_header.try_into().expect("bad block header"),
        height,
    ))
    .dispatch(origin_of(account_of(ALICE))));

    height += 1;

    let value = amount as i64;
    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(
            TransactionInputBuilder::new()
                .with_coinbase(false)
                .with_script(script)
                .with_previous_hash(init_block.transactions[0].hash())
                .build(),
        )
        .add_output(TransactionOutput::payment(value.into(), &address))
        .add_output(TransactionOutput::op_return(0, return_data.as_bytes()))
        .build();

    let block = BlockBuilder::new()
        .with_previous_hash(init_block_hash)
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588814835)
        .add_transaction(transaction.clone())
        .mine(U256::from(2).pow(254.into()));

    let raw_block_header = RawBlockHeader::from_bytes(&block.header.format())
        .expect("could not serialize block header");

    let tx_id = transaction.tx_id();
    let tx_block_height = height;
    let proof = block.merkle_proof(&vec![tx_id]);
    let bytes_proof = proof.format();
    let raw_tx = transaction.format_with(true);

    assert_ok!(Call::BTCRelay(BTCRelayCall::store_block_header(
        raw_block_header.try_into().expect("bad block header")
    ))
    .dispatch(origin_of(account_of(ALICE))));
    assert_store_main_chain_header_event(height, block.header.hash());

    // Mine six new blocks to get over required confirmations
    let mut prev_block_hash = block.header.hash();
    let mut timestamp = 1588814835;
    for _ in 0..confirmations {
        height += 1;
        timestamp += 1000;
        let conf_block = BlockBuilder::new()
            .with_previous_hash(prev_block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(timestamp)
            .mine(U256::from(2).pow(254.into()));

        let raw_conf_block_header = RawBlockHeader::from_bytes(&conf_block.header.format())
            .expect("could not serialize block header");
        assert_ok!(Call::BTCRelay(BTCRelayCall::store_block_header(
            raw_conf_block_header.try_into().expect("bad block header"),
        ))
        .dispatch(origin_of(account_of(ALICE))));

        assert_store_main_chain_header_event(height, conf_block.header.hash());

        prev_block_hash = conf_block.header.hash();
    }

    (tx_id, tx_block_height, bytes_proof, raw_tx)
}

#[allow(dead_code)]
pub type SecurityModule = security::Module<Runtime>;
#[allow(dead_code)]
pub type SystemModule = frame_system::Module<Runtime>;
#[allow(dead_code)]
pub type SecurityError = security::Error<Runtime>;

#[allow(dead_code)]
pub type VaultRegistryCall = vault_registry::Call<Runtime>;
#[allow(dead_code)]
pub type VaultRegistryModule = vault_registry::Module<Runtime>;

#[allow(dead_code)]
pub type ExchangeRateOracleCall = exchange_rate_oracle::Call<Runtime>;
#[allow(dead_code)]
pub type ExchangeRateOracleModule = exchange_rate_oracle::Module<Runtime>;

#[allow(dead_code)]
pub type SlaModule = sla::Module<Runtime>;

#[allow(dead_code)]
pub type FeeModule = fee::Module<Runtime>;

#[allow(dead_code)]
pub type CollateralModule = collateral::Module<Runtime>;

#[allow(dead_code)]
pub type TreasuryModule = treasury::Module<Runtime>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance1> {
            balances: vec![
                (account_of(ALICE), 1_000_000),
                (account_of(BOB), 1_000_000),
                (account_of(CLAIRE), 1_000_000),
            ],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance2> { balances: vec![] }
            .assimilate_storage(&mut storage)
            .unwrap();

        exchange_rate_oracle::GenesisConfig::<Runtime> {
            authorized_oracles: vec![(account_of(BOB), BOB.to_vec())],
            max_delay: 3600000, // one hour
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig::<Runtime> {
            bitcoin_confirmations: CONFIRMATIONS,
            parachain_confirmations: CONFIRMATIONS,
            disable_difficulty_check: false,
            disable_inclusion_check: false,
            disable_op_return_check: false,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Runtime> {
            minimum_collateral_vault: 0,
            punishment_delay: 8,
            secure_collateral_threshold: 100_000,
            auction_collateral_threshold: 150_000,
            premium_redeem_threshold: 120_000,
            liquidation_collateral_threshold: 110_000,
            liquidation_vault: account_of(LIQUIDATION_VAULT),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        issue::GenesisConfig::<Runtime> { issue_period: 10 }
            .assimilate_storage(&mut storage)
            .unwrap();

        redeem::GenesisConfig::<Runtime> {
            redeem_period: 10,
            redeem_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        replace::GenesisConfig::<Runtime> {
            replace_period: 10,
            replace_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        fee::GenesisConfig::<Runtime> {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(),                // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(),   // 10%
            fee_pool_account_id: account_of(FEE_POOL),
            maintainer_account_id: account_of(MAINTAINER),
            epoch_period: 5,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(),
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            collator_rewards: FixedU128::checked_from_integer(0).unwrap(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sla::GenesisConfig::<Runtime> {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(-10),
            vault_executed_issue_max_sla_change: FixedI128::from(4),
            vault_submitted_issue_proof: FixedI128::from(0),
            relayer_target_sla: FixedI128::from(100),
            relayer_block_submission: FixedI128::from(1),
            relayer_correct_no_data_vote_or_report: FixedI128::from(1),
            relayer_correct_invalid_vote_or_report: FixedI128::from(10),
            relayer_correct_liquidation_report: FixedI128::from(1),
            relayer_correct_theft_report: FixedI128::from(1),
            relayer_correct_oracle_offline_report: FixedI128::from(1),
            relayer_false_no_data_vote_or_report: FixedI128::from(-10),
            relayer_false_invalid_vote_or_report: FixedI128::from(-100),
            relayer_ignored_vote: FixedI128::from(-10),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}
