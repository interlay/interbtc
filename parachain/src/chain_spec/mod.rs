use bitcoin::utils::{virtual_transaction_size, InputType, TransactionInputMetadata, TransactionOutputMetadata};
use cumulus_primitives_core::ParaId;
use frame_support::BoundedVec;
use hex_literal::hex;
use pallet_evm::AddressMapping;
use primitives::{
    AccountId, Balance, CurrencyId, CurrencyId::Token, CurrencyInfo, Rate, Signature, VaultCurrencyPair,
    BITCOIN_MAINNET, BITCOIN_REGTEST, BITCOIN_TESTNET, DOT, IBTC, INTR, KBTC, KINT, KSM,
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use serde_json::{map::Map, Value};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::UncheckedInto, sr25519, storage::Storage, Pair, Public, H160};
use sp_runtime::traits::{IdentifyAccount, Verify, Zero};
use std::str::FromStr;

pub mod interlay;
pub mod kintsugi;
pub mod testnet_interlay;
pub mod testnet_kintsugi;

pub use interlay::{InterlayChainSpec, InterlayDevChainSpec, InterlayDevGenesisExt};
pub use kintsugi::{KintsugiChainSpec, KintsugiDevChainSpec, KintsugiDevGenesisExt};

pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

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

// this is the simplest bytecode to revert without returning any data
// pre-deploy it under all of our precompiles to ensure they can be
// called from within contracts (PUSH1 0x00 PUSH1 0x00 REVERT)
pub const REVERT_BYTECODE: [u8; 5] = [0x60, 0x00, 0x60, 0x00, 0xFD];

// Default dev accounts (taken from Foundry)
pub fn endowed_evm_accounts() -> Vec<[u8; 20]> {
    // Mnemonic: test test test test test test test test test test test junk
    // Derivation path: m/44'/60'/0'/0/
    vec![
        // (0) 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
        hex!["f39fd6e51aad88f6f4ce6ab8827279cfffb92266"],
        // (1) 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d
        hex!["70997970c51812dc3a010c7d01b50e0d17dc79c8"],
        // (2) 0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a
        hex!["3c44cdddb6a900fa2b585dd299e03d12fa4293bc"],
        // (3) 0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6
        hex!["90f79bf6eb2c4f870365e785982e1f101e93b906"],
        // (4) 0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a
        hex!["15d34aaf54267db7d7c367839aaf71a00a2c6a65"],
        // (5) 0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba
        hex!["9965507d1a55bcc2695c58ba16fb37d819b0a4dc"],
        // (6) 0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e
        hex!["976ea74026e726554db657fa54763abd0c3a0aa9"],
        // (7) 0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356
        hex!["14dc79964da2c08b23698b3d3cc7ca32193d9955"],
        // (8) 0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97
        hex!["23618e81e3f5cdf7f54c3d65f7fbc0abf5b21e8f"],
        // (9) 0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6
        hex!["a0ee7a142d267c1f36714e4a8f75612f20a79720"],
    ]
}
