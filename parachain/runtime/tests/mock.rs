extern crate hex;

pub use bitcoin::formatter::Formattable;
pub use bitcoin::types::*;
pub use btc_parachain_runtime::{AccountId, Event, Runtime};
pub use frame_support::{assert_err, assert_ok};
pub use mocktopus::mocking::*;
use primitive_types::{H256, U256};
pub use security::StatusCode;
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use sp_std::convert::TryInto;
pub use x_core::Error;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];

pub type BTCRelayCall = btc_relay::Call<Runtime>;

pub fn origin_of(account_id: AccountId) -> <Runtime as system::Trait>::Origin {
    <Runtime as system::Trait>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

pub fn set_default_thresholds() {
    let secure = 200_000; // 200%
    let auction = 150_000; // 150%
    let premium = 120_000; // 120%
    let liquidation = 110_000; // 110%

    vault_registry::Module::<Runtime>::_set_secure_collateral_threshold(secure);
    vault_registry::Module::<Runtime>::_set_auction_collateral_threshold(auction);
    vault_registry::Module::<Runtime>::_set_premium_redeem_threshold(premium);
    vault_registry::Module::<Runtime>::_set_liquidation_collateral_threshold(liquidation);
}

pub fn force_issue_tokens(
    user: [u8; 32],
    vault: [u8; 32],
    collateral: u128,
    tokens: u128,
    btc_address: H160,
) {
    // register the vault
    assert_ok!(VaultRegistryCall::register_vault(collateral, btc_address)
        .dispatch(origin_of(account_of(vault))));

    // increase to be issued tokens
    vault_registry::Module::<Runtime>::_increase_to_be_issued_tokens(&account_of(vault), tokens)
        .unwrap();

    // issue tokens
    assert_ok!(vault_registry::Module::<Runtime>::_issue_tokens(
        &account_of(vault),
        tokens
    ));

    // mint tokens to the user
    treasury::Module::<Runtime>::mint(user.into(), tokens);
}

pub fn generate_transaction_and_mine(
    dest_address: H160,
    amount: u128,
    return_data: H256,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let mut height = 1;
    let confirmations = 6;
    let address = Address::from(*dest_address.as_fixed_bytes());
    // initialize BTC Relay with one block
    let init_block = BlockBuilder::new()
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()));

    let init_block_hash = init_block.header.hash();
    let raw_init_block_header: sp_std::vec::Vec<u8> = Formattable::format(&init_block.header);

    assert_ok!(BTCRelayCall::initialize(
        raw_init_block_header.try_into().expect("bad block header"),
        height,
    )
    .dispatch(origin_of(account_of(ALICE))));

    height += 1;

    let value = amount as i64;
    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(TransactionInputBuilder::new().with_coinbase(false).build())
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

    let raw_block_header: sp_std::vec::Vec<u8> = Formattable::format(&block.header);

    let tx_id = transaction.tx_id();
    let tx_block_height = height;
    let proof = block.merkle_proof(&vec![tx_id]);
    let bytes_proof = proof.format();
    let raw_tx = Formattable::format_with(&transaction, true);

    assert_ok!(BTCRelayCall::store_block_header(
        raw_block_header.try_into().expect("bad block header")
    )
    .dispatch(origin_of(account_of(ALICE))));

    // FIXME: mine six new blocks to get over required confirmations
    let mut prev_block_hash = block.header.hash();
    for _ in 0..confirmations {
        let conf_block = BlockBuilder::new()
            .with_previous_hash(prev_block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let raw_conf_block_header = Formattable::format(&conf_block.header);
        assert_ok!(btc_relay::Call::<Runtime>::store_block_header(
            raw_conf_block_header.try_into().expect("bad block header"),
        )
        .dispatch(origin_of(account_of(ALICE))));
        prev_block_hash = conf_block.header.hash();
    }

    (tx_id, height, bytes_proof, raw_tx)
}
/*
pub fn generate_transaction_and_mine(
    dest_address: H160,
    amount: u128,
    return_data: H256,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let address = Address::from(*dest_address.as_fixed_bytes());

    let mut height = 1;
    // initialize BTC Relay with one block
    let init_block = BlockBuilder::new()
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()));

    let init_block_hash = init_block.header.hash();
    let raw_init_block_header = init_block
        .header
        .try_into()
        .expect("could not serialize block header");
    BTCRelayCall::initialize(raw_init_block_header, height)
        .dispatch(origin_of(account_of(ALICE)))
        .expect("could not store block header");

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

    let raw_block_header = block
        .header
        .try_into()
        .expect("could not serialize block header");

    let tx_id = transaction.tx_id();
    let tx_block_height = height;
    let proof = block.merkle_proof(&vec![tx_id]);
    let bytes_proof = proof.format();
    let raw_tx = transaction.format_with(true);

    BTCRelayCall::store_block_header(raw_block_header)
        .dispatch(origin_of(account_of(ALICE)))
        .expect("could not store block header");

    return (tx_id, height, bytes_proof, raw_tx);
}
*/

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
            balances: vec![(account_of(ALICE), 1_000_000), (account_of(BOB), 1_000_000)],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        exchange_rate_oracle::GenesisConfig::<Runtime> {
            admin: account_of(BOB),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig { confirmations: 0 }
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
