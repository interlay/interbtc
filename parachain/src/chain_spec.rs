use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use interbtc_rpc::jsonrpc_core::serde_json::{map::Map, Value};
use primitives::{AccountId, Balance, BlockNumber, CurrencyId, Signature, VaultCurrencyPair, KINT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify, Zero};
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// Specialized `ChainSpec` for the interlay parachain runtime.
pub type InterlayChainSpec = sc_service::GenericChainSpec<interlay_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the kintsugi parachain runtime.
pub type KintsugiChainSpec = sc_service::GenericChainSpec<kintsugi_runtime::GenesisConfig, Extensions>;

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecExtension, ChainSpecGroup)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

type AccountPublic = <Signature as Verify>::Signer;

fn get_from_string<TPublic: Public>(src: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(src, None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    get_from_string::<TPublic>(&format!("//{}", seed))
}

fn get_account_id_from_string(account_id: &str) -> AccountId {
    AccountId::from_str(account_id).expect("account id is not valid")
}

/// Helper function to generate an account ID from seed
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AuraId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<AuraId>(seed),
    )
}

fn get_authority_keys_from_public_key(src: [u8; 32]) -> (AccountId, AuraId) {
    (src.clone().into(), src.unchecked_into())
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
fn get_kintsugi_session_keys(keys: AuraId) -> kintsugi_runtime::SessionKeys {
    kintsugi_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
fn get_interlay_session_keys(keys: AuraId) -> interlay_runtime::SessionKeys {
    interlay_runtime::SessionKeys { aura: keys }
}

const DEFAULT_MAX_DELAY_MS: u32 = 60 * 60 * 1000; // one hour
const DEFAULT_DUST_VALUE: Balance = 1000;
const DEFAULT_BITCOIN_CONFIRMATIONS: u32 = 1;
const SECURE_BITCOIN_CONFIRMATIONS: u32 = 6;

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

pub fn local_config(id: ParaId) -> KintsugiChainSpec {
    KintsugiChainSpec::from_genesis(
        "interBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_authority_keys_from_seed("Alice")],
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    "Bob".as_bytes().to_vec(),
                )],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        vec![],
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "local".into(),
            para_id: id.into(),
        },
    )
}

pub fn development_config(id: ParaId) -> KintsugiChainSpec {
    KintsugiChainSpec::from_genesis(
        "interBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_authority_keys_from_seed("Alice")],
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
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn rococo_testnet_config(id: ParaId) -> KintsugiChainSpec {
    KintsugiChainSpec::from_genesis(
        "interBTC",
        "rococo_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                vec![
                    // 5DJ3wbdicFSFFudXndYBuvZKjucTsyxtJX5WPzQM8HysSkFY
                    get_authority_keys_from_public_key(hex!(
                        "366a092a27b4b28199a588b0155a2c9f3f0513d92481de4ee2138273926fa91c"
                    )),
                    // 5HW7ApFamN6ovtDkFyj67tRLRhp8B2kVNjureRUWWYhkTg9j
                    get_authority_keys_from_public_key(hex!(
                        "f08cc7cf45f88e6dbe312a63f6ce639061834b4208415b235f77a67b51435f63"
                    )),
                    // 5FNbq8zGPZtinsfgyD4w2G3BMh75H3r2Qg3uKudTZkJtRru6
                    get_authority_keys_from_public_key(hex!(
                        "925ad4bdf35945bea91baeb5419a7ffa07002c6a85ba334adfa7cb5b05623c1b"
                    )),
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
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "rococo".into(),
            para_id: id.into(),
        },
    )
}

pub fn rococo_local_testnet_config(id: ParaId) -> KintsugiChainSpec {
    development_config(id)
}

pub fn westend_testnet_config(id: ParaId) -> KintsugiChainSpec {
    KintsugiChainSpec::from_genesis(
        "interBTC",
        "westend_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5DUupBJSyBDcqQudgPR4gttFie3cLPRw3HwaUfq9H2D2mKiA"),
                vec![
                    // 5H75GkhA6TnyCW7fM4H8LyoTqmPJWf3JuZZPFR9Bpv26LGHA
                    get_authority_keys_from_public_key(hex!(
                        "defbbf8f70964f6a4952bc168b6c1489b502e05d6b5ef57f8767589cf3813705"
                    )),
                    // 5GdqW1xV8bpcJM1AVPWCdqrnUYJ9UQro1bWuPvmY2hoaQxWp
                    get_authority_keys_from_public_key(hex!(
                        "ca35c3927b934b111acadfcf98e9b50846e7596beb7a355df1ab50b1c48e3017"
                    )),
                    // 5CdNwrXY3mFMMTiVsxbNTmg3MMDXcyErhxkdLx7yUqhXKopt
                    get_authority_keys_from_public_key(hex!(
                        "18eb708be158d0059d005da4188976caaa1aa24c8450ed3f4ad17e7a6a0cb85e"
                    )),
                    // 5EcCjUzqBBpmf7E3gXFX3jFosY22yEL7iXYVFWZExPgF6YwD
                    get_authority_keys_from_public_key(hex!(
                        "707e47b5a236b10cc8dcb52698ab41ee4e3a23063d999e81af5781b1e03f7048"
                    )),
                    // 5DoegnR7GDewmsswNgGuhZZQ8KxTPeVNd9MF1ezhSKdztEPD
                    get_authority_keys_from_public_key(hex!(
                        "4cfd1cfc3af74ef3189d6b92734eabae763ae86f1f6dfdf91b04e5d43a369175"
                    )),
                    // 5GRKDYVdQ6AAS6xEQ85LzmxNwgP1u2YM81WAUjiD6YLbe69B
                    get_authority_keys_from_public_key(hex!(
                        "c0a8dfbd58ed57758594841d3cc8e6a34c97ef75380fe3c3925b1dbddf988f6f"
                    )),
                    // 5FKbkKSb9jft3KpZSJviG8EFmdcLanpr4mBj56NpvQ6uL3bQ
                    get_authority_keys_from_public_key(hex!(
                        "9010d0a8a099505887e772417734ee94dc767b8ec00f42086dac9742f3b6e037"
                    )),
                    // 5H8WaYthvpavtRmYkVkSBzCjbhHqYp9hnNhJXDDnVr2GJt6v
                    get_authority_keys_from_public_key(hex!(
                        "e0142f20c1ad92ac9467a4e01ecc0572c45704a730b5337b23b68cb7279a6b49"
                    )),
                    // 5ECnot77onJJrSGbKtvTaB7L9zKXB9VrS97vSqBx5bcy15G9
                    get_authority_keys_from_public_key(hex!(
                        "5ea31992c7fb94695c225010b47daf82dd9a1db4751362ae30f299d8164b6c3e"
                    )),
                    // 5HNEdfdAvhvAA67pqPgoctiUTCraXkscSv5wYQbUwrKNmpQq
                    get_authority_keys_from_public_key(hex!(
                        "ea8bf097557a70b3c8beed5a95ecc127534f6fe00709c20352dcfb8bd073e240"
                    )),
                ],
                vec![
                    (
                        get_account_id_from_string("5EPKc1xDF2V337FwgpMozdcZKS1rgFjY3rTudEysMPK7paef"),
                        "Interlay".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_string("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj"),
                        "Band".as_bytes().to_vec(),
                    ),
                ],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "westend".into(),
            para_id: id.into(),
        },
    )
}

fn default_pair(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: kintsugi_runtime::GetWrappedCurrencyId::get(),
    }
}

fn testnet_genesis(
    root_key: AccountId,
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
    start_shutdown: bool,
) -> kintsugi_runtime::GenesisConfig {
    kintsugi_runtime::GenesisConfig {
        system: kintsugi_runtime::SystemConfig {
            code: kintsugi_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        collator_selection: kintsugi_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: kintsugi_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                     // account id
                        acc.clone(),                     // validator id
                        get_kintsugi_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        parachain_info: kintsugi_runtime::ParachainInfoConfig { parachain_id: id },
        security: kintsugi_runtime::SecurityConfig {
            initial_status: if start_shutdown {
                kintsugi_runtime::StatusCode::Shutdown
            } else {
                kintsugi_runtime::StatusCode::Error
            },
        },
        sudo: kintsugi_runtime::SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: kintsugi_runtime::TokensConfig { balances: vec![] },
        vesting: Default::default(),
        oracle: kintsugi_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: kintsugi_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(kintsugi_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        },
        issue: kintsugi_runtime::IssueConfig {
            issue_period: kintsugi_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: kintsugi_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: kintsugi_runtime::DAYS,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: kintsugi_runtime::ReplaceConfig {
            replace_period: kintsugi_runtime::DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: kintsugi_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: kintsugi_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair(CurrencyId::KSM), 1000 * CurrencyId::KSM.one())],
            secure_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
            premium_redeem_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(135, 100).unwrap(),
            )], /* 135% */
            liquidation_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(110, 100).unwrap(),
            )], /* 110% */
        },
        fee: kintsugi_runtime::FeeConfig {
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
        refund: kintsugi_runtime::RefundConfig {
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
            refund_transaction_size: expected_transaction_size(),
        },
        nomination: kintsugi_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        annuity: Default::default(),
        supply: kintsugi_runtime::SupplyConfig {
            initial_supply: kintsugi_runtime::token_distribution::INITIAL_ALLOCATION,
            start_height: kintsugi_runtime::YEARS * 5,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
    }
}

pub fn kintsugi_mainnet_config(id: ParaId) -> KintsugiChainSpec {
    KintsugiChainSpec::from_genesis(
        "Kintsugi",
        "kintsugi",
        ChainType::Live,
        move || {
            kintsugi_mainnet_genesis(
                get_account_id_from_string("5G49RwnYdfHywAfEpsPRhP47XuznQHpaPuSoSdt6S1kyi69g"),
                vec![
                    // 5DyzufhT1Ynxk9uxrWHjrVuap8oB4Zz7uYdquZHxFxvYBovd
                    get_authority_keys_from_public_key(hex!(
                        "54e1a41c9ba60ca45e911e8798ba9d81c22b04435b04816490ebddffe4dffc5c"
                    )),
                    // 5EvgAvVBQXtFFbcN74rYR2HE8RsWsEJHqPHhrGX427cnbvY2
                    get_authority_keys_from_public_key(hex!(
                        "7e951061df4d5b61b31a69d62233a5a3a2abdc3195902dd22bc062fadbf42e17"
                    )),
                    // 5Hp2yfUMoA5uJM6DQpDJAuCHdzvhzn57gurH1Cxp4cUTzciB
                    get_authority_keys_from_public_key(hex!(
                        "fe3915da55703833883c8e0dc9a81bc5ab5e3b4099b23d810cd5d78c6598395b"
                    )),
                    // 5FQzZEbc5CtF7gR1De449GtvDwpyVwWPZMqyq9yjJmxXKmgU
                    get_authority_keys_from_public_key(hex!(
                        "942dd2ded2896fa236c0f0df58dff88a04d7cf661a4676059d79dc54a271234a"
                    )),
                    // 5EqmSYibeeyypp2YGtJdkZxiNjLKpQLCMpW5J3hNgWBfT9Gw
                    get_authority_keys_from_public_key(hex!(
                        "7ad693485d4d67a2112881347a553009f0c1de3b26e662aa3863085f536d0537"
                    )),
                    // 5E1WeDF5L8xXLmMnLmJUCXo5xqLD6zzPP14T9vESydQmUA29
                    get_authority_keys_from_public_key(hex!(
                        "5608fa7874491c640d0420f5f44650a0b5b8b67411b2670b68440bb97e74ee1c"
                    )),
                    // 5D7eFVnyAhcbEJAPAVENqoCr44zTbztsiragiYjz1ExDePja
                    get_authority_keys_from_public_key(hex!(
                        "2e79d45517532bc4b6b3359be9ea2aa8b711a0a5362880cfb6651bcb87fe1b05"
                    )),
                    // 5FkCciu8zasoDoViTbAYpcHgitQgB5GHN64HWdXyy8kykXFK
                    get_authority_keys_from_public_key(hex!(
                        "a2d4159da7f458f8140899f443b480199c65e75ffb755ea9e097aa5b18352001"
                    )),
                    // 5H3E3GF1LUeyowgRx47n8AJsRCyzA4f2YNuTo4qEQy7fbbBo
                    get_authority_keys_from_public_key(hex!(
                        "dc0c47c6f8fd81190d4fcee4ab2074db5d83eaf301f2cd795ec9b39b8e753f66"
                    )),
                    // 5ERqgB3mYvotBFu6vVf7fdnTgxHJvVidBpQL8W4yrpFL25mo
                    get_authority_keys_from_public_key(hex!(
                        "6896f1128f9a92c68f14713f0cbeb67a402621d7c80257ea3b246fcca5aede17"
                    )),
                ],
                vec![
                    (
                        get_account_id_from_string("5DcrZv97CipkXni4aXcg98Nz9doT6nfs6t3THn7hhnRXTd6D"),
                        "Interlay".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_string("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj"),
                        "Band".as_bytes().to_vec(),
                    ),
                ],
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
                vec![],
                vec![],
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: id.into(),
        },
    )
}

fn kintsugi_mainnet_genesis(
    root_key: AccountId,
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
    initial_allocation: Vec<(AccountId, Balance)>,
    vesting_list: Vec<(
        AccountId,   // who
        BlockNumber, // start
        BlockNumber, // period
        u32,         // period_count
        Balance,     // per_period
    )>,
) -> kintsugi_runtime::GenesisConfig {
    kintsugi_runtime::GenesisConfig {
        system: kintsugi_runtime::SystemConfig {
            code: kintsugi_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        parachain_system: Default::default(),
        parachain_info: kintsugi_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: kintsugi_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: kintsugi_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                     // account id
                        acc.clone(),                     // validator id
                        get_kintsugi_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: kintsugi_runtime::SecurityConfig {
            initial_status: kintsugi_runtime::StatusCode::Shutdown,
        },
        sudo: kintsugi_runtime::SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: kintsugi_runtime::TokensConfig {
            balances: initial_allocation
                .iter()
                .map(|(who, amount)| (who.clone(), KINT, *amount))
                .collect(),
        },
        vesting: kintsugi_runtime::VestingConfig { vesting: vesting_list },
        oracle: kintsugi_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: kintsugi_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(kintsugi_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: false,
            disable_inclusion_check: false,
        },
        issue: kintsugi_runtime::IssueConfig {
            issue_period: kintsugi_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: kintsugi_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: kintsugi_runtime::DAYS,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: kintsugi_runtime::ReplaceConfig {
            replace_period: kintsugi_runtime::DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: kintsugi_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: kintsugi_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair(CurrencyId::KSM), 317 * CurrencyId::KSM.one())], /* 317 ksm, about 100k
                                                                                                            * USD at
                                                                                                            * time of writing */
            secure_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )], /* 260% */
            premium_redeem_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )], /* 200% */
            liquidation_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
        },
        fee: kintsugi_runtime::FeeConfig {
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
        refund: kintsugi_runtime::RefundConfig {
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
            refund_transaction_size: expected_transaction_size(),
        },
        nomination: kintsugi_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        annuity: Default::default(),
        supply: kintsugi_runtime::SupplyConfig {
            initial_supply: kintsugi_runtime::token_distribution::INITIAL_ALLOCATION,
            start_height: kintsugi_runtime::YEARS * 5,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
    }
}

pub fn interlay_mainnet_config(id: ParaId) -> InterlayChainSpec {
    InterlayChainSpec::from_genesis(
        "Interlay",
        "interlay",
        ChainType::Live,
        move || {
            interlay_mainnet_genesis(
                get_account_id_from_string("5E4kVWCtww5YmkWTR8Pf5q4apDbb1Ei5nZJ29e9DP2HgLJWn"),
                vec![
                    // 5CDEceADNMhAgHBCDnb7Ls1YZKgwe2z3qmcwNHTeAFr5dGrW (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "068181205488a5517460dd305c9ec781ddf6e68627609ec88cbb60d0b7647d0f"
                    ]),
                    // 5G6AgvRRkzFvs69SXY2Ah6PmjySswGFqHTgriqLohNMzfEsc (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "b20e80ecc31ce2ccb3487e7cc4447098417813cf7553f1f459662f782bbfd12a"
                    ]),
                    // 5EXCEev51P1KFkMQQdjT25KzMWMLG5EXw51uhaCQbDziPe8t (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "6cac613f09264c7397fa27dfc131d0c77a4dc8d5b5e22a22e3e1a6ac8e00d211"
                    ]),
                    // 5GH6mdEu56ku6ez26udZkaL9F5unbV7sUeJHnYbkLx4LTgiN (//authority/4)
                    get_authority_keys_from_public_key(hex![
                        "ba6502c812d5ece87390df7f955d50f1fc55adff99e4bc68fa7b58494bd0dc1e"
                    ]),
                    // 5H3X7DPUsnUUBqtRxCnSbrPX38jwsxg5pXcNyMabCf9QaU6i (//authority/5)
                    get_authority_keys_from_public_key(hex![
                        "dc45bc9ddeaacb1ffd04bfaf1366033f54640380a51a255448a639aa670d680c"
                    ]),
                    // 5Fy933qEzYeiN22fbWEU4RgJkvhVwXurPPZsrXstkoZFNcBS (//authority/6)
                    get_authority_keys_from_public_key(hex![
                        "acb238ad11721c943d8e43232efde998276179d7994aa2500b45d3adbe4ab90c"
                    ]),
                    // 5Ew8SA3y8jg4kfYAAatJ541EdZAmpyG8yCaZESJnE2nhsAE5 (//authority/7)
                    get_authority_keys_from_public_key(hex![
                        "7eed78d2af8350ddc6da7bafaeeac9df86f71ae0efcfd04e99a423b72003c007"
                    ]),
                    // 5EpntRydKc1AbGwPk7xt4aLnDoisQQ8dqY6zCYGFCxH9ex7M (//authority/8)
                    get_authority_keys_from_public_key(hex![
                        "7a1832d12c6ab761b9fbc7747d6a26601c42a68e2e3086cee64c7e84178d306d"
                    ]),
                    // 5Fjk4u3j4buQtf5YMU7Pj6AtSrvFaH5eGyKeUdQvyc41ipgY (//authority/9)
                    get_authority_keys_from_public_key(hex![
                        "a27ab6a94eb0d61f9e95adb45e68b5c71fd668070e664238bcbd51ca7515e168"
                    ]),
                ],
                vec![
                    (
                        get_account_id_from_string("5FyE5kCDSVtM1KmscBBa2Api8ZsF2DBT81QHf9RuS2NntUPw"),
                        "Interlay".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_string("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj"),
                        "Band".as_bytes().to_vec(),
                    ),
                ],
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
                vec![],
                vec![],
            )
        },
        Vec::new(),
        None,
        None,
        Some(get_properties()),
        Extensions {
            relay_chain: "polkadot".into(),
            para_id: id.into(),
        },
    )
}

fn interlay_mainnet_genesis(
    root_key: AccountId,
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
    initial_allocation: Vec<(AccountId, Balance)>,
    vesting_list: Vec<(
        AccountId,   // who
        BlockNumber, // start
        BlockNumber, // period
        u32,         // period_count
        Balance,     // per_period
    )>,
) -> interlay_runtime::GenesisConfig {
    interlay_runtime::GenesisConfig {
        system: interlay_runtime::SystemConfig {
            code: interlay_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        parachain_system: Default::default(),
        parachain_info: interlay_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: interlay_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: interlay_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                     // account id
                        acc.clone(),                     // validator id
                        get_interlay_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: interlay_runtime::SecurityConfig {
            initial_status: interlay_runtime::StatusCode::Shutdown,
        },
        sudo: interlay_runtime::SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: interlay_runtime::TokensConfig {
            balances: initial_allocation
                .iter()
                .map(|(who, amount)| (who.clone(), KINT, *amount))
                .collect(),
        },
        vesting: interlay_runtime::VestingConfig { vesting: vesting_list },
        oracle: interlay_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: interlay_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(interlay_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: false,
            disable_inclusion_check: false,
        },
        issue: interlay_runtime::IssueConfig {
            issue_period: interlay_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: interlay_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: interlay_runtime::DAYS,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: interlay_runtime::ReplaceConfig {
            replace_period: interlay_runtime::DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: interlay_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: interlay_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair(CurrencyId::KSM), 317 * CurrencyId::KSM.one())], /* 317 ksm, about 100k
                                                                                                            * USD at
                                                                                                            * time of writing */
            secure_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )], /* 260% */
            premium_redeem_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )], /* 200% */
            liquidation_collateral_threshold: vec![(
                default_pair(CurrencyId::KSM),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
        },
        fee: interlay_runtime::FeeConfig {
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
        refund: interlay_runtime::RefundConfig {
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
            refund_transaction_size: expected_transaction_size(),
        },
        nomination: interlay_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        annuity: Default::default(),
        supply: interlay_runtime::SupplyConfig {
            initial_supply: interlay_runtime::token_distribution::INITIAL_ALLOCATION,
            start_height: interlay_runtime::YEARS * 5,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
    }
}
