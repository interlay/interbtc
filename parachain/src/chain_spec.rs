use btc_parachain_runtime::{
    AccountId, BTCRelayConfig, DOTConfig, ExchangeRateOracleConfig, FeeConfig, GenesisConfig,
    IssueConfig, ParachainInfoConfig, PolkaBTCConfig, RedeemConfig, RefundConfig, ReplaceConfig,
    Signature, SlaConfig, StakedRelayersConfig, SudoConfig, SystemConfig, VaultRegistryConfig,
    DAYS, MINUTES, WASM_BINARY,
};

use cumulus_primitives::ParaId;
use hex_literal::hex;
use jsonrpc_core::serde_json::{self, json};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::account;

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

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Option<&Self> {
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

pub fn get_chain_spec(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "PolkaBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
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
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn staging_testnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Staging Testnet",
        "staging_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                hex!["9ed7705e3c7da027ba0583a22a3212042f7e715d3c168ba14f1424e2bc111d00"].into(),
                vec![
                    hex!["9ed7705e3c7da027ba0583a22a3212042f7e715d3c168ba14f1424e2bc111d00"].into(),
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
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

fn testnet_genesis(
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),
        parachain_info: Some(ParachainInfoConfig { parachain_id: id }),
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
            gov_id: get_account_id_from_seed::<sr25519::Public>("Alice"),
            maturity_period: 10 * MINUTES,
        }),
        exchange_rate_oracle: Some(ExchangeRateOracleConfig {
            authorized_oracles: vec![(
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                "Bob".as_bytes().to_vec(),
            )],
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
            liquidation_vault_account_id: get_account_id_from_seed::<sr25519::Public>(
                "LiquidationVault",
            ),
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
            fee_pool_account_id: get_account_id_from_seed::<sr25519::Public>("FeePool"),
            maintainer_account_id: get_account_id_from_seed::<sr25519::Public>("Maintainer"),
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
