extern crate hex;

pub use bitcoin::formatter::Formattable;
pub use bitcoin::types::*;
pub use btc_parachain_runtime::{AccountId, Call, Event, Runtime};
pub use frame_support::{assert_err, assert_ok};
pub use mocktopus::mocking::*;
use primitive_types::{H256, U256};
pub use security::StatusCode;
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use sp_std::convert::TryInto;
pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CLAIRE: [u8; 32] = [2u8; 32];
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
    btc_address: H160,
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
    dest_address: H160,
    amount: u128,
    return_data: H256,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let address = Address::from(*dest_address.as_fixed_bytes());

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
                .with_previous_hash(init_block.transactions[0].hash())
                .build(),
        )
        .add_output(TransactionOutput::p2pkh(value.into(), &address))
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
pub type OracleCall = exchange_rate_oracle::Call<Runtime>;

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
            admin: account_of(BOB),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig {
            confirmations: CONFIRMATIONS,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig {
            secure_collateral_threshold: 100000,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}
