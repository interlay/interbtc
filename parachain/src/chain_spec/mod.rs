use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use frame_support::BoundedVec;
use hex_literal::hex;
use primitives::{
    AccountId, Balance, CurrencyId, CurrencyId::Token, CurrencyInfo, Signature, VaultCurrencyPair, BITCOIN_MAINNET,
    BITCOIN_REGTEST, BITCOIN_TESTNET, DOT, IBTC, INTR, KBTC, KINT, KSM,
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use serde_json::{map::Map, Value};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify, Zero};
use std::str::FromStr;

pub mod interlay;
pub mod kintsugi;
pub mod testnet_interlay;
pub mod testnet_kintsugi;

// The URL for the telemetry server.
// pub const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// Specialized `ChainSpec` for the interlay parachain runtime.
pub type InterlayChainSpec = sc_service::GenericChainSpec<interlay_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the kintsugi parachain runtime.
pub type KintsugiChainSpec = sc_service::GenericChainSpec<kintsugi_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the kintsugi testnet parachain runtime.
pub type KintsugiTestnetChainSpec = sc_service::GenericChainSpec<testnet_kintsugi_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the interlay testnet parachain runtime.
pub type InterlayTestnetChainSpec = sc_service::GenericChainSpec<testnet_interlay_runtime::GenesisConfig, Extensions>;

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

const DEFAULT_MAX_DELAY_MS: u32 = 60 * 60 * 1000; // one hour
const DEFAULT_DUST_VALUE: Balance = 1000;
const DEFAULT_BITCOIN_CONFIRMATIONS: u32 = 1;
const SECURE_BITCOIN_CONFIRMATIONS: u32 = 6;

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
