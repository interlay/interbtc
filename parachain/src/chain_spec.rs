use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use interbtc_runtime::{
    AccountId, AuraConfig, BTCRelayConfig, CurrencyId, FeeConfig, GenesisConfig, IssueConfig, NominationConfig,
    OracleConfig, ParachainInfoConfig, RedeemConfig, RefundConfig, ReplaceConfig, Signature, SudoConfig, SystemConfig,
    TokensConfig, VaultRegistryConfig, BITCOIN_BLOCK_SPACING, DAYS, KSM, WASM_BINARY,
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::crypto::UncheckedInto;

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::account;

use interbtc_rpc::jsonrpc_core::serde_json::{map::Map, Value};
use sc_service::ChainType;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

fn get_account_id_from_string(account_id: &str) -> AccountId {
    AccountId::from_str(account_id).expect("account id is not valid")
}

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

pub fn local_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_from_seed::<AuraId>("Alice")],
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
                id,
                0,
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

pub fn development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_from_seed::<AuraId>("Alice")],
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
                    #[cfg(feature = "runtime-benchmarks")]
                    account("Origin", 0, 0),
                    #[cfg(feature = "runtime-benchmarks")]
                    account("Vault", 0, 0),
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
                id,
                1,
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

pub fn rococo_testnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "rococo_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                vec![
                    // 5DJ3wbdicFSFFudXndYBuvZKjucTsyxtJX5WPzQM8HysSkFY
                    hex!["366a092a27b4b28199a588b0155a2c9f3f0513d92481de4ee2138273926fa91c"].unchecked_into(),
                    // 5HW7ApFamN6ovtDkFyj67tRLRhp8B2kVNjureRUWWYhkTg9j
                    hex!["f08cc7cf45f88e6dbe312a63f6ce639061834b4208415b235f77a67b51435f63"].unchecked_into(),
                    // 5FNbq8zGPZtinsfgyD4w2G3BMh75H3r2Qg3uKudTZkJtRru6
                    hex!["925ad4bdf35945bea91baeb5419a7ffa07002c6a85ba334adfa7cb5b05623c1b"].unchecked_into(),
                ],
                vec![],
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
                1,
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

pub fn rococo_local_testnet_config(id: ParaId) -> ChainSpec {
    development_config(id)
}

pub fn westend_testnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "westend_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5H5wcrRsz7wjX6LNhh4ZeSKWGmSJjsEqgge6QbGk6n53QX7j"),
                vec![
                    // 5H75GkhA6TnyCW7fM4H8LyoTqmPJWf3JuZZPFR9Bpv26LGHA
                    hex!["defbbf8f70964f6a4952bc168b6c1489b502e05d6b5ef57f8767589cf3813705"].unchecked_into(),
                    // 5GdqW1xV8bpcJM1AVPWCdqrnUYJ9UQro1bWuPvmY2hoaQxWp
                    hex!["ca35c3927b934b111acadfcf98e9b50846e7596beb7a355df1ab50b1c48e3017"].unchecked_into(),
                    // 5CdNwrXY3mFMMTiVsxbNTmg3MMDXcyErhxkdLx7yUqhXKopt
                    hex!["18eb708be158d0059d005da4188976caaa1aa24c8450ed3f4ad17e7a6a0cb85e"].unchecked_into(),
                    // 5EcCjUzqBBpmf7E3gXFX3jFosY22yEL7iXYVFWZExPgF6YwD
                    hex!["707e47b5a236b10cc8dcb52698ab41ee4e3a23063d999e81af5781b1e03f7048"].unchecked_into(),
                    // 5DoegnR7GDewmsswNgGuhZZQ8KxTPeVNd9MF1ezhSKdztEPD
                    hex!["4cfd1cfc3af74ef3189d6b92734eabae763ae86f1f6dfdf91b04e5d43a369175"].unchecked_into(),
                    // 5GRKDYVdQ6AAS6xEQ85LzmxNwgP1u2YM81WAUjiD6YLbe69B
                    hex!["c0a8dfbd58ed57758594841d3cc8e6a34c97ef75380fe3c3925b1dbddf988f6f"].unchecked_into(),
                    // 5FKbkKSb9jft3KpZSJviG8EFmdcLanpr4mBj56NpvQ6uL3bQ
                    hex!["9010d0a8a099505887e772417734ee94dc767b8ec00f42086dac9742f3b6e037"].unchecked_into(),
                    // 5H8WaYthvpavtRmYkVkSBzCjbhHqYp9hnNhJXDDnVr2GJt6v
                    hex!["e0142f20c1ad92ac9467a4e01ecc0572c45704a730b5337b23b68cb7279a6b49"].unchecked_into(),
                    // 5ECnot77onJJrSGbKtvTaB7L9zKXB9VrS97vSqBx5bcy15G9
                    hex!["5ea31992c7fb94695c225010b47daf82dd9a1db4751362ae30f299d8164b6c3e"].unchecked_into(),
                    // 5HNEdfdAvhvAA67pqPgoctiUTCraXkscSv5wYQbUwrKNmpQq
                    hex!["ea8bf097557a70b3c8beed5a95ecc127534f6fe00709c20352dcfb8bd073e240"].unchecked_into(),
                ],
                vec![],
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
                6,
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

fn testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<AuraId>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
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
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        parachain_info: ParachainInfoConfig { parachain_id: id },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: TokensConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, KSM, 1 << 60)).collect(),
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
            disable_op_return_check: false,
        },
        issue: IssueConfig {
            issue_period: DAYS,
            issue_btc_dust_value: 1000,
        },
        redeem: RedeemConfig {
            redeem_transaction_size: virtual_transaction_size(
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
            ),
            redeem_period: DAYS,
            redeem_btc_dust_value: 1000,
        },
        replace: ReplaceConfig {
            replace_period: DAYS,
            replace_btc_dust_value: 1000,
        },
        vault_registry: VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: DAYS,
            secure_collateral_threshold: vec![(CurrencyId::KSM, FixedU128::checked_from_rational(150, 100).unwrap())], /* 150% */
            premium_redeem_threshold: vec![(CurrencyId::KSM, FixedU128::checked_from_rational(135, 100).unwrap())], /* 135% */
            liquidation_collateral_threshold: vec![(
                CurrencyId::KSM,
                FixedU128::checked_from_rational(110, 100).unwrap(),
            )], /* 110% */
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
        },
        nomination: NominationConfig {
            is_nomination_enabled: false,
        },
        general_council: Default::default(),
        technical_committee: Default::default(),
        treasury: Default::default(),
        technical_membership: Default::default(),
        democracy: Default::default(),
    }
}
