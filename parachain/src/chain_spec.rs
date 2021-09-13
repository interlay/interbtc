use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use interbtc_runtime::{
    AccountId, AuraConfig, BTCRelayConfig, Balance, CurrencyId, FeeConfig, GenesisConfig, IssueConfig,
    NominationConfig, OracleConfig, ParachainInfoConfig, RedeemConfig, RefundConfig, ReplaceConfig, SecurityConfig,
    Signature, StatusCode, SudoConfig, SystemConfig, TokensConfig, VaultRegistryConfig, VestingConfig,
    BITCOIN_BLOCK_SPACING, DAYS, WASM_BINARY,
};
use primitives::{BlockNumber, KINT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};
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

pub fn local_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "interBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_from_seed::<AuraId>("Alice")],
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
                get_account_id_from_string("5DUupBJSyBDcqQudgPR4gttFie3cLPRw3HwaUfq9H2D2mKiA"),
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

fn testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<AuraId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
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
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        parachain_info: ParachainInfoConfig { parachain_id: id },
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
        tokens: TokensConfig { balances: vec![] },
        vesting: Default::default(),
        oracle: OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        },
        issue: IssueConfig {
            issue_period: DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
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
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: ReplaceConfig {
            replace_period: DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: DAYS,
            system_collateral_ceiling: vec![(CurrencyId::KSM, 1000 * CurrencyId::KSM.one())],
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
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
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

pub fn kusama_mainnet_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "kintsugi",
        "kusama",
        ChainType::Live,
        move || {
            mainnet_genesis(
                get_account_id_from_string("5G49RwnYdfHywAfEpsPRhP47XuznQHpaPuSoSdt6S1kyi69g"),
                vec![
                    // 5DyzufhT1Ynxk9uxrWHjrVuap8oB4Zz7uYdquZHxFxvYBovd
                    hex!["54e1a41c9ba60ca45e911e8798ba9d81c22b04435b04816490ebddffe4dffc5c"].unchecked_into(),
                    // 5EvgAvVBQXtFFbcN74rYR2HE8RsWsEJHqPHhrGX427cnbvY2
                    hex!["7e951061df4d5b61b31a69d62233a5a3a2abdc3195902dd22bc062fadbf42e17"].unchecked_into(),
                    // 5Hp2yfUMoA5uJM6DQpDJAuCHdzvhzn57gurH1Cxp4cUTzciB
                    hex!["fe3915da55703833883c8e0dc9a81bc5ab5e3b4099b23d810cd5d78c6598395b"].unchecked_into(),
                    // 5FQzZEbc5CtF7gR1De449GtvDwpyVwWPZMqyq9yjJmxXKmgU
                    hex!["942dd2ded2896fa236c0f0df58dff88a04d7cf661a4676059d79dc54a271234a"].unchecked_into(),
                    // 5EqmSYibeeyypp2YGtJdkZxiNjLKpQLCMpW5J3hNgWBfT9Gw
                    hex!["7ad693485d4d67a2112881347a553009f0c1de3b26e662aa3863085f536d0537"].unchecked_into(),
                    // 5E1WeDF5L8xXLmMnLmJUCXo5xqLD6zzPP14T9vESydQmUA29
                    hex!["5608fa7874491c640d0420f5f44650a0b5b8b67411b2670b68440bb97e74ee1c"].unchecked_into(),
                    // 5D7eFVnyAhcbEJAPAVENqoCr44zTbztsiragiYjz1ExDePja
                    hex!["2e79d45517532bc4b6b3359be9ea2aa8b711a0a5362880cfb6651bcb87fe1b05"].unchecked_into(),
                    // 5FkCciu8zasoDoViTbAYpcHgitQgB5GHN64HWdXyy8kykXFK
                    hex!["a2d4159da7f458f8140899f443b480199c65e75ffb755ea9e097aa5b18352001"].unchecked_into(),
                    // 5H3E3GF1LUeyowgRx47n8AJsRCyzA4f2YNuTo4qEQy7fbbBo
                    hex!["dc0c47c6f8fd81190d4fcee4ab2074db5d83eaf301f2cd795ec9b39b8e753f66"].unchecked_into(),
                    // 5ERqgB3mYvotBFu6vVf7fdnTgxHJvVidBpQL8W4yrpFL25mo
                    hex!["6896f1128f9a92c68f14713f0cbeb67a402621d7c80257ea3b246fcca5aede17"].unchecked_into(),
                    // 5EhzXM7z194e7gdTPakHwS2EBMTM6yY3cgrqnitUkcTx3qvN
                    hex!["74e8a70fa6dd5efdec0bc52d746a5cc72626baedbcfd93a32c06d686cde9fb6d"].unchecked_into(),
                    // 5HdjLATUR3XwFfb2dqMHHPkUu3A43NHCFa3xY76t1WUxJeoG
                    hex!["f65d3a4b8f7a0c251f4adeb1b6297f685278f345fd905f2e6d57403b6071d36a"].unchecked_into(),
                    // 5EjxSnqDQpzm82TzArx6X5fWRn5qKCs5CpYBCisFLyMeH1cT
                    hex!["76682575508fe8eacd79223ae11178b4a8425d90ca32c7812b3b389e7b506778"].unchecked_into(),
                    // 5HgAJJT29LbcZKHyrrynT7E25hYvg8ox2axopoePj5vMv9GT
                    hex!["f837c49801300e494af073b6df97d000620fc268f2d0ba5e66d31acd374af67b"].unchecked_into(),
                    // 5Gs9J1mCewZs6oaXzM1m9DVwqkUA7xkk1UNhMZkket2mX3bH
                    hex!["d45bd9d74b8511f1a2c26cb870ba13171248c23835abe81a7cd571162c521420"].unchecked_into(),
                    // 5ELCC8nm2fpcx5pMdaHSH1k5PfE7X4CNwsoPMvkJumYWQ6JQ
                    hex!["644888c0f310a452a7e3221f52cca64922e5f638f87ec17e8fc73bc45e32aa16"].unchecked_into(),
                    // 5GrnkEi6wi9T5nPjoBhgjRCB4pfeHYCpNWvHNeknPDY3y2MM
                    hex!["d416ae3d869ed826d90f366a8bbf1edd722edbe5954657d66449d82c7e0e5a41"].unchecked_into(),
                    // 5CzNqQVuHu8EbtXryz8kcnBvrLnaKbPoFaFUgmRfvicCYB8Y
                    hex!["28ef3806091fcb50df12003d4c62c81cdd28d2e152ea5bf01189d8cd8ecb0927"].unchecked_into(),
                    // 5GN6a5uU6BNLTUZYYBMHDAj3UHnVbQcs5Hhoizi6K2FbnhbV
                    hex!["be34916ea4a7175d211cf041550346800d798d88208b1cf507025a6ebed12247"].unchecked_into(),
                    // 5HGvZ1WCyYarfPdws7qg6qdFY6i6jKEvUugtp3vmhCBvbcdY
                    hex!["e67ed95d635b20c401be1ea8d682d28a09e1d32304cdaf1d87c47904d6f1bc38"].unchecked_into(),
                    // 5E5LuHWr4Niv46MFfApqyf88Y1G9QRLTpVuSbDSRTUKoKwzm
                    hex!["58f52c00c711897862f381de3a8f9be88bebfb2e37841d57215dd5aa5e51d438"].unchecked_into(),
                    // 5FvDthRvXeUkSvNXscn8bNcHYtM9VBd78SjqZgzvxWWNoJo5
                    hex!["aa78d4fa569bedde43d7b52b1237806b48074308eb841cea71d2b8e7e724012c"].unchecked_into(),
                    // 5ECfNdJcAkEjt2kVB3P9BKx3ad1yGCNFFabhEYsLnUAetJud
                    hex!["5e8a11eafe9312d6feaeff14430221ccbf4c7d85f418adfe4d7ce084ed637519"].unchecked_into(),
                    // 5FhUeVMd3FEWfEVMddkZgoD7As8wQSekqzkKQHWYH3nL4oJN
                    hex!["a0c04fc1d9bda258017ed3632afbd98228332f526e06ed485a6947b618ce6337"].unchecked_into(),
                    // 5CqpnaqjVoZ8r4u3be5RuWUnAEznW7B1BXFJo7EPkXzAr3qz
                    hex!["22695ec7b572198690d3f427df703dd53602aa2067e03c2272e38325489ab56e"].unchecked_into(),
                    // 5G3wfLgP6GSE9kRVt5Nwi5NJ5HLznUujWo5rZQxWKpvckik7
                    hex!["b05c28baa73f083a6ddc7dc9eb61bade0c78adc6e3c0b20d5d13a74c38eb7306"].unchecked_into(),
                    // 5HEMPqyvfiMddVa4giN1N6mDYi2g8j4MuSzBVujmLyynvqBG
                    hex!["e488bd0d526a384b9556f4c9200bce729863fbd9a6ec53d1812e745b136efd28"].unchecked_into(),
                    // 5D2xyGox4gaGPczG9xK6SW46TPHByDW2JVXikDu1xNvdrvkE
                    hex!["2ae89ed78cf10320cb59ab106cf8180f97a664d91b726bf69b5e01bd7d84060b"].unchecked_into(),
                    // 5C5TSbMEJDFkppN4V1sCd17n25hLfXYzALM1UY3sbyBroiop
                    hex!["0092b8488e09aaec7bca178c90a075a9fd6f2d70ecdc115dfc6fdbcb91c88876"].unchecked_into(),
                    // 5DcP4SrNPz366T8p9XbPLZuXtjRGBrmfKavkeCnyFFjeTLJ6
                    hex!["4464cf6dcc7b817df845816e87920fe5bffbc400334da6abd31d806ce0a34652"].unchecked_into(),
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

fn mainnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<AuraId>,
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
        security: SecurityConfig {
            initial_status: StatusCode::Shutdown,
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key.clone(),
        },
        tokens: TokensConfig {
            balances: initial_allocation
                .iter()
                .map(|(who, amount)| (who.clone(), KINT, *amount))
                .collect(),
        },
        vesting: VestingConfig { vesting: vesting_list },
        oracle: OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: false,
            disable_inclusion_check: false,
        },
        issue: IssueConfig {
            issue_period: DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
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
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: ReplaceConfig {
            replace_period: DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: VaultRegistryConfig {
            minimum_collateral_vault: vec![(CurrencyId::KSM, 0)],
            punishment_delay: DAYS,
            system_collateral_ceiling: vec![(CurrencyId::KSM, 5533 * CurrencyId::KSM.one())], /* 5533 ksm, about 2 mm
                                                                                               * USD at
                                                                                               * time of writing */
            secure_collateral_threshold: vec![(CurrencyId::KSM, FixedU128::checked_from_rational(300, 100).unwrap())], /* 300% */
            premium_redeem_threshold: vec![(CurrencyId::KSM, FixedU128::checked_from_rational(175, 100).unwrap())], /* 175% */
            liquidation_collateral_threshold: vec![(
                CurrencyId::KSM,
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
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
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
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
