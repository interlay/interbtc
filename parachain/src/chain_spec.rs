use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use interbtc_rpc::jsonrpc_core::serde_json::{map::Map, Value};
use primitives::{
    AccountId, Balance, CurrencyId, CurrencyId::Token, CurrencyInfo, Signature, VaultCurrencyPair, BITCOIN_MAINNET,
    BITCOIN_TESTNET, DOT, IBTC, INTR, KBTC, KINT, KSM,
};
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

/// Specialized `ChainSpec` for the testnet parachain runtime.
pub type TestnetChainSpec = sc_service::GenericChainSpec<testnet_runtime::GenesisConfig, Extensions>;

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

fn get_authority_keys_from_public_key(src: [u8; 32]) -> (AccountId, AuraId) {
    (src.clone().into(), src.unchecked_into())
}

fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AuraId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<AuraId>(seed),
    )
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
fn get_interlay_session_keys(keys: AuraId) -> interlay_runtime::SessionKeys {
    interlay_runtime::SessionKeys { aura: keys }
}

fn get_testnet_session_keys(keys: AuraId) -> testnet_runtime::SessionKeys {
    testnet_runtime::SessionKeys { aura: keys }
}

const DEFAULT_MAX_DELAY_MS: u32 = 60 * 60 * 1000; // one hour
const DEFAULT_DUST_VALUE: Balance = 1000;
const DEFAULT_BITCOIN_CONFIRMATIONS: u32 = 1;
const SECURE_BITCOIN_CONFIRMATIONS: u32 = 6;

fn testnet_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [KINT, KBTC, KSM, INTR, IBTC, DOT].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), testnet_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), BITCOIN_TESTNET.into());
    properties
}

fn kintsugi_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [KINT, KBTC, KSM, INTR, IBTC, DOT].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), kintsugi_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), BITCOIN_MAINNET.into());
    properties
}

fn interlay_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [INTR, IBTC, DOT, KINT, KBTC, KSM].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), interlay_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), BITCOIN_MAINNET.into());
    properties
}

fn expected_transaction_size() -> u32 {
    virtual_transaction_size(
        TransactionInputMetadata {
            count: 4,
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

pub fn local_config(id: ParaId) -> TestnetChainSpec {
    TestnetChainSpec::from_genesis(
        "interBTC",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_authority_keys_from_seed("Alice")],
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
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        vec![],
        None,
        None,
        None,
        Some(testnet_properties()),
        Extensions {
            relay_chain: "local".into(),
            para_id: id.into(),
        },
    )
}

pub fn development_config(id: ParaId) -> TestnetChainSpec {
    TestnetChainSpec::from_genesis(
        "interBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_authority_keys_from_seed("Alice")],
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
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties()),
        Extensions {
            relay_chain: "dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn staging_testnet_config(id: ParaId) -> TestnetChainSpec {
    TestnetChainSpec::from_genesis(
        "interBTC",
        "staging_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                // 5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv (//sudo/1)
                get_account_id_from_string("5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv"),
                vec![
                    // 5EqCiRZGFZ88JCK9FNmak2SkRHSohWpEFpx28vwo5c1m98Xe (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "7a6868acf544dc5c3f2f9f6f9a5952017bbefb51da41819307fc21cf3efb554d"
                    ]),
                    // 5DbwRgYTAtjJ8Mys8ta8RXxWPcSmiyx4dPRsvU1k4TYyk4jq (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "440e84dd3604be606f3110c21f93a0e981fb93b28288270dcdce8a43c68ff36e"
                    ]),
                    // 5GVtSRJmnFxVcFz7jejbCrY2SREhZJZUHuJkm2KS75bTqRF2 (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "c425b0d9fed64d3bd5be0a6d06053d2bfb72f4983146788f5684aec9f5eb0c7f"
                    ]),
                ],
                vec![
                    // 5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv (//sudo/1)
                    get_account_id_from_string("5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv"),
                    // 5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq (//oracle/1)
                    get_account_id_from_string("5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq"),
                    // 5FgWDuxgS8VasP6KtvESHUuuDn6L8BTCqbYyFW9mDwAaLtbY (//account/1)
                    get_account_id_from_string("5FgWDuxgS8VasP6KtvESHUuuDn6L8BTCqbYyFW9mDwAaLtbY"),
                    // 5H3n25VshwPeMzKhn4gnVEjCEndFsjt85ydW2Vvo8ysy7CnZ (//account/2)
                    get_account_id_from_string("5H3n25VshwPeMzKhn4gnVEjCEndFsjt85ydW2Vvo8ysy7CnZ"),
                    // 5GKciEHZWSGxtAihqGjXC6XpXSGNoudDxACuDLbYF1ipygZj (//account/3)
                    get_account_id_from_string("5GKciEHZWSGxtAihqGjXC6XpXSGNoudDxACuDLbYF1ipygZj"),
                    // 5GjJ26ffHApgUFLgxKWpWL5T5ppxWjSRJe42PjPNATLvjcJK (//account/4)
                    get_account_id_from_string("5GjJ26ffHApgUFLgxKWpWL5T5ppxWjSRJe42PjPNATLvjcJK"),
                    // 5DqzGaydetDXGya818gyuHA7GAjEWRsQN6UWNKpvfgq2KyM7 (//account/5)
                    get_account_id_from_string("5DqzGaydetDXGya818gyuHA7GAjEWRsQN6UWNKpvfgq2KyM7"),
                ],
                vec![(
                    // 5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq (//oracle/1)
                    get_account_id_from_string("5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties()),
        Extensions {
            relay_chain: "staging".into(),
            para_id: id.into(),
        },
    )
}

pub fn rococo_testnet_config(id: ParaId) -> TestnetChainSpec {
    TestnetChainSpec::from_genesis(
        "interBTC",
        "rococo_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                // 5E4hDxbuLqzpAhcEsqaJKULgkTcEfzAqsbEQLV471cDC2Hhx (//sudo/1)
                get_account_id_from_string("5E4hDxbuLqzpAhcEsqaJKULgkTcEfzAqsbEQLV471cDC2Hhx"),
                vec![
                    // 5GELfhX7eEeJfXuSe3NdfVyj13yKYegBtg8BLPQxeKDbAwzd (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "b84a0f13ef5eb4d7c1caf735081bd2c91667b84f4b18cd7fa176a73ffd36c133"
                    ]),
                    // 5Et1qfhf6zNmgZF7JWYApngE4HCxw2SxZTWKLEMZ73cFBnh6 (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "7c8d8946973c243888a4eba8f34288cc9f26a3b0f7114b932d6fde362ad67034"
                    ]),
                    // 5Cw1w8J8W8grtyWLUT8bs7GtjCm483pGT1ym8Q6K3HVaRcWb (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "265f1f526a9360030fcb0780ca597e398930cd9571f161b67d33d2bdd9957024"
                    ]),
                ],
                vec![
                    // 5E4hDxbuLqzpAhcEsqaJKULgkTcEfzAqsbEQLV471cDC2Hhx (//sudo/1)
                    get_account_id_from_string("5E4hDxbuLqzpAhcEsqaJKULgkTcEfzAqsbEQLV471cDC2Hhx"),
                    // 5FKuXEdswjda6EfXtWcTbdVH8vQbmNDWhK2qrPGx6GeHvvZh (//oracle/1)
                    get_account_id_from_string("5FKuXEdswjda6EfXtWcTbdVH8vQbmNDWhK2qrPGx6GeHvvZh"),
                    // 5CRrztZ1XYGBZ2asHJFD81W1vSpWiDqq8ndGJmLpRQboeMjM (//account/1)
                    get_account_id_from_string("5CRrztZ1XYGBZ2asHJFD81W1vSpWiDqq8ndGJmLpRQboeMjM"),
                    // 5GjX9J4w1QkfbzoeRL9Uv2JjLg7DkcJfFt4CnKYcPtgkXtmb (//account/2)
                    get_account_id_from_string("5GjX9J4w1QkfbzoeRL9Uv2JjLg7DkcJfFt4CnKYcPtgkXtmb"),
                    // 5GNTqNZL5ADeHRML85C5Y7tdDCZiiXbN3JJNEZvKJXVbyHUT (//account/3)
                    get_account_id_from_string("5GNTqNZL5ADeHRML85C5Y7tdDCZiiXbN3JJNEZvKJXVbyHUT"),
                    // 5HjPDnGAx3opfZtu3wKiZ7BYXXAjEgjwKiufXtZfesTCMgmP (//account/4)
                    get_account_id_from_string("5HjPDnGAx3opfZtu3wKiZ7BYXXAjEgjwKiufXtZfesTCMgmP"),
                    // 5GuXvbk5MaXvm9enTocGmzF8L7T6djzgt4T29SGAFDvLHmAL (//account/5)
                    get_account_id_from_string("5GuXvbk5MaXvm9enTocGmzF8L7T6djzgt4T29SGAFDvLHmAL"),
                ],
                vec![(
                    // 5FKuXEdswjda6EfXtWcTbdVH8vQbmNDWhK2qrPGx6GeHvvZh (//oracle/1)
                    get_account_id_from_string("5FKuXEdswjda6EfXtWcTbdVH8vQbmNDWhK2qrPGx6GeHvvZh"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties()),
        Extensions {
            relay_chain: "rococo".into(),
            para_id: id.into(),
        },
    )
}

pub fn rococo_local_testnet_config(id: ParaId) -> TestnetChainSpec {
    development_config(id)
}

pub fn westend_testnet_config(id: ParaId) -> TestnetChainSpec {
    TestnetChainSpec::from_genesis(
        "interBTC",
        "westend_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                // 5DjsgavDiY8xMcR4riDvs9JXYUpCMnHBe45xsA1rPeBD5woG (//sudo/1)
                get_account_id_from_string("5DjsgavDiY8xMcR4riDvs9JXYUpCMnHBe45xsA1rPeBD5woG"),
                vec![
                    // 5FxMV7qEw3h5yJkrbxUtW18FzU7jhBzeCHfbLB1CDJ1ikyVY (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "ac18e27687e17fe0a7fc49e3c4bf22673b5beb4f38fa950e62ec4105e9842714"
                    ]),
                    // 5Cr6MMKUAbKSzhmLmi9RRNeMfkh7eMXS3Ya11mBTSTRQGBTu (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "229dc43a3b9647a4c8b1aa44b1655ea8655f00c44740ec6bb8e45a628fc99a7c"
                    ]),
                    // 5FHuLURhM4aDXy7Rd6e4Lbbg9H7fbQcUutMbRviaPjCi5SZt (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "8ec588a0de7ba6e877c676e2f276254f8033141df8ee9fad66f89090c6c3b376"
                    ]),
                ],
                vec![
                    // 5DjsgavDiY8xMcR4riDvs9JXYUpCMnHBe45xsA1rPeBD5woG (//sudo/1)
                    get_account_id_from_string("5DjsgavDiY8xMcR4riDvs9JXYUpCMnHBe45xsA1rPeBD5woG"),
                    // 5DMALjH2zJXa4YgG33J2YFBHKeWeP6M7pHugEi5Bk8Qda6bs (//oracle/1)
                    get_account_id_from_string("5DMALjH2zJXa4YgG33J2YFBHKeWeP6M7pHugEi5Bk8Qda6bs"),
                    // 5ENdYBBpnXWMcufn84g6zNevaKdsuFzyCPJu9zG8q6jqwZPu (//account/1)
                    get_account_id_from_string("5ENdYBBpnXWMcufn84g6zNevaKdsuFzyCPJu9zG8q6jqwZPu"),
                    // 5ECo5XVKPRwMu1Zue9deUChx4VmJiaUz5JY4fVFa7zWz555D (//account/2)
                    get_account_id_from_string("5ECo5XVKPRwMu1Zue9deUChx4VmJiaUz5JY4fVFa7zWz555D"),
                    // 5CDjUcujZfXmJv4cmP5cUS7N96yiJNfN9ScTE1QHDak3vEnD (//account/3)
                    get_account_id_from_string("5CDjUcujZfXmJv4cmP5cUS7N96yiJNfN9ScTE1QHDak3vEnD"),
                    // 5FRnXtLTLNbGuEF63YkqLwEeeDh1xtuaCy6Qp3VEUZErJa4M (//account/4)
                    get_account_id_from_string("5FRnXtLTLNbGuEF63YkqLwEeeDh1xtuaCy6Qp3VEUZErJa4M"),
                    // 5CAep2mugERSXpCQTWT5i9vLJXtF1L7CqwpKhVBrmwKsix4A (//account/5)
                    get_account_id_from_string("5CAep2mugERSXpCQTWT5i9vLJXtF1L7CqwpKhVBrmwKsix4A"),
                ],
                vec![(
                    // 5DMALjH2zJXa4YgG33J2YFBHKeWeP6M7pHugEi5Bk8Qda6bs (//oracle/1)
                    get_account_id_from_string("5DMALjH2zJXa4YgG33J2YFBHKeWeP6M7pHugEi5Bk8Qda6bs"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
                false,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties()),
        Extensions {
            relay_chain: "westend".into(),
            para_id: id.into(),
        },
    )
}

fn default_pair_interlay(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: interlay_runtime::GetWrappedCurrencyId::get(),
    }
}

fn default_pair_kintsugi(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: kintsugi_runtime::GetWrappedCurrencyId::get(),
    }
}

fn testnet_genesis(
    root_key: AccountId,
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
    start_shutdown: bool,
) -> testnet_runtime::GenesisConfig {
    testnet_runtime::GenesisConfig {
        system: testnet_runtime::SystemConfig {
            code: testnet_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_system: Default::default(),
        parachain_info: testnet_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: testnet_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: testnet_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                    // account id
                        acc.clone(),                    // validator id
                        get_testnet_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: testnet_runtime::SecurityConfig {
            initial_status: if start_shutdown {
                testnet_runtime::StatusCode::Shutdown
            } else {
                testnet_runtime::StatusCode::Error
            },
        },
        sudo: testnet_runtime::SudoConfig {
            // Assign network admin rights.
            key: Some(root_key.clone()),
        },
        tokens: testnet_runtime::TokensConfig {
            balances: endowed_accounts
                .iter()
                .flat_map(|k| {
                    vec![
                        (k.clone(), Token(DOT), 1 << 60),
                        (k.clone(), Token(INTR), 1 << 60),
                        (k.clone(), Token(KSM), 1 << 60),
                        (k.clone(), Token(KINT), 1 << 60),
                    ]
                })
                .collect(),
        },
        vesting: Default::default(),
        oracle: testnet_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: testnet_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(testnet_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        },
        issue: testnet_runtime::IssueConfig {
            issue_period: testnet_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: testnet_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: testnet_runtime::DAYS,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: testnet_runtime::ReplaceConfig {
            replace_period: testnet_runtime::DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: testnet_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(Token(KSM), 0)],
            punishment_delay: testnet_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_kintsugi(Token(KSM)), 1000 * KSM.one())],
            secure_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
            premium_redeem_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(135, 100).unwrap(),
            )], /* 135% */
            liquidation_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(110, 100).unwrap(),
            )], /* 110% */
        },
        fee: testnet_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),    // 5%
            theft_fee_max: 10000000,                                         // 0.1 BTC
        },
        refund: testnet_runtime::RefundConfig {
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
            refund_transaction_size: expected_transaction_size(),
        },
        nomination: testnet_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        supply: testnet_runtime::SupplyConfig {
            initial_supply: testnet_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: testnet_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: testnet_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
    }
}

pub fn kintsugi_mainnet_config() -> KintsugiChainSpec {
    let id: ParaId = 2092.into();
    KintsugiChainSpec::from_genesis(
        "Kintsugi",
        "kintsugi",
        ChainType::Live,
        move || {
            kintsugi_mainnet_genesis(
                vec![
                    // 5DyzufhT1Ynxk9uxrWHjrVuap8oB4Zz7uYdquZHxFxvYBovd (//authority/0)
                    hex!["54e1a41c9ba60ca45e911e8798ba9d81c22b04435b04816490ebddffe4dffc5c"].unchecked_into(),
                    // 5EvgAvVBQXtFFbcN74rYR2HE8RsWsEJHqPHhrGX427cnbvY2 (//authority/1)
                    hex!["7e951061df4d5b61b31a69d62233a5a3a2abdc3195902dd22bc062fadbf42e17"].unchecked_into(),
                    // 5Hp2yfUMoA5uJM6DQpDJAuCHdzvhzn57gurH1Cxp4cUTzciB (//authority/2)
                    hex!["fe3915da55703833883c8e0dc9a81bc5ab5e3b4099b23d810cd5d78c6598395b"].unchecked_into(),
                    // 5FQzZEbc5CtF7gR1De449GtvDwpyVwWPZMqyq9yjJmxXKmgU (//authority/3)
                    hex!["942dd2ded2896fa236c0f0df58dff88a04d7cf661a4676059d79dc54a271234a"].unchecked_into(),
                    // 5EqmSYibeeyypp2YGtJdkZxiNjLKpQLCMpW5J3hNgWBfT9Gw (//authority/4)
                    hex!["7ad693485d4d67a2112881347a553009f0c1de3b26e662aa3863085f536d0537"].unchecked_into(),
                    // 5E1WeDF5L8xXLmMnLmJUCXo5xqLD6zzPP14T9vESydQmUA29 (//authority/5)
                    hex!["5608fa7874491c640d0420f5f44650a0b5b8b67411b2670b68440bb97e74ee1c"].unchecked_into(),
                    // 5D7eFVnyAhcbEJAPAVENqoCr44zTbztsiragiYjz1ExDePja (//authority/6)
                    hex!["2e79d45517532bc4b6b3359be9ea2aa8b711a0a5362880cfb6651bcb87fe1b05"].unchecked_into(),
                    // 5FkCciu8zasoDoViTbAYpcHgitQgB5GHN64HWdXyy8kykXFK (//authority/7)
                    hex!["a2d4159da7f458f8140899f443b480199c65e75ffb755ea9e097aa5b18352001"].unchecked_into(),
                    // 5H3E3GF1LUeyowgRx47n8AJsRCyzA4f2YNuTo4qEQy7fbbBo (//authority/8)
                    hex!["dc0c47c6f8fd81190d4fcee4ab2074db5d83eaf301f2cd795ec9b39b8e753f66"].unchecked_into(),
                    // 5ERqgB3mYvotBFu6vVf7fdnTgxHJvVidBpQL8W4yrpFL25mo (//authority/9)
                    hex!["6896f1128f9a92c68f14713f0cbeb67a402621d7c80257ea3b246fcca5aede17"].unchecked_into(),
                ],
                vec![(
                    get_account_id_from_string("5DcrZv97CipkXni4aXcg98Nz9doT6nfs6t3THn7hhnRXTd6D"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(kintsugi_properties()),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: id.into(),
        },
    )
}

fn kintsugi_mainnet_genesis(
    initial_authorities: Vec<AuraId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
) -> kintsugi_runtime::GenesisConfig {
    kintsugi_runtime::GenesisConfig {
        system: kintsugi_runtime::SystemConfig {
            code: kintsugi_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_system: Default::default(),
        parachain_info: kintsugi_runtime::ParachainInfoConfig { parachain_id: id },
        aura: kintsugi_runtime::AuraConfig {
            authorities: initial_authorities,
        },
        aura_ext: Default::default(),
        security: kintsugi_runtime::SecurityConfig {
            initial_status: kintsugi_runtime::StatusCode::Shutdown,
        },
        tokens: Default::default(),
        vesting: Default::default(),
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
            minimum_collateral_vault: vec![(Token(KSM), 3 * KSM.one())],
            punishment_delay: kintsugi_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_kintsugi(Token(KSM)), 317 * KSM.one())], /* 317 ksm, about
                                                                                                    * 100k
                                                                                                    * USD at
                                                                                                    * time of writing */
            secure_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )], /* 260% */
            premium_redeem_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )], /* 200% */
            liquidation_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
        },
        fee: kintsugi_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),    // 5%
            theft_fee_max: 10000000,                                         // 0.1 BTC
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
        supply: kintsugi_runtime::SupplyConfig {
            initial_supply: kintsugi_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: kintsugi_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: kintsugi_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
    }
}

pub fn interlay_mainnet_config() -> InterlayChainSpec {
    let id: ParaId = 2032.into();
    InterlayChainSpec::from_genesis(
        "Interlay",
        "interlay",
        ChainType::Live,
        move || {
            interlay_mainnet_genesis(
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
                vec![(
                    get_account_id_from_string("5FyE5kCDSVtM1KmscBBa2Api8ZsF2DBT81QHf9RuS2NntUPw"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(interlay_properties()),
        Extensions {
            relay_chain: "polkadot".into(),
            para_id: id.into(),
        },
    )
}

fn interlay_mainnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
) -> interlay_runtime::GenesisConfig {
    interlay_runtime::GenesisConfig {
        system: interlay_runtime::SystemConfig {
            code: interlay_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
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
        tokens: Default::default(),
        vesting: Default::default(),
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
            minimum_collateral_vault: vec![(Token(DOT), 30 * DOT.one())],
            punishment_delay: interlay_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_interlay(Token(DOT)), 3333 * DOT.one())], /* 3333 DOT, about 100k
                                                                                                     * USD at
                                                                                                     * time of writing */
            secure_collateral_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )], /* 260% */
            premium_redeem_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )], /* 200% */
            liquidation_collateral_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
        },
        fee: interlay_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),    // 5%
            theft_fee_max: 10000000,                                         // 0.1 BTC
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
        supply: interlay_runtime::SupplyConfig {
            initial_supply: interlay_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: interlay_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: interlay_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
    }
}
