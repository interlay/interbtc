extern crate hex;

#[cfg(test)]
use mocktopus::macros::mockable;

pub use crate::merkle::MerkleProof;
use crate::{
    formatter::{Formattable, TryFormattable},
    merkle::MerkleTree,
    parser::{extract_address_hash_scriptsig, extract_address_hash_witness},
    utils::{log2, reverse_endianness, sha256d_le},
    Address, Error, PublicKey, Script,
};
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
pub use sp_core::{H160, H256, U256};
use sp_std::{convert::TryFrom, prelude::*, vec};

#[cfg(feature = "std")]
use codec::alloc::string::String;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub(crate) const SERIALIZE_TRANSACTION_NO_WITNESS: i32 = 0x4000_0000;

/// Bitcoin Script OpCodes
/// <https://github.com/bitcoin/bitcoin/blob/master/src/script/script.h>
#[derive(Copy, Clone)]
pub enum OpCode {
    // push value
    Op0 = 0x00,
    OpPushData1 = 0x4c,
    OpPushData2 = 0x4d,
    OpPushData4 = 0x4e,
    Op1Negate = 0x4f,
    OpReserved = 0x50,
    Op1 = 0x51,
    Op2 = 0x52,
    Op3 = 0x53,
    Op4 = 0x54,
    Op5 = 0x55,
    Op6 = 0x56,
    Op7 = 0x57,
    Op8 = 0x58,
    Op9 = 0x59,
    Op10 = 0x5a,
    Op11 = 0x5b,
    Op12 = 0x5c,
    Op13 = 0x5d,
    Op14 = 0x5e,
    Op15 = 0x5f,
    Op16 = 0x60,

    // control
    OpNop = 0x61,
    OpVer = 0x62,
    OpIf = 0x63,
    OpNotIf = 0x64,
    OpVerIf = 0x65,
    OpVerNotIf = 0x66,
    OpElse = 0x67,
    OpEndIf = 0x68,
    OpVerify = 0x69,
    OpReturn = 0x6a,

    // stack ops
    OpToaltStack = 0x6b,
    OpFromAltStack = 0x6c,
    Op2Drop = 0x6d,
    Op2Dup = 0x6e,
    Op3Dup = 0x6f,
    Op2Over = 0x70,
    Op2Rot = 0x71,
    Op2Swap = 0x72,
    OpIfdup = 0x73,
    OpDepth = 0x74,
    OpDrop = 0x75,
    OpDup = 0x76,
    OpNip = 0x77,
    OpOver = 0x78,
    OpPick = 0x79,
    OpRoll = 0x7a,
    OpRot = 0x7b,
    OpSwap = 0x7c,
    OpTuck = 0x7d,

    // splice ops
    OpCat = 0x7e,
    OpSubstr = 0x7f,
    OpLeft = 0x80,
    OpRight = 0x81,
    OpSize = 0x82,

    // bit logic
    OpInvert = 0x83,
    OpAnd = 0x84,
    OpOr = 0x85,
    OpXor = 0x86,
    OpEqual = 0x87,
    OpEqualVerify = 0x88,
    OpReserved1 = 0x89,
    OpReserved2 = 0x8a,

    // numeric
    Op1Add = 0x8b,
    Op1Sub = 0x8c,
    Op2Mul = 0x8d,
    Op2Div = 0x8e,
    OpNegate = 0x8f,
    OpAbs = 0x90,
    OpNot = 0x91,
    Op0NotEqual = 0x92,

    OpAdd = 0x93,
    OpSub = 0x94,
    OpMul = 0x95,
    OpDiv = 0x96,
    OpMod = 0x97,
    OpLshift = 0x98,
    OpRshift = 0x99,

    OpBooland = 0x9a,
    OpBoolor = 0x9b,
    OpNumEqual = 0x9c,
    OpNumEqualVerify = 0x9d,
    OpNumNotEqual = 0x9e,
    OpLessThan = 0x9f,
    OpGreaterThan = 0xa0,
    OpLessThanOrEqual = 0xa1,
    OpGreaterThanOrEqual = 0xa2,
    OpMin = 0xa3,
    OpMax = 0xa4,

    OpWithin = 0xa5,

    // crypto
    OpRipemd160 = 0xa6,
    OpSha1 = 0xa7,
    OpSha256 = 0xa8,
    OpHash160 = 0xa9,
    OpHash256 = 0xaa,
    OpCodeSeparator = 0xab,
    OpCheckSig = 0xac,
    OpCheckSigverify = 0xad,
    OpCheckMultisig = 0xae,
    OpCheckMultisigVerify = 0xaf,

    // expansion
    OpNop1 = 0xb0,
    OpCheckLocktimeVerify = 0xb1,
    OpCheckSequenceVerify = 0xb2,
    OpNop4 = 0xb3,
    OpNop5 = 0xb4,
    OpNop6 = 0xb5,
    OpNop7 = 0xb6,
    OpNop8 = 0xb7,
    OpNop9 = 0xb8,
    OpNop10 = 0xb9,
}

/// Custom Types

/// Bitcoin raw block header (80 bytes)
#[derive(Encode, Decode, Copy, Clone, TypeInfo, MaxEncodedLen)]
pub struct RawBlockHeader([u8; 80]);

impl Default for RawBlockHeader {
    fn default() -> Self {
        Self([0; 80])
    }
}

impl TryFrom<Vec<u8>> for RawBlockHeader {
    type Error = Error;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        RawBlockHeader::from_bytes(v)
    }
}

impl RawBlockHeader {
    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Result<RawBlockHeader, Error> {
        let slice = bytes.as_ref();
        if slice.len() != 80 {
            return Err(Error::InvalidHeaderSize);
        }
        let mut result = [0u8; 80];
        result.copy_from_slice(slice);
        Ok(RawBlockHeader(result))
    }

    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
    #[cfg(feature = "std")]
    pub fn from_hex<T: AsRef<[u8]>>(hex_string: T) -> Result<RawBlockHeader, Error> {
        let bytes = hex::decode(hex_string).map_err(|_e| Error::MalformedHeader)?;
        Self::from_bytes(&bytes)
    }

    /// Returns the hash of the block header using Bitcoin's double sha256
    pub fn hash(&self) -> H256Le {
        sha256d_le(self.as_bytes())
    }

    /// Returns the block header as a slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq for RawBlockHeader {
    fn eq(&self, other: &Self) -> bool {
        let self_bytes = &self.0[..];
        let other_bytes = &other.0[..];
        self_bytes == other_bytes
    }
}

impl sp_std::fmt::Debug for RawBlockHeader {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}

// Constants
pub const P2PKH_SCRIPT_SIZE: u32 = 25;
pub const P2SH_SCRIPT_SIZE: u32 = 23;
pub const P2WPKH_V0_SCRIPT_SIZE: u32 = 22;
pub const P2WSH_V0_SCRIPT_SIZE: u32 = 34;
pub const HASH160_SIZE_HEX: u8 = 0x14;
pub const HASH256_SIZE_HEX: u8 = 0x20;
pub const MAX_OPRETURN_SIZE: usize = 83;

/// Structs

/// Bitcoin Basic Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct BlockHeader {
    pub merkle_root: H256Le,
    pub target: U256,
    pub timestamp: u32,
    pub version: i32,
    pub hash: H256Le,
    pub hash_prev_block: H256Le,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn update_hash(&mut self) -> Result<H256Le, Error> {
        let new_hash = sha256d_le(&self.try_format()?);

        self.hash = new_hash;
        Ok(self.hash)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum TransactionInputSource {
    /// Spending from transaction with the given hash, from output with the given index
    FromOutput(H256Le, u32),
    /// coinbase transaction with given height
    Coinbase(Option<u32>),
}

/// Bitcoin transaction input
#[derive(PartialEq, Clone, Debug)]
pub struct TransactionInput {
    pub source: TransactionInputSource,
    pub script: Vec<u8>,
    pub sequence: u32,
    pub witness: Vec<Vec<u8>>,
}

impl TransactionInput {
    pub fn with_witness(&mut self, witness: Vec<Vec<u8>>) {
        self.witness = witness;
    }

    pub fn extract_address(&self) -> Result<Address, Error> {
        extract_address_hash_scriptsig(&self.script).or_else(|_| {
            // the last element in the witness slice is either the
            // compressed public key (P2WPKH) or the redeem script (P2WSH)
            if let Some(witness_script) = self.witness.last() {
                extract_address_hash_witness(witness_script)
            } else {
                Err(Error::MalformedTransaction)
            }
        })
    }
}

pub type Value = i64;

/// Bitcoin transaction output
#[derive(PartialEq, Debug, Clone)]
pub struct TransactionOutput {
    pub value: Value,
    pub script: Script,
}

impl TransactionOutput {
    pub fn payment(value: Value, address: &Address) -> TransactionOutput {
        TransactionOutput {
            value,
            script: address.to_script_pub_key(),
        }
    }

    pub fn op_return(value: Value, return_content: &[u8]) -> TransactionOutput {
        TransactionOutput {
            value,
            script: Script::op_return(return_content),
        }
    }

    pub fn extract_address(&self) -> Result<Address, Error> {
        Address::from_script_pub_key(&self.script)
    }
}

/// Bitcoin transaction
// Note: the `default` implementation is used only for testing code
#[derive(PartialEq, Debug, Clone, Default)]
pub struct Transaction {
    pub version: i32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_at: LockTime,
}

#[cfg_attr(test, mockable)]
impl Transaction {
    pub fn tx_id(&self) -> H256Le {
        sha256d_le(&self.format_with(false))
    }

    pub fn hash(&self) -> H256Le {
        sha256d_le(&self.format_with(true))
    }
}

// https://en.bitcoin.it/wiki/NLockTime
#[derive(PartialEq, Debug, Clone)]
pub enum LockTime {
    /// time as unix timestamp
    Time(u32),
    BlockHeight(u32),
}

// for testing code
impl Default for LockTime {
    fn default() -> Self {
        Self::BlockHeight(0)
    }
}

/// Bitcoin block: header and transactions
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn merkle_proof(&self, include: &[H256Le]) -> Result<MerkleProof, Error> {
        let mut proof = MerkleProof {
            block_header: self.header,
            transactions_count: self.transactions.len() as u32,
            flag_bits: Vec::with_capacity(include.len()),
            hashes: vec![],
        };

        let mut tx_ids = Vec::with_capacity(self.transactions.len());
        for tx in self.transactions.iter() {
            tx_ids.push(tx.tx_id());
        }

        let matches: Vec<bool> = self
            .transactions
            .iter()
            .map(|tx| include.contains(&tx.tx_id()))
            .collect();

        let height = proof.compute_partial_tree_height();
        proof.traverse_and_build(height as u32, 0, &tx_ids, &matches)?;
        Ok(proof)
    }
}

/// Generates a new block
/// mined with the given difficulty
///
/// # Example
/// ```ignore
/// let block = BlockBuilder::new()
///     .with_version(4) // or whatever version
///     .with_timestamp(some_timestamp)
///     .with_previous_hash(previous_hash)
///     .with_coinbase(some_address)   // will add the coinbase transaction
///     .add_transaction(some_transaction)
///     .mine(difficulty);
/// ```
pub struct BlockBuilder {
    block: Block,
}

impl Default for BlockBuilder {
    fn default() -> Self {
        BlockBuilder {
            block: Default::default(),
        }
    }
}

impl BlockBuilder {
    pub fn new() -> BlockBuilder {
        let mut ret = Self::default();
        ret.block.header.version = 4;
        ret
    }

    pub fn with_timestamp(&mut self, timestamp: u32) -> &mut Self {
        self.block.header.timestamp = timestamp;
        self
    }

    pub fn with_previous_hash(&mut self, previous_hash: H256Le) -> &mut Self {
        self.block.header.hash_prev_block = previous_hash;
        self
    }

    pub fn with_version(&mut self, version: i32) -> &mut Self {
        self.block.header.version = version;
        self
    }

    pub fn mine(&mut self, target: U256) -> Result<Block, Error> {
        // NOTE: this function is used only for testing
        // so we panic instead of returning a Result
        // as this is a problem on the caller side
        if self.block.transactions.is_empty() {
            panic!("trying to mine a block without a coinbase");
        }
        self.block.header.target = target;
        self.block.header.merkle_root = self.compute_merkle_root()?;
        let mut nonce: u32 = 0;
        // NOTE: this is inefficient because we are serializing the header
        // over and over again but it should not matter because
        // this is meant to be used only for very low difficulty
        // and not for any sort of real-world mining
        while self.block.header.update_hash()?.as_u256() >= target {
            self.block.header.nonce = nonce;
            nonce += 1;
        }
        Ok(self.block.clone())
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> &mut Self {
        self.block.transactions.push(transaction);
        self
    }

    pub fn with_coinbase(&mut self, address: &Address, reward: Value, height: u32) -> &mut Self {
        // TODO: compute witness commitment
        self.block
            .transactions
            .insert(0, generate_coinbase_transaction(address, reward, height, None, None));
        self
    }

    fn compute_merkle_root(&self) -> Result<H256Le, Error> {
        let height = log2(self.block.transactions.len() as u64);
        let mut tx_ids = Vec::with_capacity(self.block.transactions.len());
        for tx in &self.block.transactions {
            tx_ids.push(tx.tx_id());
        }
        MerkleTree::compute_root(0, height, tx_ids.len() as u32, &tx_ids)
    }
}

fn generate_coinbase_transaction(
    address: &Address,
    reward: Value,
    height: u32,
    input_script: Option<Vec<u8>>,
    witness_commitment: Option<Vec<u8>>,
) -> Transaction {
    let mut tx_builder = TransactionBuilder::new();

    let mut input_builder = TransactionInputBuilder::new();
    input_builder
        .with_source(TransactionInputSource::Coinbase(Some(height)))
        .add_witness(&[0; 32])
        .with_sequence(u32::max_value());
    if let Some(script) = input_script {
        input_builder.with_script(&script);
    }
    tx_builder.add_input(input_builder.build());

    // FIXME: this is most likely not what real-world transactions look like
    tx_builder.add_output(TransactionOutput::payment(reward, address));

    if let Some(commitment) = witness_commitment {
        // https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki#commitment-structure
        tx_builder.add_output(TransactionOutput::op_return(0, &commitment));
    }

    tx_builder.build()
}

/// Representation of a Bitcoin blockchain
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
pub struct BlockChain {
    pub chain_id: u32,
    pub start_height: u32,
    pub max_height: u32,
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy, Debug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct H256Le {
    content: [u8; 32],
}

impl H256Le {
    /// Creates a new H256Le hash equals to zero
    pub fn zero() -> H256Le {
        H256Le { content: [0; 32] }
    }

    pub fn is_zero(&self) -> bool {
        self.content == [0; 32]
    }

    /// Creates a H256Le from little endian bytes
    pub fn from_bytes_le(bytes: &[u8]) -> H256Le {
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bytes);
        H256Le { content }
    }

    /// Creates a H256Le from big endian bytes
    pub fn from_bytes_be(bytes: &[u8]) -> H256Le {
        let bytes_le = reverse_endianness(bytes);
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bytes_le);
        H256Le { content }
    }

    #[cfg(feature = "std")]
    pub fn from_hex_le(hex: &str) -> H256Le {
        H256Le::from_bytes_le(&hex::decode(hex).unwrap())
    }

    #[cfg(feature = "std")]
    pub fn from_hex_be(hex: &str) -> H256Le {
        H256Le::from_bytes_be(&hex::decode(hex).unwrap())
    }

    /// Returns the content of the H256Le encoded in big endian
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&reverse_endianness(&self.content[..]));
        content
    }

    /// Returns the content of the H256Le encoded in little endian
    pub fn to_bytes_le(&self) -> [u8; 32] {
        self.content
    }

    /// Returns the content of the H256Le encoded in little endian hex
    #[cfg(feature = "std")]
    pub fn to_hex_le(&self) -> String {
        hex::encode(&self.to_bytes_le())
    }

    /// Returns the content of the H256Le encoded in big endian hex
    #[cfg(feature = "std")]
    pub fn to_hex_be(&self) -> String {
        hex::encode(&self.to_bytes_be())
    }

    /// Returns the value as a U256
    pub fn as_u256(&self) -> U256 {
        U256::from_little_endian(&self.to_bytes_le())
    }

    /// Hashes the value a single time using sha256
    pub fn sha256d(&self) -> Self {
        sha256d_le(&self.to_bytes_le())
    }
}

macro_rules! impl_h256le_from_integer {
    ($type:ty) => {
        impl From<$type> for H256Le {
            fn from(value: $type) -> H256Le {
                let mut bytes: [u8; 32] = Default::default();
                let le_bytes = value.to_le_bytes();
                for i in 0..le_bytes.len() {
                    bytes[i] = le_bytes[i];
                }
                H256Le { content: bytes }
            }
        }
    };
}

impl_h256le_from_integer!(u8);
impl_h256le_from_integer!(u16);
impl_h256le_from_integer!(u32);
impl_h256le_from_integer!(u64);
impl_h256le_from_integer!(i8);
impl_h256le_from_integer!(i16);
impl_h256le_from_integer!(i32);
impl_h256le_from_integer!(i64);

#[cfg(feature = "std")]
impl sp_std::fmt::Display for H256Le {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "0x{}", self.to_hex_be())
    }
}

#[cfg(feature = "std")]
impl sp_std::fmt::LowerHex for H256Le {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "{}", self.to_hex_be())
    }
}

impl PartialEq<H256Le> for H256 {
    fn eq(&self, other: &H256Le) -> bool {
        let bytes_le = H256Le::from_bytes_be(self.as_bytes());
        bytes_le == *other
    }
}

impl PartialEq<H256> for H256Le {
    fn eq(&self, other: &H256) -> bool {
        *other == *self
    }
}

pub(crate) struct CompactUint {
    pub(crate) value: u64,
}

impl CompactUint {
    pub(crate) fn from_usize(value: usize) -> CompactUint {
        CompactUint { value: value as u64 }
    }
}

/// Construct txs from inputs and outputs
pub struct TransactionBuilder {
    transaction: Transaction,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        TransactionBuilder {
            transaction: Transaction {
                version: 2,
                inputs: vec![],
                outputs: vec![],
                lock_at: LockTime::BlockHeight(0),
            },
        }
    }
}

impl TransactionBuilder {
    pub fn new() -> TransactionBuilder {
        Self::default()
    }

    pub fn with_version(&mut self, version: i32) -> &mut Self {
        self.transaction.version = version;
        self
    }

    pub fn with_block_height(&mut self, block_height: u32) -> &mut Self {
        self.transaction.lock_at = LockTime::BlockHeight(block_height);
        self
    }

    pub fn with_locktime(&mut self, locktime: u32) -> &mut Self {
        self.transaction.lock_at = LockTime::Time(locktime);
        self
    }

    pub fn add_input(&mut self, input: TransactionInput) -> &mut Self {
        self.transaction.inputs.push(input);
        self
    }

    pub fn add_output(&mut self, output: TransactionOutput) -> &mut Self {
        self.transaction.outputs.push(output);
        self
    }

    pub fn build(&self) -> Transaction {
        self.transaction.clone()
    }
}

/// Construct transaction inputs
pub struct TransactionInputBuilder {
    transaction_input: TransactionInput,
}

impl Default for TransactionInputBuilder {
    fn default() -> Self {
        TransactionInputBuilder {
            transaction_input: TransactionInput {
                source: TransactionInputSource::FromOutput(H256Le::zero(), 0),
                script: vec![],
                sequence: 0,
                witness: vec![],
            },
        }
    }
}

impl TransactionInputBuilder {
    pub fn new() -> TransactionInputBuilder {
        Self::default()
    }
    pub fn with_source(&mut self, source: TransactionInputSource) -> &mut Self {
        self.transaction_input.source = source;
        self
    }

    pub fn with_script(&mut self, script: &[u8]) -> &mut Self {
        self.transaction_input.script = Vec::from(script);
        self
    }

    pub fn with_p2pkh(&mut self, public_key: &PublicKey, sig: Vec<u8>) -> &mut Self {
        self.transaction_input.script = public_key.to_p2pkh_script_sig(sig).as_bytes().to_vec();
        self
    }

    pub fn with_p2sh(&mut self, public_key: &PublicKey, sig: Vec<u8>) -> &mut Self {
        self.transaction_input.script = public_key.to_p2sh_script_sig(sig).as_bytes().to_vec();
        self
    }

    pub fn with_p2wpkh(&mut self, public_key: &PublicKey, sig: Vec<u8>) -> &mut Self {
        self.transaction_input.witness = vec![sig, public_key.as_bytes().to_vec()];
        self
    }

    pub fn with_p2wsh(&mut self, public_key: &PublicKey, sig: Vec<u8>) -> &mut Self {
        self.transaction_input.witness = vec![sig, public_key.to_redeem_script()];
        self
    }

    pub fn with_sequence(&mut self, sequence: u32) -> &mut Self {
        self.transaction_input.sequence = sequence;
        self
    }

    pub fn add_witness(&mut self, witness: &[u8]) -> &mut Self {
        self.transaction_input.witness.push(Vec::from(witness));
        self
    }

    pub fn build(&self) -> TransactionInput {
        self.transaction_input.clone()
    }
}

#[cfg(test)]
mod tests {
    use frame_support::assert_err;
    use mocktopus::mocking::*;

    use super::*;
    use sp_std::str::FromStr;

    use crate::{parser::parse_transaction, Address};

    fn sample_example_real_rawtx() -> String {
        "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000".to_owned()
    }

    fn sample_example_real_txid() -> String {
        "c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a".to_owned()
    }

    fn sample_example_real_transaction_hash() -> String {
        "b759d39a8596b70b3a46700b83e1edb247e17ba58df305421864fe7a9ac142ea".to_owned()
    }

    #[test]
    fn test_h256() {
        let mut bytes: [u8; 32] = [0; 32];
        bytes[0] = 5;
        bytes[1] = 10;
        let content = H256Le::from_bytes_le(&bytes);
        assert_eq!(content.to_bytes_le(), bytes);
        let bytes_be = content.to_bytes_be();
        assert_eq!(bytes_be[31], 5);
        assert_eq!(bytes_be[30], 10);
        let content_be = H256Le::from_bytes_be(&bytes);
        assert_eq!(content_be.to_bytes_be(), bytes);
    }

    #[test]
    fn test_partial_eq() {
        let mut bytes: [u8; 32] = [0; 32];
        bytes[0] = 5;
        bytes[1] = 10;
        let h256 = H256::from_slice(&bytes);
        let h256_le = H256Le::from_bytes_be(&bytes);
        assert_eq!(h256, h256_le);
        assert_eq!(h256_le, h256);
    }

    #[test]
    fn test_transaction_hash() {
        let raw_tx = hex::decode(&sample_example_real_rawtx()).unwrap();
        let transaction = parse_transaction(&raw_tx).unwrap();
        let expected_hash = H256Le::from_hex_be(&sample_example_real_transaction_hash());
        assert_eq!(transaction.hash(), expected_hash);
    }

    #[test]
    fn test_transaction_txid() {
        clear_mocks();
        let raw_tx = hex::decode(&sample_example_real_rawtx()).unwrap();
        let transaction = parse_transaction(&raw_tx).unwrap();
        let expected_txid = H256Le::from_hex_be(&sample_example_real_txid());
        assert_eq!(transaction.tx_id(), expected_txid);
    }

    #[test]
    fn test_transaction_txid_with_witness() {
        // the witness data should not be included in the input of the hashfunction that calculates the txid  - check
        // that we correctly exclude it.

        // real tx with txinwitness. Look for the txid on https://chainquery.com/bitcoin-cli/getrawtransaction for details
        let raw_tx = "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000";

        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let txid = transaction.tx_id();
        let expected_txid = H256Le::from_hex_be("c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a");
        assert_eq!(txid, expected_txid);

        // txid of transaction without witness is NOT equal to the hash of all bytes
        assert_ne!(sha256d_le(&tx_bytes), expected_txid);
    }

    #[test]
    fn test_transaction_txid_without_witness() {
        // the witness data should not be included in the input of the hashfunction that calculates the txid  - check
        // that without witness, the txid is equal to the hash of the raw bytes

        // real tx with txinwitness. Look for the txid on https://chainquery.com/bitcoin-cli/getrawtransaction for details
        let raw_tx = "020000000210b8fbfb6e1a5d2d30677c4ce797b0520774a6a250c22192eacd63b2f8025970110000006b483045022100819b0bdc0568a549cb5230c4f5fc0561764dd95b2e191efe9ab154bb8a5a95820220021f3547cefe915a5bb2906a89bc7ec4e858077ec9b023b48f7929898207de91012102279da390217bff00f6dbae65c993c714e5cd6b7ea384ffb9d4a51f09f044fa30ffffffff43ac430a2b980dbd82911eed89ec70526ed33ac614137e310f2ca70fefaa8c29010000006a473044022069e74ad037fe7304f8545230a32eff39e8fc6133640ee4bc8eb1b9108d79cfa702206dee0ba9b9e0e329074d414bb92609a34e1ae3c7ef2d0658c29230f4b5e85a2b012103bb7b040b18c3ab6d6c4ea8f42e47cb8628ccbcad016804c327603d80951a5850ffffffff02b80581000000000017a914dfea03c60b988da73084af5c9c863d988ae99a18874c113b00000000001976a914c8b46a12370c76a1e382773a3d044fa17beea53288ac00000000";

        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let txid = transaction.tx_id();
        let expected_txid = H256Le::from_hex_be("c052ff9439346a70488f308b009501178be6bb8cb1ecb2ceac8e8a3a8143c687");
        assert_eq!(txid, expected_txid);

        // txid of transaction without witness is just the hash of all bytes
        assert_eq!(sha256d_le(&tx_bytes), expected_txid);
    }

    #[test]
    fn test_script_height() {
        assert_eq!(Script::height(100).len(), 4);
    }

    #[test]
    fn test_transaction_input_builder() {
        let source = TransactionInputSource::FromOutput(H256Le::from_bytes_le(&[5; 32]), 123);
        let input = TransactionInputBuilder::new()
            .with_sequence(10)
            .with_source(source.clone())
            .build();
        assert_eq!(input.sequence, 10);
        let mut bytes: [u8; 32] = Default::default();
        bytes[0] = 100;
        assert_eq!(input.source, source);
    }

    #[test]
    fn test_transaction_builder() {
        let address = Address::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let return_data = hex::decode("01a0").unwrap();
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(TransactionInputBuilder::new().build())
            .add_output(TransactionOutput::payment(100, &address))
            .add_output(TransactionOutput::op_return(0, &return_data))
            .build();
        assert_eq!(transaction.version, 2);
        assert_eq!(transaction.inputs.len(), 1);
        assert_eq!(transaction.outputs.len(), 2);
        assert_eq!(transaction.outputs[0].value, 100);
        assert_eq!(transaction.outputs[0].extract_address().unwrap(), address);
        assert_eq!(transaction.outputs[1].value, 0);
        assert_eq!(
            transaction.outputs[1].script.extract_op_return_data().unwrap(),
            return_data
        );
    }

    #[test]
    fn test_compute_merkle_root_balanced() {
        // https://www.blockchain.com/btc/block/100000
        let transactions = vec![
            TransactionBuilder::new().with_version(1).build(),
            TransactionBuilder::new().with_version(2).build(),
            TransactionBuilder::new().with_version(3).build(),
            TransactionBuilder::new().with_version(4).build(),
        ];
        Transaction::tx_id.mock_safe(|tx| {
            let txid = match tx.version {
                1 => H256Le::from_hex_be("8c14f0db3df150123e6f3dbbf30f8b955a8249b62ac1d1ff16284aefa3d06d87"),
                2 => H256Le::from_hex_be("fff2525b8931402dd09222c50775608f75787bd2b87e56995a7bdd30f79702c4"),
                3 => H256Le::from_hex_be("6359f0868171b1d194cbee1af2f16ea598ae8fad666d9b012c8ed2b79a236ec4"),
                4 => H256Le::from_hex_be("e9a66845e05d5abc0ad04ec80f774a7e585c6e8db975962d069a522137b80c1d"),
                _ => panic!("should not happen"),
            };
            MockResult::Return(txid)
        });
        let mut builder = BlockBuilder::new();
        for tx in transactions {
            builder.add_transaction(tx);
        }
        let merkle_root = builder.compute_merkle_root().unwrap();
        let expected = H256Le::from_hex_be("f3e94742aca4b5ef85488dc37c06c3282295ffec960994b2c0d5ac2a25a95766");
        assert_eq!(merkle_root, expected);
    }

    #[test]
    fn test_compute_merkle_root_inbalanced() {
        // https://www.blockchain.com/btc/block/100018
        let transactions = vec![
            TransactionBuilder::new().with_version(1).build(),
            TransactionBuilder::new().with_version(2).build(),
            TransactionBuilder::new().with_version(3).build(),
            TransactionBuilder::new().with_version(4).build(),
            TransactionBuilder::new().with_version(5).build(),
        ];
        Transaction::tx_id.mock_safe(|tx| {
            let txid = match tx.version {
                1 => H256Le::from_hex_be("a335b243f5e343049fccac2cf4d70578ad705831940d3eef48360b0ea3829ed4"),
                2 => H256Le::from_hex_be("d5fd11cb1fabd91c75733f4cf8ff2f91e4c0d7afa4fd132f792eacb3ef56a46c"),
                3 => H256Le::from_hex_be("0441cb66ef0cbf78c9ecb3d5a7d0acf878bfdefae8a77541b3519a54df51e7fd"),
                4 => H256Le::from_hex_be("1a8a27d690889b28d6cb4dacec41e354c62f40d85a7f4b2d7a54ffc736c6ff35"),
                5 => H256Le::from_hex_be("1d543d550676f82bf8bf5b0cc410b16fc6fc353b2a4fd9a0d6a2312ed7338701"),
                _ => panic!("should not happen"),
            };
            MockResult::Return(txid)
        });
        let mut builder = BlockBuilder::new();
        for tx in transactions {
            builder.add_transaction(tx);
        }
        let merkle_root = builder.compute_merkle_root().unwrap();
        let expected = H256Le::from_hex_be("5766798857e436d6243b46b5c1e0af5b6806aa9c2320b3ffd4ecff7b31fd4647");
        assert_eq!(merkle_root, expected);
    }

    #[test]
    fn test_mine_block() {
        clear_mocks();
        let address = Address::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();
        assert_eq!(block.header.version, 4);
        assert_eq!(block.header.merkle_root, block.transactions[0].tx_id());
        // should be 3, might change if block is changed
        assert_eq!(block.header.nonce, 3);
        assert!(block.header.nonce > 0);
    }

    #[test]
    fn test_merkle_proof() {
        clear_mocks();
        let address = Address::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(TransactionInputBuilder::new().build())
            .add_output(TransactionOutput::payment(100, &address))
            .build();

        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        // FIXME: flag_bits incorrect
        let proof = block.merkle_proof(&[transaction.tx_id()]).unwrap();
        let bytes = proof.try_format().unwrap();
        MerkleProof::parse(&bytes).unwrap();
    }

    #[test]
    fn extract_witness_address_native_p2wsh() {
        // source: https://blockstream.info/tx/babdc5ac2572569233b4e4720bcfc89f290db8eac6132427914c8272a5233278
        let raw_tx = "0100000000013371dc2489d798152866b18f6547d550dc36184f68757c9839a97e9b89de0050881200000000fdffffff1fc4e57217b2e4fad634d606679dc7f1d0be625b6506325471089c1dc9a3030f1f00000023220020bf70731e86657c04c21916b94ffc5f6c88aa371fd485b0747a1ed87cf2386232fdffffff8e80e7b970a18c3a186434a0dd28912a07befee05d3ffb00a3d7d7713b979fbd0000000023220020b7f11fc8190510214b1e68916c4bf847ab762a43426122c0b210c18c5c5b859dfdffffff1c3b7e5ac28dcd2349f61981be32f8e914fd4556f712851ae41a52ddcefbcd9313000000232200207e91dc22bf6a721df417e4eef20af9f7a4e060ff4c355f9dba14ff2faf347a4dfdffffff958561ee081bb20f83847793eac53195d166a0738c865850a3cf334a3fb737142f000000232200207e91dc22bf6a721df417e4eef20af9f7a4e060ff4c355f9dba14ff2faf347a4dfdffffff2c499bb879df18358924ec8f9d2d7aa43e7af6ee27c777465f145ca51c1cf1eb31000000232200207e91dc22bf6a721df417e4eef20af9f7a4e060ff4c355f9dba14ff2faf347a4dfdffffff25626a8352bc5157d791c636b9d80a903c406b4b50e8ebe18e1d1a0ae63377f600000000232200206217d620e95d66d3b11db9ffd7b926212a9a778ebaff923ce9a69911e071a8ecfdffffff1cbcc9e2e8e5a7c882307c469b87dc1ec262311690bc3e38b5712cc88d1f1e7f18000000232200207c8128029900f035bf471aedbd499688c2d8e088861d564f53322d9255b735cdfdffffff7746c9a26158be28f0f60462022a24df58e80c6d720320d53af48c4a0005610af90000002322002064c0949dacdf5fbe3cba7620facde16afa0d61a671bb7454dc5abb8d0079c9affdffffff9b671d6087647135a869a3b9954a2af3107fca1442a46e81eb1e8fac4e005c3f04000000232200207cd008c5573ae12fba7285239206f31097114fdcac20d8ce107a0fd8dfc48984fdfffffff11da434de4fc1d48e5c44872c0f3be438cac701ca2ae3c3f20a5e8fd97af0da0600000023220020bd9c2892cb4433bc039e3ebd95f5ee74d0f339b123702be77a3bb593958e6ed6fdffffff958561ee081bb20f83847793eac53195d166a0738c865850a3cf334a3fb737149f00000023220020138ffb50b4aea02fa5c48cae8f815324de7662824bebba192e001d1ffdba5c01fdffffff2c499bb879df18358924ec8f9d2d7aa43e7af6ee27c777465f145ca51c1cf1ebe4000000232200201434711448e39923c22df36201d30383a67cb7354007c57f1b6f164b98a25833fdffffff86fcd27fb1ce47846f335b53c088c756d4523cf227e5f3821cd7381d7fbb97e01600000023220020d4c13a38ca1686eb0299138eb6ff9215989e08d9b7307eb6793c545132d936fcfdffffff958561ee081bb20f83847793eac53195d166a0738c865850a3cf334a3fb737140501000023220020138ffb50b4aea02fa5c48cae8f815324de7662824bebba192e001d1ffdba5c01fdffffff33ca2270ca88a0c3df6e97334416b7014a4542d4ca45e19e753ff524db998b5e73010000232200207192d245eca5c3a1ff83519b8f1b36caa1d13f51db6090252f41b29dc3055291fdffffff8cd56a05f942a90c2b53900bdf89d284b3fc40efedc6b7d3a2212709f0f79b735401000023220020f1467b7c8b19cebf7a8f207d64f18d7a41c89d758dc81536717c7a910967ab7bfdffffff7c08d4fd284b0c1fc7c6e5f6da1bd5130db3207d9478f66b1139f6d5ede76b6e3a00000023220020a92f6225614ec86981783a70c3d07064e3347e51fc2f7d793b8f7665dd70bc61fdffffff6dc27fb844c8ad0305cb01fa30dd6f11a4a10b5c1b85ced769fa0bbac7a2a77705000000232200200413611bdc9bab8b7097e108d14c1367fb2fd5031df6b2b91e3ed5c11ee62940fdffffff2c499bb879df18358924ec8f9d2d7aa43e7af6ee27c777465f145ca51c1cf1eb6301000023220020008ce4595400a9d93b13f256d247d4730905b5bde584f07ad8e9ba3bc5885d6cfdffffffb1e127f3091d9b7e9b8c96746f1aec549ff8598aa8068e75c2505d336c3db1280100000023220020558020ea6661bb0dbd83afb11c9fd6d1d02609309349e541e5a3ab0e03e91ad8fdffffffe41d9503063114a3818764c547c44ff8693eeb01b8a2245ad086a27be02d7c655b01000023220020d5c91e08cdbf8152c28a931c066fcca3b9650c1a49a3035e3e282d71eac3a6b6fdffffffadc17f4e37a1a452af84056b2e5a56c1f830df6b87cd9fefb436bfbbd515fd5c000000002322002072ec8c96d3aab5feb70d6c44330dde241b06c10c5d1988eae818917356c47736fdffffff82ce61b2d833134ea268b30290c91726f75a7d06806d9596f57217d20a52b9510000000023220020c38b60e1d2a95c5415115f067037cec7b2f7672586564d56d0cc4d80c32b24f8fdffffffbb2793bdb62b5928ca6a4b787037602a8182c91a5c957329c3404db62da5c10f000000002322002012ebe4986dee5b10d9b859aede2d62381bde6cf081d4b15926d332737063bf8efdffffff2676086728aadab8778c790dd730675ce628ee2b8c76ed580624b1d7c6c3ff750000000023220020b7f11fc8190510214b1e68916c4bf847ab762a43426122c0b210c18c5c5b859dfdffffffa62f654a0c746952957532fd1861bc5b790dd7402a10050ce0bf5e5e5d877ffa0000000023220020257ce87d8da3b1cf130850c357c7a829f1036b2ff2a66ef96069512e67a89312fdffffff13bd9cfc967c01b16be126224fdfaaf4c6f49ab5821d184d797f2a7fc4f3ca630000000023220020b7c31ea27b40af8b9ff6aa5968a9f95c382703954a53dcd945f88723b7fda9d9fdffffff8934ec519206ca8b4ded4517df0d5d372f3805074afd31be1a94c99b34b827b80000000023220020257ce87d8da3b1cf130850c357c7a829f1036b2ff2a66ef96069512e67a89312fdffffffb059d60c5750bb0384dc70d7b6998432f3ec6158ebe8a7c26802bd2dad9eec740300000023220020e07bfdf25e1a0c791cf57d06ebc658b87b56632975ac04f5df7d1c7fdda85d12fdffffffa8da07346fb930e1633f66864bc9e6c0df1a694629e00e74f1eb1824e48d0fc70100000023220020c83082d063b394033df14a0b44b6144a059cd9dc6d16c2ad1ef237bf38d7ecf9fdffffff20b8de7cba6eb6e0ddebf4ca7c24f0df157a2faecd74a1dcbaeb770c79c08a4200000000232200209a24a244f5cf184aad47bae4ac9e0ab230d4c85fc4296b2a47371ea5ad543489fdffffff49f0b49301e0915eaa796be067e47223918be9914159b3b1f35a25d81a9093410500000023220020dede4d7d90ad7e81bfb16136a25d3cfd256db26fc1dc6c1454d3986d1f13a342fdffffff4672a649030a7b5941556ef4c8c0e06928cf6c8e4c1b1ed40897b2d8d4e558203a000000232200203e632d80128167406b1b7524bbd9911785561332b4414baafa747d30d9464d9efdffffffcf17046dfded5663f342883d8aa0a8ab0dcce3c9ab6f9f6eafbb13554856232d0000000023220020a15acc4b32fac0a3e4f05e1c8a605af9765ff68e7debab5ae7ad64e8ccd1bc83fdffffff0c12e83cd8cb2b1e694b09745f910de9cd301503919dd541dc450cdf4134020201000000232200201a8f70939107d86e8b6364c4777eaad294ca978f7dd2ee011950045bcee966fffdffffffdccdf6acaa834bc51ea3472ec80c39b3a39d7cfb09ee929ebb60bfd7f8dc6b26350000002322002048d035ae6542eb437807849b2d3465e9abeca1bcf5a0c2f76d60a7ec4a1f8c3ffdffffff2eb9d0f20b4649f975e1357e04ad41c7419c5f3efab7e4fba468b61f0c16e8ed0100000023220020daa8a1345dd4bbc986401dfd3911ee1d3212e88589753162c3eecdd4f10eae5cfdffffffaf5bb66f906b5d4691a9fb11d043673506f1ff7c45afbfb9f50c2cf5cb91ed5b0100000023220020bfc8009869f275c59b71cd27629e51a87f831529782229d7a3eeabe7681af873fdffffff12bf44b93d86ed0ca8004ef9ba45b1b6b7d5ab34fb1dbefd79679525ddd8499001000000232200205356ff6eecd416856d71e1493540026173115c0a53de25a0a36b296bc424725cfdffffff4672a649030a7b5941556ef4c8c0e06928cf6c8e4c1b1ed40897b2d8d4e558200a00000023220020e218db939ac0b4be2c6d061e121082510655afe6aa3886372c2d87a690e89f01fdffffff8d6116be83c5aeb7190965eae7d8af8628eaabe05c01f4ae10b8e08d9314bc880000000023220020a223fe27b789b088e5a3f4e56d84f4f30b21d698044fe86bada2cb12f4171730fdffffffb9a79b0dfd0fff431e0ddccae1edd5a7ab36d4673c9b48c4b7747fd44b8cc9d6000000002322002035f763022775fccee04731c531bf37f8f13450b8167f8dcbb144c19d91f6cce6fdffffff0a3d72b52d1dd2ecb9d6efd2a4952961c354d69378c85ad2b1f57d64e83214080000000023220020b28ba8896a929f1d9f79a42a00e79bb5538ec3b15803067e0a2503ee031cd836fdffffff8359fa77f6d4823afd2e11221de227f6c9de17a6b9e9e7fd4b46abc7ea7a1bca00000000232200204a55c30fa4ef9c1364f65f163540506c4b8548e5f6b7985bd0ce92cb1479a79dfdffffffd9569ae4b520fe4657d8d46867553b916e6300ae69741317cca86597947ab880000000002322002026b809ccde797a942b18c1cc6838eef3e0bb780ec5c222d2f7c62fe74ccec789fdffffffd2211c43992708472bd36d7762197501c3de85a43db2df9e1f270feebef4d8e300000000232200201a3b7b40ae06e698d5c9816b3fc3450efbd49ece5db2054c2d8e76a8b83b92e3fdffffff03ae52693e72cf14c28577fd06f42106b7b46ba1b9ad5f3605952e3c8eb4a2c900000000232200200c6797e1a0ad933ba57648c962247c9ea3a30b45ac29669545375f6350376333fdffffff47e9a7d47d7e39cec70757f3acebbc8681d237772082c50033ab1a49e391feaa00000000232200200e32cd2df625beab5905eba4af4091bc2284645e163ebebe1eb33d610868ae9bfdffffff4aed1dd8e1628a2399b01eeef0a49d88dd68455dd2446fe1575c9bec5d0add2a0000000023220020623b762cd25985d695e56e446a8a62bed72379c5e2299601c9197046ac33b0b8fdffffff0f4299e82869f579c6017eda4264e3266d1a942a5496178d76a1eb6850f747e90000000023220020466257167c74d4d715e06bab2b7802c5a363c39983b28305291d566f4fd78ad9fdffffff1b78ce040000000000160014b478f16d8f55ef6ac25c25e6e42ab901725736efb87f40000000000017a914a9a4be47adbfbda29d70fa55c59aa7292f63980987b07d09000000000017a91419de4d77d5d2bc1241b4df8f61cdf69cdbb27ba587e86e03000000000017a9141ccacc1caa23e57559b953b96d4e366e484552ed876c8e01000000000017a914e119f8b456ceed01624eca3a2026e09672e2b79287b0785a04000000001976a914a2ae66bf4755b584eb68760ad1eeb2ec4368ec1988ac38af0400000000001976a91459c47a7ec3eacbececff5b27f9c89ea38f608f5188acb8880000000000001976a91434f721c23c62e6fab542a14b69acc8aa4a9e35d288ac28fe1a00000000001976a91469022ec5e005d1a38728faee0959a86fe6870a0188ac60c41e030000000017a9140fd79831691c61bcad40f8a99cbbcfa4c5709ecb8700d60600000000001976a914ad3b00cda70e6af6468c1854d25f1b42b662195e88ac5819a1010000000017a914fd890d35764e97fa23bfeebd88636c7ead3b65a08718de040000000000160014de5acedb3b3444449035e2851769d15cad350cb4e0c63a000000000017a914d76e7058ac134d54103008751875d8b67de0dafe87b88800000000000017a914e3c25212d2a1de27309f4e97f19890a89e25cbec876887eb0b0000000017a9143512381c25f32e873dd9ef1fae38fbf05bf4c54f87e04fb6000000000017a91467d49a512ea1e3bad30ac617648fb539748adb4787904d0400000000001976a9146f7b89ee782ae5f6543d0aa1b7e23ba4c3001f1288acb8ed0400000000001976a914cc198bce1d1cb038288350cb611524fe1d469ffe88acb0126505000000001600147d476be07a55c6244432ed54c7cf299c07e4799ca85b01000000000017a914972f9167cecb009293c4c40eca03cd0d18968d8a87e8b5fa020000000017a914ae83c75b066c6b89069139dfdf4595c742940ca287c83714000000000017a914637c6989a3a71ef7b6a4c3f39ca801b4409f500487e8b5fa020000000017a914ae83c75b066c6b89069139dfdf4595c742940ca287904d04000000000017a9147ce4d38014196ee7381bcae62c6e07eec8d89d828728d668020000000016001447a951f8861995aad53038b512d0000314134e27448f439002000000220020c7a0d34887b00225139b01cc7c4bd6194cb02d7c8fb54877151d991401ee18050400473044022005bdad2cab53ecbb1a5d3f379f6f9fecdf3d13160a05eb126bbfc7341c40742f022012446f5bf3c05e88d102926ddb4d6c43998e69ca1b0b7e375594481d9e68aba001483045022100e2d61d5b4718e111cab53b55f3be2ef8dd3f296ce8d11abe4a9c4999307943e102202189766b74de07ca1b18802a0a5c6ddc0aaae91ff418b55bd638c7af2086977a01695221028d2d3d77487f0dd29d1cbdf9bd190b8ed0949bfe4dd0da685e49472a23feadc02102eccf8ae6b3c0a633b18ca8a33dd8af5146959a086b77ffbaa4ee318ff275564e2103722afb25e10098f2ebd708f6f8065aa16aadc617226f33d8c32b45d953e8b8b953ae0400483045022100a23ab5c8bc40552695f380f355b5553bb3cf08074236e9b4319c487cc46e157e022062e608e8abcfdcaa3fd73d1ab17b4ca81281f0e975160f90a4206fba7b23e0e401483045022100ce27302616002e9d607dfd7e6a4b318d3d9f8ac692ed55923fba7619a815ed4202202ffd877b6783ec301599516869287e88069220b760c7dbafc75db485b85ad22501695221035552a23354ec92e20f02b24f0a854380615951af32a120f680f10093bc4f3ee42102ec5904dfc3ab72e6baf4bd72c08f12d7355059d9a200586674b64f04786186ef210311aaa8381e233afb99b11ada9c8ca6e16e6fa00b235629f150746e2413e31b3c53ae0400473044022051790da9c82919d84fc5de1a95ab1cbd2deab0ec7dab9192a0026f3701ac456802202c3678617c93bc1e0b0a0b206e430055b77baa7d94ac141191b04980469b2e4c01483045022100ced94b3d1044971cba23d629ab4507b06766db9aaffdfa6a35313ff8a3197e940220628ce836f76dccb3b15a7e04fe36448499ef3e5ed1578b5de0924bc2002dccdb016952210349d4e115c685dfd6c597eb3cdf7beff83632a4879adc5a7d0a806c2ef7c48f6b2103a381606dce4ded6487b47ce132b0e5ef883dc348269ce1232e350af2e77045722103ac7216cdfc0269ede9b3a07d8e9516ce556cd3e5b567604bb855c0f3d563987853ae040047304402204b92b3dc7326dfa3f4cf68efb14b4e963231d551d2f5afcc1d5ae24f2a13ef3102201c9995dd74e81d859fb18d7ec9632a991a7bbcdbe5eab997800456cd5dceecd301483045022100db53635600bf8f3c8c62bf4290e5de307ecac78d0c4ff5da11928d1eb71ae51802203addcc7bfbf3b99b4d2d624505fcc647249b687a140a89bb4204a48e80e2e79d01695221034bd002b755b568e9cfc2533c61c466395661a31b336b05f5765d2709c4de398e2103ab4527d08aa2cd6b89d07fe252246d3c2e1eff1a1d11739841cbe20f944719d1210387c69664146440bf6f393af5f77ccedcdb5309c60403973467afb56a3ec67c8953ae0400473044022017cd5d5396329aee1c1abd519b4f8881941a292b51e44223d32d139d36fec23b022018a07828ccbe5693e0db2bb156a1a7ad003b667b9d9381d8ba8a616cf45d9cb90147304402202adf732a884081afc8d6a330f1d226e82515c44d3310213b60942fd74dd73e40022037ea11a65bc66837349eb25b1667a927cc2f9ca4fec5ec7133a90865584021e601695221034bd002b755b568e9cfc2533c61c466395661a31b336b05f5765d2709c4de398e2103ab4527d08aa2cd6b89d07fe252246d3c2e1eff1a1d11739841cbe20f944719d1210387c69664146440bf6f393af5f77ccedcdb5309c60403973467afb56a3ec67c8953ae0400483045022100abe8e87c17d69a42e20932ccff89596d246e99f7c1b06e7d258236ec04eb682b0220207535928d4c8dd4fdf15b2269ba6d8bbd64e9cd5e3874925efcd43f783e256901483045022100de9acd0054fa2bf709c340d21246e37e8cc6d04a493ce1e1130ddaadb3c05b4b022078269fedb249e5bed8e209c9429964c45b311d713e6b8e18c4c24172d89a99f401695221034bd002b755b568e9cfc2533c61c466395661a31b336b05f5765d2709c4de398e2103ab4527d08aa2cd6b89d07fe252246d3c2e1eff1a1d11739841cbe20f944719d1210387c69664146440bf6f393af5f77ccedcdb5309c60403973467afb56a3ec67c8953ae0400473044022070f2f2c6bb2f0f4638bc3b17b5129d927ea38ee225a04e096bb65b81927f894002204b005a09bed7ad365ab355970450b6aa73176e7b76e66b61a90907fabd885df9014830450221009bcc587e7e1985885c0c14ce3838f5d03ad60799b5346b67df2b5757aa2390fd022060f7f5be26612a0b8e32cbceb51872d288b3ea7173e503051e787b2eaeae045101695221032514f42322d9b324750b9472beb96e6e58f45d252bd91b7a1eef8428a6d97b572102888621ee665dd13856127edcbf0b93345d0966c0557eb79f39fc8b69b3d45ddd2102a1035f6931ef39c93bf8188751d4029359bfb283ce9e689e468771d72ee0620053ae0400483045022100bd034f30a6fecd1c561045b1747df8e93f38bf3a62608025cdc840b23de8db620220476db1d026cd17ee7bb51ad1c73e27fc994d712903616be8712630d9ab9f06e701483045022100fb8f855f1386576a65f599617bab21cd7176856e4a430dc1069e581dae3f8c45022029b94e3ded161d5b381f810838a2f690d7fd583cd5bd9130d9171a85277cf4a801695221035cb8f32683a306cd8af12798f29b951a8ed8afa1c957e70dc26ef51cc1bf292d2102de57b713cce763dd2591876f33b43411bcac8e3a388b07a05b7e4af90ae5cc952103c813f127af9d34fe788889eb6cbcc13ea01a254e0c079aefc9153598eafeb2ab53ae040047304402205b15c55339c055f979b33c8f092f85e0b3aaf0c143258720ad6c6a194724b9d002207d7191fb9e846da9384164fe6b76abe0ab2f8d87406fe6d5039fbd36a40aa1a701483045022100a0ff55e364e5efc8d8793746e2039870e05ead03b6b55ae49ec0ea3b4c21c4c50220783dcd481dacf3cd1784184d2741aec16b03f76dbb8ef90ab7bf061e4c88f4320169522103ac1f3a5a07a462c3584654c9f27df860bc01a9e84e85eaeb4219de3af9d053762103e4253ecdfc0b152fd67d8c64b1d61986b81b72d8a2dd03d968be1f8525cb4c842102aabe8443da647a6a4ae6f155731384c6f6e0c4af490897e9e53e992d2f2ed4a653ae040047304402207456e7a1059ff034d07d27c85d91d47770fc7cedf1a09d22f762e30be1de8835022017417ebfb2496fd24cbc30e809ca2244b5c018b3b823e879683dd4ba40aef6cc01483045022100cbedf3fc30ea452ea4e695083796bf8b0adf7a176d08a9a117014ff35301254b02202f60c8d04feeae106d3fdc32c4eb189777f9388b9b791fb7c220b98bc3dad0c4016952210292cd5ff1725d7bc969b02ba2f92b789039a4780baa3ae40a81b86bfaa252a3a32103e5144cf1c186a98de7896d9af1485e9a448aba638caf1921eda676fb3f1c302f21034079e9e9e45a60879e1030d6d17f0a7b3de70513c285ee2d25778c95637e74b253ae04004730440220161c1322b29e60c1706416b1679be92677bce37cf4a53868e47dbb5018fadb6c02203a60f9a36be9ffcc0eb3ba0dc19e73f6bc53e0c0b6ef87550887a610becb71d3014830450221009ca6ed5e478e83f66f2231555f853f8da97a2ff9ea809c321e4a8c22f9798e2e02204ba85f79245a293205e326c513ff9d2c27877600f4658b82eb7f6b24c7c89a740169522102c940543387b792c1e44925cd721efe7677fc1e11758b06e2b68fe617c554d0e62102cc07399ffe7d97cac518db83d1ec8345d9c811fb8dae56626d5f435416d93a812102ef5a41c8bafc0bb00c4d61a0dbeea9965a8e04f07708d6506e901c1aafd7bf0553ae0400483045022100c9cec0b73b0ba70c4db14b7a02f5de4a87b519dc6f11a89cedd020b44816be38022030c843bf60949b585192e698fd8cfcf957fe957ec5805cb80761f86cccc68ef501483045022100e595391bfee85ee0de430676bed8809046d0a3bfdd17ee9af33f215bbd19866202201a2c0277dd4a9299b7efc70a9b46fa6bb24a9f859d69df54470708f1baaee8040169522103c1643d4dbdc2923259fdc07594581938c05c862ab15bda6ad72c4577b33973f02103f32df2afcb5cf7c7158f0df2d273d90ea41ec5f120f26eddd1be139cbdcfa1a2210237381f299d5008782ba4ad192783952a71e4847545a2ab67d842db5b3d77adf853ae04004730440220126bbc05d05c9dd9c5448fc290b4f7a498e7de9069e18f3512fc67ceedf3d4c802207bf57467c71ca9aec3f77e77ed164895205d7d7647bcccec457fd5af5b44d0ab014730440220766a31ab623a9726441c8d176fe3812e336430ffd194c6bd7d16097c90ec44f1022068e55df41c997ead06a5f2cdbe5bd77271b991eeb5a2131e24c43252423319d801695221033a54882001ebd29a1c563541f929e360cecb2553f2c204574d4297c8af0cd987210235f149b4738bb252fe3314bc9bdafba33139f5054be504a2d7fdbe5a0b3146a92102eb7eae91ecf882ba1f2eb97bb1dec4a53f3a090952aafe3ef17910ae6d79d6ff53ae0400483045022100e5ac8cc349a0c9902bba5934a4f5666ef14f27de6e79a07eff8b28f8109262e002201093082d6c9777d727341f4dffee9b5159913764f08b67433a2e6594151bb1eb01483045022100cebb737f6b2dcff4216b07937cc4029c55584b0a2d0234bec1cbac871e07216202203251cbac7bc15a0c45b60b24b8074e9cbefb6bcd44d2b63477b26ca77eed20d401695221027a529ca3429c7a78c2388e71d92568e3dfc5c4523aed8397dacf4ded5ae402d5210315566c6ddc780ee1d04305338d545af10d26eddeed5f75b66c4323ce5e13baeb2103705e19c2bbc4d30ec9cbface6820b9b237cd568c22bc4c2bcef768171691625753ae0400483045022100ab36834405cab5ecf05179a9ba3b38c49dd5526be440e56c0803d9155558f05c0220150c37187d794bdc07b19d8b1f92f6c2563af61f927b642dc1a567400f544ad101483045022100ab27a402af7fd0d27eec3e412a3405b265c7dc35bd5c2aed8f0aa854452d59a6022076353428cac57de47d0d35f4f7ea6768672589ec6c035fcf1f70f8ddc43b8b9a0169522103c1643d4dbdc2923259fdc07594581938c05c862ab15bda6ad72c4577b33973f02103f32df2afcb5cf7c7158f0df2d273d90ea41ec5f120f26eddd1be139cbdcfa1a2210237381f299d5008782ba4ad192783952a71e4847545a2ab67d842db5b3d77adf853ae0400483045022100d55daaa2edba35ac295d22a8b03844ecc68865d50be85839badf07803d70bcda02201e0bc2861abf0e9d6e4c1fa53c06ff684ecff2fa86e93a111e9b27a4025d62330147304402204d1de739110f2a8b14428f1da86ebf08d60cffe0c57ef54a874a9f2568baa44802202f175217e9ea3a2d5da0c2e6f667c2bedf8bb0d71da773526214e10c72c0e07d016952210206bc76276be750523c36a2813aca3bc26c0bf0da81804d10e1ed01c4daa1553b2103fb3fa9c09a09b514b16a2680ea380a484c9d9b978e07d0348ab96ef644a67153210210b851eee9bbbe87d6627e328bafc59ad62737ad5733af396e4438f7bf17bbe053ae040048304502210097457f51ce13ec1a625ed67ebe463b2dc4d9f35afceb3211313273996162902d02206bb6a0b6d5694d253f903df2df23dd3d142ed43c86e0b76427563d029b3cf0b401483045022100ff1eb07f8c4cd7fcb28806ea859b753ae65acb8084bce5b955e6cff7e19cd5bb02206e5af550fe8ac7f6687cd47fb45b1a826efda7d4eb78cf113674dc69dc4dcf490169522103b56b545d08850771303ac96e94433540bc02dfe5031b70d907ce1da0accd8ab621036db07836e3f7375979e41d52ea31874732449ad107872315ab2fd7fa6fe62c7f21020584520d5daf212938800964b80c75f8ab076569f034c26f815bb298408d3f7953ae04004830450221009cb5fc46357a8c9366e86e28c4489ebe1146e27bfbae7f9d71a05c1094855d6c02204056faee61634f74f67a8c8c6bb5225bce20389592edfd3e779964e8af68c61d0147304402206485282294382cd6d216a452cc258c0911b1fb95c9039d0bb224abc29645560502205551a023fbfcc8b16c06209de2fafde003082a9c9488b23ed9cfbd38acc5bb60016952210290ac42aea9bd3d64d1942f2cd0bd038926ec53986febbbd678539173bc42c6322102027bcee985369820a1cb3bab34193f69017698d2ea840aadc97df2cca89da1652103d1621540a2d3f06f97e5847a013691262c813c065516ee227e36877c7473f1d353ae040047304402203eeed130766d7b246e6c6fda838b32e00e299060b72658bc755fcdea34f62a420220559641422483f45bcffd65525b2748e7fa32790c771ce81fd72a26ae23f67fff014730440220795b6787599f62aefe46a4b78fdcff0a3bf986fc5f26e304f116f3f0395b0ce8022008aa5609c7e77725782118bd352f1a8a5477e69763a473a108306b0471146b5601695221021c7c9089bb4e229f3fe060fb9ceff90e94d21673ea52f6e56fabf87bf9e72efa2102eec0ddf7337fd386c1559f0abb42b578654046b20719b83735a5d0cfa3fdcf4c21021bcfd7e66c9b741700e0e91a709e5a4dc6c35f8c56b8a7dce1219c5e1ff155b753ae04004730440220273a8b7909fd8c88158b616ebbdf3826372eb0b10ac524e28a3c7fb2b3beb76702202a355ebc216780973d6dc8a42442542549733ec15d5b6062dc8f0a3685a5fefc01483045022100c4ac5cfd40adf138595895fb6ace7dbc8aa5b0c09257629eb3f0dba3c8f14cca02200dc43d8876315bd4d9b2eac6556b62acd3ed5a98653ea610b6ca7221c51e1976016952210217909bd8ac387b930540298e00327a5eadf273db1114da5595ae3907995ded7121030e57a914aa195703684feab47a8b2a34c93a7c4ea08336c7647b822a148b45712103af585af0b06fee0ccff8a209cd40e95aa0ee7257cd439578b9f9d595019319fb53ae04004730440220380c5f1ee526b501f0316c9c2670f04f5703f5929050e26cb15b15dabf42e45d0220509f59366b03cc9c7f953417395899269c8d8b2922e4adbf53c2b2e10bd9a09101473044022029ce79e35ef197f9869eafcfe796779a6716a3e078751748817debdd647e312602201f3d607c4e58c8482c2ef360550bb66c94f13464527356356bf757060b902f9b016952210216d4e76489489d502881162f1007dabd265664397a1e0fca7754093b6fdc78192102039cb8dac5fb4d2f98bca1ea42d19cd98b1d9fbe035448edee017e40bdca1809210258b61633db0d0a23f8f39c6716ebbe83366199508095b38d5020560a9a8c1e2253ae040047304402206d1427d6f30b857f0ca5aa11685df006897ebf72d0ecdcd416c6e3c7504666520220085dbfcc388bbfec00758af5319bf85455e6670e06f4e419fafdd75daff4df750147304402207e8e8bf530d5a7538e1aaf78a648d1db155e15fa2166e18a9c503314401a31f202200a8458033e842118260fc1555c4e2590f626a75e90b2a48064b88944b683c6da0169522102f4b9283c794cd6bbf042d05739db267792bfcb32fac521878f92f96441439b93210395132fc3205f23b37e15a809c918bb0f3c1e8676dc300de57d0854d77323963921021df66e37534c507cc27805dc687fb975ed16f2f6a82ba3775f512b61d5766b1053ae040048304502210082f563d524eca23f3a0967f7bd1836bc46c0468d30c8cd8677722e101671553f02204542613f80e2b3f7ec243d58a4d42a41d320a3350d0853ff89a2b54f58e7303a01473044022030a6866508d0079c1ce337970aa3f284337f746ee37f0885366cd4a91e008fe5022029f3772edcaf5969bf1e7dbe1c5555ac8a7908e8f4bd6145c65d70b29f6f6a610169522103145cecae7c2970a5531c223b4ce1d6e45a2881963322f233cc68bc507952584a2103adb2bd1445e2a4d7b6ae5e48c2a37bedd0e8b14ae313f5562cb05e553216078621035d8a7beffcb5adf7cb02b8adfacfda2a29b93d8b4806a7d401f006caf29c24e153ae040047304402202b6ba7466326310c2a42b5b348eda7ce83bf915984ba2d4061c73b25b4f82fae02201a129638875d256aded75b50f0a5594ab4ec6e3f27256f2619767dc60dc7f9bc014730440220355696d3d02143b8d7237fd7f89de96b9686630db187a6ca8d7af64881e367ef022073d4708f7534b347a49a44dc36fe78402e8ab9206671fe7745cddfe21fc054f10169522103bb78ac9c048bda193a334aead10085c4e882a4bd1d06d380fca9226299b62000210295f71198d5c7cf10733bea0739fba593cd5150c7c8252cd6d975b87be2b6b3492103d800faa4eef24389e5a044840687dc2d64751cf653d6dd0cb7692c7db40c799c53ae040047304402201fbb4f3d2a5e4ffa20cfb2b689e20e4fc4ebbff239adcd116a335c2270e1d37a02203b8bde5c5277df900711ea5cd66d4432d3343535d46ae998100175761dd827a40147304402202cfabcb254ec2f6aa73e4ca779f86be1308f84cb952d125e47ac00341a802c7602204671bab38e7f46ba5286a1a942536d8e7de2f5b1705d0a001cf60d5bfac6600501695221021dd7362c47229cd379774e0d9661b7074609d4d40cd796dd7fb67724e2611a322103680684fed57e67cddc340e125f4d90106f628e91705eeb0a88ac7de06217d7422102d05e9f92ed117fd0a48c02f7fec3fe01e19b458aae8c5407796063a2617e6e7853ae040047304402201f8409af3cee9498c9c759bd357dbdb1024cf09a6bf6f1ca54679ae7d574eb4402203126af2152978bd86f791852f6623f851b9359ac0e253c5005ba0390e74ddee90147304402206c391b17afd1b41edd55e69cb5013d282204374e83d6434b32d5de60ac507aaa02200950cab0d0623573f8e4dfb8c4a8a87dddcf1b94d650b5b94cb743b1eca9ffd4016952210349d4e115c685dfd6c597eb3cdf7beff83632a4879adc5a7d0a806c2ef7c48f6b2103a381606dce4ded6487b47ce132b0e5ef883dc348269ce1232e350af2e77045722103ac7216cdfc0269ede9b3a07d8e9516ce556cd3e5b567604bb855c0f3d563987853ae04004730440220411846f592ce2c27b1b933f044ac5a536d399bfe1b3e3d20e9b35715d3bd13430220662ee33c18c9749887a93e0ca8ad011540eb86209b74bf59b439b524db9df4a1014830450221009eb44a39cd7283e53d501644872078e54db6790cc3f83c461e3e6d8a39c4f22d02207846c605bed9f0c54e2247b430ad9b3f8734f4ba3b1e1b36f4d2c847820861d301695221026b9b57f4cb4b985abd47edb785b62e6cf5807906fe16bca5072d258c56be4c912102b58dfe92ebf05872bff71488626508348bdafca8a0a5fdb26910ad6554e519702102877662da59be84a917eed0186a11abbeb2c9472010c194d36b82ad0d0b19ab2a53ae040048304502210099701362a334c5c835b5e02c0e9c85c1159fb73b82b70cbf2f736cd81752d86102202927e4bfa13e1546e674b4670d0a29e7aedaad197cfcec740fbf4cbec4edc25d01483045022100e65422a8ef80b85e9825fb501550349bc450b28ca7ea117408e5d087f32da0a802205054fa80e5422959972167631ef7d82bff765ec713c3cc3aa4667e913caf7c4e016952210375993eee396cf2afe516ac73742a1018157ad0cdb31ce00f7154215074dc6f7921038147edb94d0aa3aabcf9f7d4372a0b31baeae440f718efceab66df12cebf4ab92102d10f9ede3727b98d905a76b98f5e270704f0e284ed1421f34b968198bcc39a5853ae0400483045022100b6687d790c54c82932119d18ff2c0b86e89a4dee4e6094bf7c35efb80d1e180802207e4db5f4ada27e2d42e735ba37f620a619a9d0767d590f801e73c6fc34a3cb120147304402205cd4a157de2024f67d82216cc70643902225b8295082396712f801a882a4153f022066974711830957a6298a9d61fe8913fdf0ece8b750df1ce953015de045429b5d01695221026b9b57f4cb4b985abd47edb785b62e6cf5807906fe16bca5072d258c56be4c912102b58dfe92ebf05872bff71488626508348bdafca8a0a5fdb26910ad6554e519702102877662da59be84a917eed0186a11abbeb2c9472010c194d36b82ad0d0b19ab2a53ae0400483045022100a3841ce5e1f8adbef539ec8c74a2075890eeb0fda48fff59d482e6455ce8effd022068075d9f665e3ccc85f8225c7a353760ef247a071a80e19775945aa2033ae2d901483045022100d8c6db8ceb507809e1a7c9df1eacca7dbb8fdfe006cc5efda0140faff96d2ff202201af65821451cf71955fa09aa0a9d21c58a514d045affd27a729914a76426dfea0169522103ee47fabfb677394cf29074bfede928aacd5e7e45f3c5c394cc9616d6ec00720221031da5dc5d607437f5e29bf38c21ff4a26b34a8ad46d8c61ef4b2efd157f4c37672102aeb02b501d59881f698f2dc4959574974e721798250e76adbf8ed1fe3082870f53ae0400483045022100a559171c5576d1cbfc1fdf1340525b14f65c2777aac67033fcafa6f8bb3dcd8c022007b2e6212d8c066d72d644ebabbd8ab8752989eac1cdd79b7d042cce8932cb940147304402204c956720ab8094e4df4a17f72852335218c25922ec7b38a2edd73cf127ea8db102206a9d0e9b8e46819fff88b0c99c730528a2d2d7abd00d5063e465a0a39257b21a01695221038bcf1db1256ce2003514585f8a5eabcb81b51f019d45a7cef61b47d613eca67e210395081e232d43a55f64a4756c63918770dc87d044b4fc0e955b2a0eed022cc0c3210292026f44c60df0c04d8f5ef07b3743c9141a909fb25e1514266a3049356a759353ae04004830450221009431232e59c981e1adea2d50e2cce24c4d30ca5633a9e9f616a70514de16c55a0220223b716ae59676cf167c00ad2c96c12bc04ed7fcb90901df88fa3325d9adff9b01473044022003eec62fc52141c433169ed5c4c40f77cba61712a7d83de7102bfcf0c87d835102203b873895f04d07857c9b53fc10cd4b9069cf0c897705c69bd326155f41e3749901695221030d70ffba8efb59dd6cff146e4fd8ab4f648b2129e164d2bc7d1aacf9cfcbda5a21026f4b48115dd8a2e3d39cd187e35b2d882186eb158108e999bc1a18165113d7bd21029bfadb549a440d291eae3cee9475fe512cb3c2ff75b2b1e8ccf075954c84617753ae04004830450221009c60c2c0f4b82b73e08a5b7f8ae6de1396dcb6fb63ca42fbd605552718f5dd3102204905d1db89246e547a7f1770919253582b529e34212f17e273e363546fa394c401483045022100da96217bf90cab02ef7fb784f9ca6488b3f74f2840a854dc95e9dddd6c53a50a022060bc7e8f9996082347f1ac89c3b895fa88ec7c08c3798f68323829328359711001695221033e6fca23b94eceab466ec2ca0ae0c795e9c9675d10f8540a52244bf8e5cf31e921025afc6ec717c8783cdd582445425b63e8e48c2de10bb8b4b91244e3e907d7aeea2103e6b85441a22ae8a4dc849f181a5a9e34759d7a6c8054b5ce0cb4c6b47c46bd8c53ae040047304402202afebf4dbfc9fcd947849c1a5a02322032288c314df2cc7ae6e848768eba307402203ccdfcdc3e79ba5f8f22841ea1d8017aba03bbc5524c76eb45bb6c6baf6c2ce101473044022017748d12a2f02a1596a909e0baeaa4420dccd62432397fef1f90b54d1e7a9ec302200ec08423131b39a75fad7bb6f959c888e0062965c2c035b3c6880080eb64e47501695221026c5da632ecf62a60b3ab276e79853524617eda13ec39967f2b77cf2b654a213a210279cb90a0c9f72388439b90d3c167e3acad46a6ed66e802dd9952d5b76425f89b2103a51bc5b3fcf697adc133cc20af79e8e15c3b81ed84536f90ecfa51f96cb88b1a53ae0400483045022100a645834356d3c2745c7266af9ccba4042f70a115ef51a6fde1c241b576de5d5902206ee3d157cb67f5dfe1c09ae88a41a1240f19921fcb0cdf497bab6551c007c22d0147304402200347900fba88be8812c93337d1d18d6c5183d0ed728aaca4211c739c5970c9e302202136639a16169e922b03a632d1bc3dec45db65bc1f113c575a0f65919ea3facf01695221027313f706e3b2965e5d16d0f5f7e3fe3e7085ca3df7f41d019d8e65d7b1f8c1152103cdc1ba09c17e07e066ea513e65a6e996cbc862bd872b1244d3ba5c47a9677b77210395b4629ad28463a5349b77bdeff048aa49dffb01713d18c2b6d0a47b7b6d3caf53ae0400483045022100d3211bf94aeac2526741438147b32e1cb74eccf1fd1302f5079c9e69097706db02205558b24e98cb9b373cd58a533a321743740bf8d14020d3e5f41578dc1f33bbe4014830450221009baf9e8a93de8fdf5de72fbf7a55eb6901479ef30e884a92ee424f234b16c5cd02206f13aa4eb644fa2beec30576f3edb9b418c5277ea47f3cf721e53e411e3ea6e601695221020ef0a45343575bfd627985cc5d5cc943714de69bdc2986a1fa3a19121601b02a2103103fc9cc74cc395f79493539b3974f3b6e9ac7e9ae3c3441e109319dec9fb24921036d6bc64b7d38ea420529a4357d4ddda48ed0812c07725057aec69cbd172df50053ae0400483045022100f43905d7a4291fffe8c5f6efccb77328b3add22fc4dd940401b68c4ce5a251ef02201f8f5c58fb51db9ee8a96dd79edcf227ac37f64750f62602b3af8b773d53be50014730440220411f4a4459d228c37e23c8654ee2de6addd0d5d3b5f60fefc5d816f9dace64b802206116f599bb602bc2c425a614ded0b07a235d2c7078d96eacd083cfd70e93168201695221031b44ebfa5a3817873af258d0f973bc131821aeb17827e9353285231f728d395e210200a452a36619569c8e96fc6581245b7b614b93f3f630625e66cd72533df1890e21030bf1d7e9252e1875471704003f73de768b75565c224bce3ca0d3849fa9ad34ac53ae0400473044022038e6c979ad6d2d2b9c365e42748a7ea560cc7e76c726125ccdb31b8ceb87179b02207cfd7f2fd07fb4905891ca989381837733d273ef8eb36436e33b87a4e9ea65b501483045022100c88871c8b02f386b5439a3b2fbb473c756ea4fe42760b83e98c7cf9c2e81455d02200a6fde25b3e60fe904a11ffe9fdfcee944515bfedb240848da2289decd70e8c00169522103ead1ae32913255ee346a2dc10f02d3f5914bf71204f7f8530bdec03b79e750522102fd8e770fb0579f5dd6da8d8952fd805a85acaa0b0a19d671854c141c6d4881bc210292122ae57d5f49b43e509b1d0ed6fc16153d6241e9f0e2b3524c6d89a57097cc53ae0400473044022047e46dad1a43e3c5c4b73b5c93de37aa44300a3ac01964cdd37018f2acf13b28022067a7d0d8a6511b474009eb8a85cc34985b04c93a33486ca4bd3270389ebb502001473044022050a3e5c9ec42d29c1ec1abf5a02046cdea60dff97908cebabfdec1d216df28eb02200c76a53e00947320503cd348635b090282a809d1a7dfd374fcd2a69451dd0e0c0169522103a1a822de89a78e01ff9ea6fa6b71f024951c5785a0c4cf6ab5fc5b6325860bb121023edb375d36f6f03e0b4633d60d7a9ce297086d7c33b3a2270d1f2dd9c3dc6b9221025c1982f4626d1b7fa822a0982c0601f8dcae535fe68599e944ce2ec74678491a53ae0400483045022100f0189f8b5991c7d0b32166391f299f2a8a030123a88f45aed35d2b81e156afab0220508f78b9c9f88da6e7dae6e9138414d407f1b6635355b2ce7fb8226bba96520a01463043021f69aa7cad8a007c97df1c1ac7a3d5bc8fe853cc41ef76fd1d5127cb2d6e97970220180525f6fdd7eeb241a950ef6705587a6ec871b2c18943987eddb653b40e7f39016952210278e502d0db464b885e8de8c9bbb851c0d5525f437667d9e637f998378c9222de2102933b5318702ed366d6b233606b420d987350f03a0e74fcd855846529b18faa6e21026c60ce3b6b1c18be343bc02fd71fd46d68fe85f52f2b24a19e5d889f9866fe8153ae0400483045022100d8b7451018ad390938bd022f3bc9b3b70ec10cefafd61c10e04a5c868c02ba5b02200c9671b54890a317c1798b337e43a11e97b84e2783b0341295f1aeec84aa0d1601483045022100b80b44ad6e6df3dc0ec31f113b947220983367618d00becf134dd5c0f729f99a02201f9b87d681a3bb191c9eee87f90ca44425696635fc2e540cfd12ad9245b7f7980169522102c17bb8c318c05c4e43b188e990f75d213267571dbb270809ec0b13526d3392dc2103d653d821e447560e6152a9a49eee8acd0069f6a9e8a9fa66483b2f86fc4fcce52103233189cf66b80da19f85fb87f34f8cd292e6ec663201d14092132421a921128253ae04004830450221008da175c0ee4583df9524fa26612c6565646ef4333d097b226110a1239dc4be4802203f72f2732f83c5681f7a5cedc047821f4c634bf7ea5dfdf38bff043fdaed569a014830450221009bcd7bdb79ba15bb0bddfe586e561e60cf876891438623653524e228c972f10c02205a566a272af9f265f333d8bc7152e354d9dc5d0a8c535bfcf3b91ff1c456a123016952210353276b78341cb3410fede12d17ee37f62d9b54ab51a4301de1779002225e0dfe21024b461b66792dd20248cacf0a4db427226da55d98ea7529d147266d70d77ee9d62102ab540e890d65c879b2d5ba56461ddaa61a51ece8b8255b7e76d502558cf5610153ae0400483045022100bebeca078cf5af174ce50be735ea0bc7f725a46210708aa1f41e6efde4fdccc60220672a42cd009b00caef405ee46588581717deac537e01275d2d6046e960d057d2014730440220615765e92c13731b0b85b5b380337ed0f17799d01d4e2996fafa9914f8f4226d02206b77c4301e30426307b1802b1f091c0348874b49d84dcef675ba4118085a62210169522103ceb1b4f8f5cfc098b404bffea8a5d0f3e94cec8aa0cfae51580a5679c45b370e210315d2968671edd4e5329c8cebd0faf3cdba841920269bea051228d3d2981ad9de21020f9017fe51ae427a9efee364b55ac4bac6ab2186705b3d106800ecfc96e9732853ae0400483045022100a546c5d1c1a16bab8f8306ae8988b062a84c54afcdb425fd0eaa276f680e9b8502205da5c3e64e4cd15f05525c3ed3caeed765dfcf6f590f55de1c38d4ccc7b37492014830450221009cb93b2ab4e0803d8f4204956179a0402068b3b209f9e00fcb82506ebfdaf09202201f7157a6941413eacdd24838a46ccc681bd94321fc123fdd0ed1a7a911df62fe016952210386b6d258706d8550cd55c529f950ad3eb66304f7fe93e2f9b498d2b5b48bffb32102c666ba29713c2a97faeae3f99d2dd0388c318f72802a71f24a35287e3f7571c62103202ed0a8cdf58bbac0b4753b7c9a92d0233801ecf4865364d398c6eb165f944353ae040047304402201820c3a8bfbf7ec0879f0f1a87733e94622105ab0d03bdd6208942e3aac9412b02206d0f1e2e64776492b056481ccc47b2a8707c0b0f8a2c3f050d1c03ffb3f2f8030147304402206b2a19f35d497bfb85e1ee8ada6650bed3f03b1a7138394bbed6e8dc616abc9202204d8f385364b74d9f1ffeb45e31ce773ffed9427f299dd09ed6a69f50263f5ec20169522102fd88cbefda6a92510a1712be39c62a02912fbc3537bae593abfce175b05b4e342102516d79c8a2297d1e1e773f9da2d1d90c4daa29e2dabb83cf6e8eafd64522bc512103896981996969c89c4e4e8b7978a376fe40b72732bdbd3f5bb0ac6417ed3d294f53ae0400483045022100a976afeb01b0072d547a9c5328e8403555a46df4b797d93a5097d71337d103d302203a1fc8408b5ef592639f853dd0f2b4347a7fa760136a894ac5b0bde49992671301483045022100db40dcb700666045701a037827958f32a63dbf34913564bfd790cc2d72aacb1902202a8098717e94de6b1110196e80a6b8f70a4bf07e30c9a8f2b46bc2aebc2b37df0169522103f8ef745fb6877dc637d265bba3ef6b1c8c439d2c7f8cba28292aad7aa234bacc2103ff06d24470765db46ac939a19327fd997abdab5eb9c78caf09de38dae882578c2103134a27e30e6fb289fe1c95bcbf22db3ebf99a59cc069d2869ff44c97a288207953ae0400483045022100d50e0ad0af525d412f00e81e786941d98262cb539c8582fdff02e4572f29f20f02207129385fbd7891f8577d7c70f65693a570f0fe3ba0587e0ad4dbccc7af764cba0147304402205d5f84dd9ea9c397e5dbc17400f2f6d1546d05a29df42f06c46377c53b298ce702202a75216ba38832c0e369202a6300937af300cf953b576b22e092b6e29344ce620169522102de12c0fdb78b3c5a994185ae78e7198890efc9a48a1e138e487b58e151511f5b210361ca81ce016fd3f5842139f57dc33e3f95997201e5b5c002a22c91709239367721021f6a7d820ccf6d0bc3cd72907430c41642a46004171936f90f17d1c8916f4ecd53ae0400483045022100cecfd439d4bdab9c8e42b62b302fcb6dd4955c84ed1f7ca3e93085a33251a9b8022073f982653d69bdae5b37fb114ac7a10d45a698a5b2b072d8c8bf420df0f3ee7501483045022100c5968101c05e4146a842b3cc3b71bc6723a15bd608b256118e006887793ecc3802207cbe0ce6a4f20b70200f6c623ba6205582231808e66fd378cc936778a1a0e29d0169522103cdf154821e80352e8c8fa1e6901780ca4c080aa44c55530be56985ec63b2107521033a01c2fe2c3c7fee160750c759bd1b1e750d94f254479271adcf52f084a7149b210204b7563c17e1818843490c68cb0acef9a7a5748e733581ea08145602b4a2f21153ae040047304402201d81115f183e4eef22fb76a298bf160466f26cb687d1992895d05c839ee9a70202204d16adc9672c2bdcc3b7001dd5c10293f32e44b9d2baf065c040240932fd753f0147304402203b31bc39a412ceecf99ea52612f21700f329c9213920eaecdf95cdfd802214ae02205ae4052cedf06218643525a243581740748bcfa6ccf8c73edb6d224669291b0a01695221038846549a7fdbd5ab03f071adaa13f77d1d8cc65dcab76c7a476836ff8f61754c2102e0547abec1f2e7b2fb1288661b6dd8dbd51845113949f067c12100606ce575db2103c3326d493128a2d27532335a3f7b716f601425de87c1c25d0bd4ee1038d7b83953ae0400483045022100a5418abb1da77b7ffdf1750ed9dab00110c357ad9c8f3ce6f4463a315111d142022053c430aff9733e6d8786993a006bdc5f04a741f21658da6e4b47991ea669c4b801483045022100a90ede6004b769615c6d420fc5515c152c42a9dec447ffe203feb6ed02a2f73a02206683da54271fd5495df7254358717c7de97cac429dcbcd563f63b2b7de7f8b540169522102b7724dc01b5c214768cf416d66dcfce0141608a3fe58e5c1039473ada7812ff62102c2e5b3708f1247125974fa401ec9bf0c4b783bceca51a1c5529e76a896f0008b210262cae0b5434d6fa87911cef12ed4f03f0bdf0781cef7cbf6af1b010639a7dee153ae0400483045022100be709f6d01bd54aa382cf6def52b7f64eeb8940a7048b7eafc844cf212c1a10602204f21e101b225d4b73af826fafe5b0eb88b49dc23acb52e8043c41eaec0e5aed60147304402202c649d66692fc9dabc02946a72c4c1f44f732d64d90f386015f753c97e8a9093022007e53cccdc70b76a80d727f8eccb9640a81964db186a77052257abe5a72486270169522102bf892f5eeafe4c1825f815f126ebf6b427e3e95240f581f00977a23638f014b42103d6bf1e4403ac1be80608a6316b3f8405c3f35af0c6b658b187937525d5b2c39421033b6df299f0d84f4fe67d59ecb2bf157289fcec972a0bd62992537028129b6ef453ae00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2WSHv0(
            H256::from_str(&"278e2f901256e2a7bab9071cea41da7b392c157aa50e70cae90f5e2a50c49e8d").unwrap(),
        );

        let extr_address = transaction.inputs[0].extract_address().unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn extract_witness_address_native_p2ms_output() {
        // source: https://www.blockstream.info/testnet/tx/219a49b6a376e8f4ef86866e93483552679b5157318f0e4085430a3cee24e3d8?expand
        let raw_tx = "010000000125314e40cfc816ae562c10cc1855df21ff2ed2fad43046a4b6dabbb35c393c20000000006a47304402200cd7aa9166960f3374bf655a5c5ba0a47801ae22f8231baa2412e8f47941792e02206b21c44642887b32fd87fb82a052363605c109b31d971ce502322e8148caf1670121023f3b8d04b9fac2ac10b8b8e7a4d5f033f259d26a74d2b0b77313f41585b3d1b5ffffffff0160e18709000000006952210218597441c292cb6d73174c1662ac9d60b76688fd359f90e2d653d1a089c9aba921022bda026d6aee8133f0290449a282f8cfbccafdc064b0b47068854457f38af3bc21030a230982d9706247d5997df1aea7144266c33a2e6c64c6a3a44c5cdf9c0ff58a53ae00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        assert_err!(transaction.outputs[0].extract_address(), Error::InvalidBtcAddress);
    }

    #[test]
    fn extract_witness_address_native_p2wpkh() {
        // source: https://blockstream.info/tx/6bbb103d030be8b5650459d02a13b4395a0360508f181c0f2de1c5242b416b53
        let raw_tx = "02000000000101293755d15929311f39fee17a1af3b89ad2551f11a53c4c77f21e345e74239b620000000000ffffffff0192c10200000000001976a91432006d53962f35df7f8a8571526b7704fb12ea7388ac02483045022100fad65fe2a89c319dff4e2dca528393bc13bbcff4bd0dacde324c2ccc22118522022030728ac74478cbe0e0ac3077dfe06463d3c445bc3e69507adb3fa21ce1856f9d012102f82c46833e6bb32ccdee9639cd8b84b9a864c7ef25e199bc0b0fe172e9861b7600000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2WPKHv0(H160::from_str(&"d1027c73e4f3a1f4c848930c157de972b9089330").unwrap());

        let extr_address = transaction.inputs[0].extract_address().unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn extract_witness_address_p2sh_p2wpkh() {
        let raw_tx = "01000000000101db6b1b20aa0fd7b23880be2ecbd4a98130974cf4748fb66092ac4d3ceb1a5477010000001716001479091972186c449eb1ded22b78e40d009bdf0089feffffff02b8b4eb0b000000001976a914a457b684d7f0d539a46a45bbc043f35b59d0d96388ac0008af2f000000001976a914fd270b1ee6abcaea97fea7ad0402e8bd8ad6d77c88ac02473044022047ac8e878352d3ebbde1c94ce3a10d057c24175747116f8288e5d794d12d482f0220217f36a485cae903c713331d877c1f64677e3622ad4010726870540656fe9dcb012103ad1d8e89212f0b92c74d23bb710c00662ad1470198ac48c43f7d6f93a2a2687392040000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2SH(H160::from_str(&"4733f37cf4db86fbc2efed2500b4f4e49f312023").unwrap());

        let extr_address = transaction.inputs[0].extract_address().unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn extract_witness_address_p2wsh_input() {
        // 7554ff97e5a0d879eb5f81195919b1ae46183cf804ed222cc27acabb76ecad9c (1583549)
        let raw_tx = "01000000000101fcb9d97fc77e4a5645df64b03c493f6117f46a58b2f58593ba3d4bfdc31266f90200000000ffffffff01b88201000000000017a914a89aec4cd53e6d74215332459b7fea3ec4aca975870248304502210097096b8b05e5979a738949c6f332bc35d279da0c19b760beb225e27d41f5af5802202dd4004158e2d372b0c076376a9b9033ebb5589ff9c8e129f0dbb8c80e4d5ec30123210279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798ac00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2WSHv0(H256([
            24, 99, 20, 60, 20, 197, 22, 104, 4, 189, 25, 32, 51, 86, 218, 19, 108, 152, 86, 120, 205, 77, 39, 161,
            184, 198, 50, 150, 4, 144, 50, 98,
        ]));

        let extr_address = transaction.inputs[0].extract_address().unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn extract_witness_address_p2wsh_output() {
        // d2853110a8b1dc1f670b0fc3bb8441b2a9e94ede13751a08e788da2250d938fa (1717580)
        let raw_tx = "02000000000101f46a33da6e2488101516a1087b755d9523cf13c26b7038782ed2b6334789d61d010000001716001428e31af3bbf39bb5137efb54fb0c4843f20fde47ffffffff02e8030000000000002200201863143c14c5166804bd19203356da136c985678cd4d27a1b8c63296049032623ec057020000000017a91408b94c89e0dc283638716d571daefb9633c4d121870247304402205bf259e237b20ec437e53e44891599571439c4db16e656d225b850acd3871e9502201a0169eb0e925331f1018a54b55c6dd36951e494c56f3fc7ddc266a6384560ec012102ac1d49442824063855aef270fceaab850e87f897ba146a8c7ee2c9f6e78e13e900000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2WSHv0(H256([
            24, 99, 20, 60, 20, 197, 22, 104, 4, 189, 25, 32, 51, 86, 218, 19, 108, 152, 86, 120, 205, 77, 39, 161,
            184, 198, 50, 150, 4, 144, 50, 98,
        ]));

        let extr_address = transaction.outputs[0].extract_address().unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn p2pk_not_allowed() {
        // source: https://blockstream.info/tx/f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16?expand
        let raw_tx = "0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        assert_err!(transaction.inputs[0].extract_address(), Error::MalformedTransaction);
        assert_err!(transaction.outputs[0].extract_address(), Error::InvalidBtcAddress);
    }

    #[test]
    fn decode_and_generate_coinbase_transaction() {
        // testnet - 1896103
        let raw_tx = "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff2e03a7ee1c20706f6f6c2e656e6a6f79626f646965732e636f6d2031343262393163303337f72631e9f5cd76000001ffffffff025c05af00000000001600140bdd9a64240a255ee1aac57bca1df5a0f9c6a82d0000000000000000266a24aa21a9ed173684441d99dd383ca57e6a073f62694c4f7c12a158964f050b84f69ba10ec30120000000000000000000000000000000000000000000000000000000000000000000000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let expected = parse_transaction(&tx_bytes).unwrap();

        // tb1qp0we5epypgj4acd2c4au58045ruud2pd6heuee
        let address = Address::P2WPKHv0(H160::from_str("0bdd9a64240a255ee1aac57bca1df5a0f9c6a82d").unwrap());

        let input_script =
            hex::decode("20706f6f6c2e656e6a6f79626f646965732e636f6d2031343262393163303337f72631e9f5cd76000001")
                .unwrap();

        let witness_commitment =
            hex::decode("aa21a9ed173684441d99dd383ca57e6a073f62694c4f7c12a158964f050b84f69ba10ec3").unwrap();

        let actual = generate_coinbase_transaction(
            &address,
            11470172,
            1896103,
            Some(input_script),
            Some(witness_commitment),
        );

        assert_eq!(expected, actual);
    }
}
