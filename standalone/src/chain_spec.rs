use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use hex_literal::hex;
use interbtc_runtime::{
    AccountId, AuraConfig, BTCRelayConfig, CurrencyId, FeeConfig, GenesisConfig, GetWrappedCurrencyId,
     IssueConfig, NominationConfig, OracleConfig, RedeemConfig, RefundConfig, ReplaceConfig,
    SecurityConfig, Signature, StatusCode, SudoConfig, SystemConfig, TechnicalCommitteeConfig, TokensConfig,
    VaultRegistryConfig, BITCOIN_BLOCK_SPACING, DAYS, DOT, INTR, KINT, KSM, WASM_BINARY,
};
use primitives::VaultCurrencyPair;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::crypto::UncheckedInto;

use interbtc_rpc::jsonrpc_core::serde_json::{map::Map, Value};
use sc_service::ChainType;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> AuraId {
    get_from_seed::<AuraId>(s)
}

fn get_account_id_from_string(account_id: &str) -> AccountId {
    AccountId::from_str(account_id).expect("account id is not valid")
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn get_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    CurrencyId::get_info().iter().for_each(|(symbol_name, decimals)| {
        token_symbol.push(symbol_name.to_string());
        token_decimals.push(*decimals);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties
}

pub fn local_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    "Bob".as_bytes().to_vec(),
                )],
                0,
                false,
            )
        },
        vec![],
        None,
        None,
        Some(get_properties()),
        None,
    )
}

pub fn beta_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "beta_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                vec![
                    // 5DJ3wbdicFSFFudXndYBuvZKjucTsyxtJX5WPzQM8HysSkFY
                    hex!["dce82040dc0a90843897aee1cc1a96c205fe7c1165b8f46635c2547ed15a3013"].unchecked_into(),
                    // 5HW7ApFamN6ovtDkFyj67tRLRhp8B2kVNjureRUWWYhkTg9j
                    hex!["5b4651cf045ddf55f0df7bfbb9bb4c45bbeb3c536c6ce4a98275781b8f0f0754"].unchecked_into(),
                    // 5FNbq8zGPZtinsfgyD4w2G3BMh75H3r2Qg3uKudTZkJtRru6
                    hex!["8de3db7b51864804d2dd5c5905d571aa34d7161537d5a0045755b72d1ac2062e"].unchecked_into(),
                ],
                vec![
                    // root key
                    get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                    // faucet
                    get_account_id_from_string("5FHy3cvyToZ4ConPXhi43rycAcGYw2R2a8cCjfVMfyuS1Ywg"),
                    // vaults
                    get_account_id_from_string("5F7Q9FqnGwJmjLtsFGymHZXPEx2dWRVE7NW4Sw2jzEhUB5WQ"),
                    get_account_id_from_string("5CJncqjWDkYv4P6nccZHGh8JVoEBXvharMqVpkpJedoYNu4A"),
                    get_account_id_from_string("5GpnEWKTWv7xiQtDFi9Rku7DrvgHj4oqMDev4qBQhfwQE8nx"),
                    get_account_id_from_string("5DttG269R1NTBDWcghYxa9NmV2wHxXpTe4U8pu4jK3LCE9zi"),
                    // relayers
                    get_account_id_from_string("5DNzULM1UJXDM7NUgDL4i8Hrhe9e3vZkB3ByM1eEXMGAs4Bv"),
                    get_account_id_from_string("5GEXRnnv8Qz9rEwMs4TfvHme48HQvVTEDHJECCvKPzFB4pFZ"),
                    // oracles
                    get_account_id_from_string("5H8zjSWfzMn86d1meeNrZJDj3QZSvRjKxpTfuVaZ46QJZ4qs"),
                    get_account_id_from_string("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj"),
                ],
                vec![
                    (
                        get_account_id_from_string("5H8zjSWfzMn86d1meeNrZJDj3QZSvRjKxpTfuVaZ46QJZ4qs"),
                        "Interlay".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_string("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj"),
                        "Band".as_bytes().to_vec(),
                    ),
                ],
                1,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        None,
    )
}

pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![authority_keys_from_seed("Alice")],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        "Alice".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        "Bob".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Charlie"),
                        "Charlie".as_bytes().to_vec(),
                    ),
                ],
                1,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        None,
    )
}

fn default_pair(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: GetWrappedCurrencyId::get(),
    }
}

fn expected_transaction_size() -> u32 {
    virtual_transaction_size(
        TransactionInputMetadata {
            count: 2,
            script_type: InputType::P2WPKHv0,
        },
        TransactionOutputMetadata {
            num_op_return: 1,
            num_p2pkh: 2,
            num_p2sh: 0,
            num_p2wpkh: 0,
        },
    )
}

fn testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<AuraId>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    bitcoin_confirmations: u32,
    start_shutdown: bool,
) -> GenesisConfig {
    GenesisConfig {
        system: SystemConfig {
            code: WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        aura: AuraConfig {
            authorities: initial_authorities,
        },
        security: SecurityConfig {
            initial_status: if start_shutdown {
                StatusCode::Shutdown
            } else {
                StatusCode::Error
            },
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: TokensConfig {
            balances: endowed_accounts
                .iter()
                .flat_map(|k| {
                    vec![
                        (k.clone(), DOT, 1 << 60),
                        (k.clone(), INTR, 1 << 60),
                        (k.clone(), KSM, 1 << 60),
                        (k.clone(), KINT, 1 << 60),
                    ]
                })
                .collect(),
        },
        oracle: OracleConfig {
            authorized_oracles,
            max_delay: 3600000, // one hour
        },
        btc_relay: BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        },
        issue: IssueConfig {
            issue_period: DAYS,
            issue_btc_dust_value: 1000,
        },
        redeem: RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: DAYS,
            redeem_btc_dust_value: 1000,
        },
        replace: ReplaceConfig {
            replace_period: DAYS,
            replace_btc_dust_value: 1000,
        },
        vault_registry: VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::DOT, 0), (CurrencyId::KSM, 0)],
            punishment_delay: DAYS,
            secure_collateral_threshold: vec![
                (
                    default_pair(CurrencyId::DOT),
                    FixedU128::checked_from_rational(150, 100).unwrap(),
                ),
                (
                    default_pair(CurrencyId::KSM),
                    FixedU128::checked_from_rational(150, 100).unwrap(),
                ),
            ], /* 150% */
            premium_redeem_threshold: vec![
                (
                    default_pair(CurrencyId::DOT),
                    FixedU128::checked_from_rational(135, 100).unwrap(),
                ),
                (
                    default_pair(CurrencyId::KSM),
                    FixedU128::checked_from_rational(135, 100).unwrap(),
                ),
            ], /* 135% */
            liquidation_collateral_threshold: vec![
                (
                    default_pair(CurrencyId::DOT),
                    FixedU128::checked_from_rational(110, 100).unwrap(),
                ),
                (
                    default_pair(CurrencyId::KSM),
                    FixedU128::checked_from_rational(110, 100).unwrap(),
                ),
            ], /* 110% */
            system_collateral_ceiling: vec![
                (default_pair(CurrencyId::DOT), 1000 * CurrencyId::DOT.one()),
                (default_pair(CurrencyId::KSM), 1000 * CurrencyId::KSM.one()),
            ],
        },
        fee: FeeConfig {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),  // 5%
            theft_fee_max: 10000000,                                       // 0.1 BTC
        },
        refund: RefundConfig {
            refund_btc_dust_value: 1000,
            refund_transaction_size: expected_transaction_size(),
        },
        nomination: NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: TechnicalCommitteeConfig {
            members: vec![get_account_id_from_seed::<sr25519::Public>("Alice")],
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
    }
}
