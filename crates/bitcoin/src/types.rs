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
use codec::{Decode, Encode};
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
#[derive(Encode, Decode, Copy, Clone)]
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
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, Debug)]
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
        // the last element in the witness slice is either the
        // compressed public key (P2WPKH) or the redeem script (P2WSH)
        let witness_script = self.witness.last();
        witness_script.map_or(
            extract_address_hash_scriptsig(&self.script),
            extract_address_hash_witness,
        )
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
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BlockChain {
    pub chain_id: u32,
    pub start_height: u32,
    pub max_height: u32,
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy, Debug)]
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
    fn extract_witness_address_p2wpkh() {
        let raw_tx = "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2WPKHv0(H160([
            164, 180, 202, 72, 222, 11, 63, 255, 193, 84, 4, 161, 172, 220, 141, 186, 174, 34, 105, 85,
        ]));

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
