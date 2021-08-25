use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use interbtc_runtime::{
    AccountId, AuraConfig, BTCRelayConfig, CurrencyId, FeeConfig, GeneralCouncilConfig, GenesisConfig, IssueConfig,
    NominationConfig, OracleConfig, ParachainInfoConfig, RedeemConfig, RefundConfig, ReplaceConfig, Signature,
    SudoConfig, SystemConfig, TechnicalCommitteeConfig, TokensConfig, VaultRegistryConfig, BITCOIN_BLOCK_SPACING, DAYS,
    KSM, WASM_BINARY,
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::crypto::UncheckedInto;

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::account;

use interbtc_rpc::jsonrpc_core::serde_json::{self, json};
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

pub fn local_config(id: ParaId) -> ChainSpec {
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
                "tokenSymbol": ["KSM", "kBTC"]
            }))
            .unwrap(),
        ),
        Extensions {
            relay_chain: "local".into(),
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
                "tokenSymbol": ["KSM", "kBTC"]
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
        Some(
            serde_json::from_value(json!({
                "ss58Format": 42,
                "tokenDecimals": [10, 8],
                "tokenSymbol": ["KSM", "kBTC"]
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
        general_council: GeneralCouncilConfig {
            members: vec![],
            phantom: Default::default(),
        },
        technical_committee: TechnicalCommitteeConfig {
            members: vec![],
            phantom: Default::default(),
        },
        treasury: Default::default(),
        technical_membership: Default::default(),
        democracy: Default::default(),
    }
}
