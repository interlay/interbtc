use btc_parachain_runtime::{
    AccountId, BTCRelayConfig, BackingConfig, BlockNumber, ExchangeRateOracleConfig, FeeConfig, GenesisConfig,
    IssueConfig, IssuingConfig, NominationConfig, RedeemConfig, RefundConfig, ReplaceConfig, Signature, SlaConfig,
    SudoConfig, SystemConfig, VaultRegistryConfig, DAYS, HOURS, MILLISECS_PER_BLOCK, TARGET_SPACING, WASM_BINARY,
};

use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
const BITCOIN_SPACING_MS: u32 = TARGET_SPACING * 1000;
const BLOCK_SPACING: BlockNumber = BITCOIN_SPACING_MS / MILLISECS_PER_BLOCK as BlockNumber;

#[cfg(feature = "aura-grandpa")]
use {btc_parachain_runtime::GrandpaConfig, sp_finality_grandpa::AuthorityId as GrandpaId};

#[cfg(feature = "cumulus-polkadot")]
use {
    btc_parachain_runtime::ParachainInfoConfig,
    cumulus_primitives::ParaId,
    sc_chain_spec::{ChainSpecExtension, ChainSpecGroup},
    serde::{Deserialize, Serialize},
};

#[cfg(any(feature = "aura-grandpa", feature = "cumulus-polkadot"))]
use {
    btc_parachain_runtime::AuraConfig, hex_literal::hex, sp_consensus_aura::sr25519::AuthorityId as AuraId,
    sp_core::crypto::UncheckedInto,
};

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::account;

use btc_parachain_rpc::jsonrpc_core::serde_json::{self, json};
use sc_service::ChainType;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec` for the normal parachain runtime.
#[cfg(feature = "cumulus-polkadot")]
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

#[cfg(feature = "aura-grandpa")]
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate an Aura authority key.
#[cfg(feature = "aura-grandpa")]
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

fn get_account_id_from_string(account_id: &str) -> AccountId {
    AccountId::from_str(account_id).expect("account id is not valid")
}

/// The extensions for the [`ChainSpec`].
#[cfg(feature = "cumulus-polkadot")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecExtension, ChainSpecGroup)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

#[cfg(feature = "cumulus-polkadot")]
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

pub fn local_config(#[cfg(feature = "cumulus-polkadot")] id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Maintainer"),
                #[cfg(any(feature = "aura-grandpa", feature = "cumulus-polkadot"))]
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
                #[cfg(feature = "cumulus-polkadot")]
                id,
                0,
            )
        },
        vec![],
        None,
        None,
        Some(
            serde_json::from_value(json!({
                "ss58Format": 42,
                "tokenDecimals": [10, 8],
                "tokenSymbol": ["DOT", "PolkaBTC"]
            }))
            .unwrap(),
        ),
        #[cfg(feature = "cumulus-polkadot")]
        Extensions {
            relay_chain: "local".into(),
            para_id: id.into(),
        },
        #[cfg(feature = "aura-grandpa")]
        None,
    )
}

#[cfg(feature = "cumulus-polkadot")]
pub fn rococo_testnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC",
        "rococo_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                get_account_id_from_string("5FqYNDWeJ9bwa3NhEryxscBELAMj54yrKqGaYNR9CjLZFYLB"),
                vec![
                    // 5DJ3wbdicFSFFudXndYBuvZKjucTsyxtJX5WPzQM8HysSkFY
                    hex!["366a092a27b4b28199a588b0155a2c9f3f0513d92481de4ee2138273926fa91c"].unchecked_into(),
                ],
                vec![
                    get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                    get_account_id_from_string("5DNzULM1UJXDM7NUgDL4i8Hrhe9e3vZkB3ByM1eEXMGAs4Bv"),
                    get_account_id_from_string("5F7Q9FqnGwJmjLtsFGymHZXPEx2dWRVE7NW4Sw2jzEhUB5WQ"),
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
                id,
                1,
            )
        },
        Vec::new(),
        None,
        None,
        Some(
            serde_json::from_value(json!({
                "ss58Format": 42,
                "tokenDecimals": [10, 8],
                "tokenSymbol": ["DOT", "PolkaBTC"]
            }))
            .unwrap(),
        ),
        Extensions {
            relay_chain: "staging".into(),
            para_id: id.into(),
        },
    )
}

#[cfg(feature = "aura-grandpa")]
pub fn beta_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC",
        "beta_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                get_account_id_from_string("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt"),
                get_account_id_from_string("5FqYNDWeJ9bwa3NhEryxscBELAMj54yrKqGaYNR9CjLZFYLB"),
                vec![
                    (
                        // 5DJ3wbdicFSFFudXndYBuvZKjucTsyxtJX5WPzQM8HysSkFY
                        hex!["366a092a27b4b28199a588b0155a2c9f3f0513d92481de4ee2138273926fa91c"].unchecked_into(),
                        hex!["dce82040dc0a90843897aee1cc1a96c205fe7c1165b8f46635c2547ed15a3013"].unchecked_into(),
                    ),
                    (
                        // 5HW7ApFamN6ovtDkFyj67tRLRhp8B2kVNjureRUWWYhkTg9j
                        hex!["f08cc7cf45f88e6dbe312a63f6ce639061834b4208415b235f77a67b51435f63"].unchecked_into(),
                        hex!["5b4651cf045ddf55f0df7bfbb9bb4c45bbeb3c536c6ce4a98275781b8f0f0754"].unchecked_into(),
                    ),
                    (
                        // 5FNbq8zGPZtinsfgyD4w2G3BMh75H3r2Qg3uKudTZkJtRru6
                        hex!["925ad4bdf35945bea91baeb5419a7ffa07002c6a85ba334adfa7cb5b05623c1b"].unchecked_into(),
                        hex!["8de3db7b51864804d2dd5c5905d571aa34d7161537d5a0045755b72d1ac2062e"].unchecked_into(),
                    ),
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
            )
        },
        Vec::new(),
        None,
        None,
        Some(
            serde_json::from_value(json!({
                "ss58Format": 42,
                "tokenDecimals": [10, 8],
                "tokenSymbol": ["DOT", "PolkaBTC"]
            }))
            .unwrap(),
        ),
        None,
    )
}

pub fn development_config(#[cfg(feature = "cumulus-polkadot")] id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Maintainer"),
                #[cfg(feature = "aura-grandpa")]
                vec![authority_keys_from_seed("Alice")],
                #[cfg(feature = "cumulus-polkadot")]
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
                #[cfg(feature = "cumulus-polkadot")]
                id,
                1,
            )
        },
        Vec::new(),
        None,
        None,
        Some(
            serde_json::from_value(json!({
                "ss58Format": 42,
                "tokenDecimals": [10, 8],
                "tokenSymbol": ["DOT", "PolkaBTC"]
            }))
            .unwrap(),
        ),
        #[cfg(feature = "cumulus-polkadot")]
        Extensions {
            relay_chain: "dev".into(),
            para_id: id.into(),
        },
        #[cfg(feature = "aura-grandpa")]
        None,
    )
}

fn testnet_genesis(
    root_key: AccountId,
    maintainer: AccountId,
    #[cfg(feature = "aura-grandpa")] initial_authorities: Vec<(AuraId, GrandpaId)>,
    #[cfg(feature = "cumulus-polkadot")] initial_authorities: Vec<AuraId>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    #[cfg(feature = "cumulus-polkadot")] id: ParaId,
    bitcoin_confirmations: u32,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: SystemConfig {
            code: WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        #[cfg(feature = "aura-grandpa")]
        pallet_aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        #[cfg(feature = "aura-grandpa")]
        pallet_grandpa: GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
        },
        #[cfg(feature = "cumulus-polkadot")]
        pallet_aura: AuraConfig {
            authorities: initial_authorities,
        },
        #[cfg(feature = "cumulus-polkadot")]
        cumulus_pallet_aura_ext: Default::default(),
        #[cfg(feature = "cumulus-polkadot")]
        parachain_info: ParachainInfoConfig { parachain_id: id },
        pallet_sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        pallet_balances_Instance1: BackingConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
        },
        pallet_balances_Instance2: IssuingConfig { balances: vec![] },
        exchange_rate_oracle: ExchangeRateOracleConfig {
            authorized_oracles,
            max_delay: 3600000, // one hour
        },
        btc_relay: BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
            disable_op_return_check: false,
        },
        issue: IssueConfig { issue_period: DAYS },
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
            minimum_collateral_vault: 0,
            punishment_delay: DAYS,
            secure_collateral_threshold: FixedU128::checked_from_rational(150, 100).unwrap(), // 150%
            premium_redeem_threshold: FixedU128::checked_from_rational(135, 100).unwrap(),    // 135%
            liquidation_collateral_threshold: FixedU128::checked_from_rational(110, 100).unwrap(), // 110%
        },
        fee: FeeConfig {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            maintainer_account_id: maintainer,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            nomination_rewards: FixedU128::checked_from_rational(0, 100).unwrap(),
        },
        sla: SlaConfig {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(-100),
            vault_execute_issue_max_sla_change: FixedI128::from(4),
            vault_deposit_max_sla_change: FixedI128::from(4),
            vault_withdraw_max_sla_change: FixedI128::from(-4),
            vault_submit_issue_proof: FixedI128::from(1),
            vault_refund: FixedI128::from(1),
            relayer_target_sla: FixedI128::from(100),
            relayer_block_submission: FixedI128::from(1),
            relayer_duplicate_block_submission: FixedI128::from(1),
            relayer_correct_theft_report: FixedI128::from(1),
        },
        refund: RefundConfig {
            refund_btc_dust_value: 1000,
        },
        nomination: NominationConfig {
            is_nomination_enabled: false,
            get_operator_unbonding_period: 24 * HOURS,
            get_max_nomination_ratio: FixedU128::checked_from_rational(50, 100).unwrap(), // 50%
            get_max_nominators_per_operator: 100,                                         // 50%
            get_nominator_unbonding_period: 12 * HOURS,
        },
    }
}
