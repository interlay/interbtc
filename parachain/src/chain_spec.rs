use btc_parachain_runtime::{
    AccountId, BTCRelayConfig, DOTConfig, ExchangeRateOracleConfig, FeeConfig, GenesisConfig,
    IssueConfig, ParachainInfoConfig, PolkaBTCConfig, RedeemConfig, RefundConfig, ReplaceConfig,
    Signature, SlaConfig, StakedRelayersConfig, SudoConfig, SystemConfig, VaultRegistryConfig,
    DAYS, MINUTES, WASM_BINARY,
};

#[cfg(feature = "standalone")]
use {
    btc_parachain_runtime::{AuraConfig, GrandpaConfig},
    sp_consensus_aura::sr25519::AuthorityId as AuraId,
    sp_finality_grandpa::AuthorityId as GrandpaId,
};

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::account;

use btc_parachain_rpc::jsonrpc_core::serde_json::{self, json};
use cumulus_primitives::ParaId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
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

/// Generate an Aura authority key.
#[cfg(feature = "standalone")]
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

#[cfg(not(feature = "standalone"))]
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

pub fn local_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC Local",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("LiquidationVault"),
                get_account_id_from_seed::<sr25519::Public>("FeePool"),
                get_account_id_from_seed::<sr25519::Public>("Maintainer"),
                #[cfg(feature = "standalone")]
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
                id,
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
        Extensions {
            relay_chain: "local".into(),
            para_id: id.into(),
        },
    )
}

pub fn staging_testnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC Staging",
        "staging_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                AccountId::from_str("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt").unwrap(),
                AccountId::from_str("5CcXK1yKz4o68AJT3yBWjJPPXKDFvEFAi1L1Gkisy7n6MbGC").unwrap(),
                AccountId::from_str("5GqMEqFQMfr2FEUBQ8yzh7NTGZUQQigfVELHnXXsUFve7TMN").unwrap(),
                AccountId::from_str("5FqYNDWeJ9bwa3NhEryxscBELAMj54yrKqGaYNR9CjLZFYLB").unwrap(),
                #[cfg(feature = "standalone")]
                vec![],
                vec![
                    AccountId::from_str("5HeVGqvfpabwFqzV1DhiQmjaLQiFcTSmq2sH6f7atsXkgvtt")
                        .unwrap(),
                ],
                vec![
                    (
                        AccountId::from_str("5H8zjSWfzMn86d1meeNrZJDj3QZSvRjKxpTfuVaZ46QJZ4qs")
                            .unwrap(),
                        "Interlay".as_bytes().to_vec(),
                    ),
                    (
                        AccountId::from_str("5FPBT2BVVaLveuvznZ9A1TUtDcbxK5yvvGcMTJxgFmhcWGwj")
                            .unwrap(),
                        "Band".as_bytes().to_vec(),
                    ),
                ],
                id,
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

pub fn development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC Dev",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("LiquidationVault"),
                get_account_id_from_seed::<sr25519::Public>("FeePool"),
                get_account_id_from_seed::<sr25519::Public>("Maintainer"),
                #[cfg(feature = "standalone")]
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
                    #[cfg(feature = "runtime-benchmarks")]
                    account("Origin", 0, 0),
                    #[cfg(feature = "runtime-benchmarks")]
                    account("Vault", 0, 0),
                ],
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    "Bob".as_bytes().to_vec(),
                )],
                id,
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
            relay_chain: "dev".into(),
            para_id: id.into(),
        },
    )
}

fn testnet_genesis(
    root_key: AccountId,
    liquidation_vault: AccountId,
    fee_pool: AccountId,
    maintainer: AccountId,
    #[cfg(feature = "standalone")] initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        }),
        #[cfg(feature = "standalone")]
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        #[cfg(feature = "standalone")]
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        parachain_info: Some(ParachainInfoConfig { parachain_id: id }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        }),
        pallet_balances_Instance1: Some(DOTConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        }),
        pallet_balances_Instance2: Some(PolkaBTCConfig { balances: vec![] }),
        staked_relayers: Some(StakedRelayersConfig {
            #[cfg(feature = "runtime-benchmarks")]
            gov_id: account("Origin", 0, 0),
            #[cfg(not(feature = "runtime-benchmarks"))]
            gov_id: root_key,
            maturity_period: 10 * MINUTES,
        }),
        exchange_rate_oracle: Some(ExchangeRateOracleConfig {
            authorized_oracles,
            max_delay: 3600000, // one hour
        }),
        btc_relay: Some(BTCRelayConfig {
            bitcoin_confirmations: 0,
            parachain_confirmations: 0,
            disable_difficulty_check: true,
            disable_inclusion_check: false,
            disable_op_return_check: false,
        }),
        issue: Some(IssueConfig { issue_period: DAYS }),
        redeem: Some(RedeemConfig {
            redeem_period: DAYS,
            redeem_btc_dust_value: 1000,
        }),
        replace: Some(ReplaceConfig {
            replace_period: DAYS,
            replace_btc_dust_value: 1000,
        }),
        vault_registry: Some(VaultRegistryConfig {
            minimum_collateral_vault: 0,
            punishment_delay: DAYS,
            secure_collateral_threshold: FixedU128::checked_from_rational(150, 100).unwrap(), // 150%
            premium_redeem_threshold: FixedU128::checked_from_rational(135, 100).unwrap(), // 135%
            auction_collateral_threshold: FixedU128::checked_from_rational(120, 100).unwrap(), // 120%
            liquidation_collateral_threshold: FixedU128::checked_from_rational(110, 100).unwrap(), // 110%
            liquidation_vault_account_id: liquidation_vault,
        }),
        fee: Some(FeeConfig {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(),                // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(),   // 10%
            fee_pool_account_id: fee_pool,
            maintainer_account_id: maintainer,
            epoch_period: 5,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(),
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            collator_rewards: FixedU128::checked_from_integer(0).unwrap(),
        }),
        sla: Some(SlaConfig {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(0),
            vault_executed_issue_max_sla_change: FixedI128::from(0),
            vault_submitted_issue_proof: FixedI128::from(0),
            vault_refunded: FixedI128::from(1),
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
        }),
        refund: Some(RefundConfig {
            refund_btc_dust_value: 1000,
        }),
    }
}
