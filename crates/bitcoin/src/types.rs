extern crate hex;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::alloc::string::String;
use codec::{Decode, Encode};
use primitive_types::{H256, U256};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::convert::TryFrom;
use sp_std::prelude::*;
use x_core::Error;

use crate::formatter::Formattable;
use crate::merkle::MerkleProof;
use crate::parser::{extract_address_hash, extract_op_return_data, FromLeBytes};
use crate::utils::{hash256_merkle_step, log2, reverse_endianness, sha256d_le};

pub(crate) const SERIALIZE_TRANSACTION_NO_WITNESS: i32 = 0x4000_0000;

// Bitcoin Script OpCodes
// https://github.com/bitcoin/bitcoin/blob/master/src/script/script.h
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

    OpInvalidOpcode,
}

/// Custom Types
/// Bitcoin Raw Block Header type

#[derive(Encode, Decode, Copy, Clone)]
pub struct RawBlockHeader([u8; 80]);

impl RawBlockHeader {
    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
    pub fn from_bytes(bytes: &[u8]) -> Result<RawBlockHeader, Error> {
        if bytes.len() != 80 {
            return Err(Error::InvalidHeaderSize);
        }
        let mut result: [u8; 80] = [0; 80];
        result.copy_from_slice(&bytes);
        Ok(RawBlockHeader(result))
    }

    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
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

#[derive(Encode, Decode, PartialEq, Copy, Clone)]
pub struct Address([u8; 20]);

impl Address {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Address {
        Address(bytes)
    }
}

impl From<Address> for [u8; 20] {
    fn from(address: Address) -> [u8; 20] {
        address.0
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = x_core::Error;
    fn try_from(bytes: &[u8]) -> Result<Address, Self::Error> {
        if bytes.len() != 20 {
            return Err(Error::RuntimeError);
        }
        let mut address: [u8; 20] = Default::default();
        address.copy_from_slice(bytes);
        Ok(Address(address))
    }
}

impl TryFrom<&str> for Address {
    type Error = x_core::Error;
    fn try_from(hex_address: &str) -> Result<Address, Self::Error> {
        let bytes = hex::decode(hex_address).map_err(|_e| Error::RuntimeError)?;
        Address::try_from(&bytes[..])
    }
}

// Constants
pub const P2PKH_SCRIPT_SIZE: u32 = 25;
pub const P2SH_SCRIPT_SIZE: u32 = 23;
pub const HASH160_SIZE_HEX: u8 = 0x14;
pub const MAX_OPRETURN_SIZE: usize = 83;

/// Structs

/// Bitcoin Basic Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Debug)]
//#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader {
    pub merkle_root: H256Le,
    pub target: U256,
    pub timestamp: u32,
    pub version: i32,
    pub hash_prev_block: H256Le,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn hash(&self) -> H256Le {
        sha256d_le(&self.format())
    }
}

/// Bitcoin transaction input
#[derive(PartialEq, Clone, Debug)]
pub struct TransactionInput {
    pub previous_hash: H256Le,
    pub previous_index: u32,
    pub coinbase: bool,
    pub height: Option<Vec<u8>>, // FIXME: Vec<u8> type here seems weird
    pub script: Vec<u8>,
    pub sequence: u32,
    pub witness: Vec<Vec<u8>>,
}

impl TransactionInput {
    pub fn with_witness(&mut self, witness: Vec<Vec<u8>>) {
        self.witness = witness;
    }
}

/// Bitcoin script
#[derive(PartialEq, Debug, Clone)]
pub struct Script {
    pub(crate) bytes: Vec<u8>,
}

impl Default for Script {
    fn default() -> Self {
        Script { bytes: vec![] }
    }
}

impl Script {
    pub fn new() -> Script {
        Self::default()
    }

    pub fn height(height: u32) -> Script {
        let mut script = Script::new();
        script.append(OpCode::Op3);
        let bytes = height.to_le_bytes();
        script.append(&bytes[0..=2]);
        script
    }

    // Format:
    // 0x76 (OP_DUP) - 0xa9 (OP_HASH160) - 0x14 (20 bytes len) - <20 bytes pubkey hash> - 0x88 (OP_EQUALVERIFY) - 0xac (OP_CHECKSIG)
    pub fn p2pkh(address: &Address) -> Script {
        let mut script = Script::new();
        script.append(OpCode::OpDup);
        script.append(OpCode::OpHash160);
        script.append(HASH160_SIZE_HEX);
        script.append(address);
        script.append(OpCode::OpEqualVerify);
        script.append(OpCode::OpCheckSig);
        script
    }

    // Format:
    // 0xa9 (OP_HASH160) - 0x14 (20 bytes hash) - <20 bytes script hash> - 0x87 (OP_EQUAL)
    pub fn p2sh(address: &Address) -> Script {
        let mut script = Script::new();
        script.append(OpCode::OpHash160);
        script.append(HASH160_SIZE_HEX);
        script.append(address);
        script.append(OpCode::OpEqual);
        script
    }

    pub fn op_return(return_content: &[u8]) -> Script {
        let mut script = Script::new();
        script.append(OpCode::OpReturn);
        script.append(return_content.len() as u8);
        script.append(return_content);
        script
    }

    pub fn append<T: Formattable<U>, U>(&mut self, value: T) {
        self.bytes.extend(&value.format())
    }

    pub fn extract_address(&self) -> Result<Vec<u8>, Error> {
        extract_address_hash(&self.bytes)
    }

    pub fn extract_op_return_data(&self) -> Result<Vec<u8>, Error> {
        extract_op_return_data(&self.bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<Vec<u8>> for Script {
    fn from(bytes: Vec<u8>) -> Script {
        Script { bytes }
    }
}

impl TryFrom<&str> for Script {
    type Error = x_core::Error;

    fn try_from(hex_string: &str) -> Result<Script, Self::Error> {
        let bytes = hex::decode(hex_string).map_err(|_e| Error::RuntimeError)?;
        Ok(Script { bytes })
    }
}

/// Bitcoin transaction output
#[derive(PartialEq, Debug, Clone)]
pub struct TransactionOutput {
    pub value: i64,
    pub script: Script,
}

impl TransactionOutput {
    pub fn p2pkh(value: i64, address: &Address) -> TransactionOutput {
        TransactionOutput {
            value,
            script: Script::p2pkh(address),
        }
    }

    pub fn p2sh(value: i64, address: &Address) -> TransactionOutput {
        TransactionOutput {
            value,
            script: Script::p2sh(address),
        }
    }

    pub fn op_return(value: i64, return_content: &[u8]) -> TransactionOutput {
        TransactionOutput {
            value,
            script: Script::op_return(return_content),
        }
    }
}

/// Bitcoin transaction
#[derive(PartialEq, Debug, Clone)]
pub struct Transaction {
    pub version: i32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub block_height: Option<u32>, //FIXME: why is this optional?
    pub locktime: Option<u32>,     //FIXME: why is this optional?
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

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq, Debug)]
pub struct RichBlockHeader {
    pub block_hash: H256Le,
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
}

impl RichBlockHeader {
    // Creates a RichBlockHeader given a RawBlockHeader, Blockchain identifier and block height
    pub fn construct(
        raw_block_header: RawBlockHeader,
        chain_ref: u32,
        block_height: u32,
    ) -> Result<RichBlockHeader, Error> {
        Ok(RichBlockHeader {
            block_hash: raw_block_header.hash(),
            block_header: BlockHeader::from_le_bytes(raw_block_header.as_bytes())?,
            block_height,
            chain_ref,
        })
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn merkle_proof(&self, include: &[H256Le]) -> MerkleProof {
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

        let height = proof.compute_tree_height();
        proof.traverse_and_build(height as u32, 0, &tx_ids, &matches);
        proof
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
        Self::default()
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

    pub fn mine(&mut self, target: U256) -> Block {
        self.block.header.target = target;
        self.block.header.merkle_root = self.compute_merkle_root();
        let mut nonce: u32 = 0;
        // NOTE: this is inefficient because we are serializing the header
        // over and over again but it should not matter because
        // this is meant to be used only for very low difficulty
        // and not for any sort of real-world mining
        while self.block.header.hash().as_u256() >= target {
            self.block.header.nonce = nonce;
            nonce += 1;
        }
        self.block.clone()
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> &mut Self {
        self.block.transactions.push(transaction);
        self
    }

    // TODO: double check this works
    // the output of real-world transactions
    // seem to finish with OP_CHECKSIG
    // need to check format
    pub fn with_coinbase(&mut self, address: &Address, reward: i64, height: u32) -> &mut Self {
        let input = TransactionInputBuilder::new()
            .with_coinbase(true)
            .with_previous_index(u32::max_value())
            .with_previous_hash(H256Le::zero())
            .with_height(Script::height(height).as_bytes())
            .with_sequence(0)
            .build();
        // FIXME: this is most likely not what real-world transactions look like
        let output = TransactionOutput::p2pkh(reward, &address);
        let transaction = TransactionBuilder::new()
            .add_input(input)
            .add_output(output)
            .build();
        self.block.transactions.insert(0, transaction);
        self
    }

    fn compute_merkle_root(&self) -> H256Le {
        let height = log2(self.block.transactions.len() as u64);
        self.rec_compute_merkle_root(0, height)
    }

    fn compute_tree_width(&self, height: u8) -> usize {
        (self.block.transactions.len() as usize + (1 << height) - 1) >> height
    }

    fn rec_compute_merkle_root(&self, index: usize, height: u8) -> H256Le {
        if height == 0 {
            return self.block.transactions[index].tx_id();
        }
        let left = self.rec_compute_merkle_root(index * 2, height - 1);
        let right = if index * 2 + 1 < self.compute_tree_width(height - 1) {
            self.rec_compute_merkle_root(index * 2 + 1, height - 1)
        } else {
            left
        };
        hash256_merkle_step(&left.to_bytes_le(), &right.to_bytes_le())
    }
}

/// Representation of a Bitcoin blockchain
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BlockChain {
    pub chain_id: u32,
    pub start_height: u32,
    pub max_height: u32,
    pub no_data: BTreeSet<u32>,
    pub invalid: BTreeSet<u32>,
}

impl BlockChain {
    // Checks if there is a NO_DATA block in the BlockChain
    pub fn is_no_data(&self) -> bool {
        !self.no_data.is_empty()
    }

    // Checks if there is an INVALID block in the BlockChain
    pub fn is_invalid(&self) -> bool {
        !self.invalid.is_empty()
    }
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub struct H256Le {
    content: [u8; 32],
}

impl H256Le {
    /// Creates a new H256Le hash equals to zero
    pub fn zero() -> H256Le {
        H256Le { content: [0; 32] }
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

    pub fn from_hex_le(hex: &str) -> H256Le {
        H256Le::from_bytes_le(&hex::decode(hex).unwrap())
    }

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
    pub fn to_hex_le(&self) -> String {
        hex::encode(&self.to_bytes_le())
    }

    /// Returns the content of the H256Le encoded in big endian hex
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
        CompactUint {
            value: value as u64,
        }
    }
}

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
                block_height: Some(0),
                locktime: None,
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
        self.transaction.block_height = Some(block_height);
        self.transaction.locktime = None;
        self
    }

    pub fn with_locktime(&mut self, locktime: u32) -> &mut Self {
        self.transaction.locktime = Some(locktime);
        self.transaction.block_height = None;
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

pub struct TransactionInputBuilder {
    trasaction_input: TransactionInput,
}

impl Default for TransactionInputBuilder {
    fn default() -> Self {
        TransactionInputBuilder {
            trasaction_input: TransactionInput {
                previous_hash: H256Le::zero(),
                previous_index: 0,
                coinbase: true,
                height: None,
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

    pub fn with_previous_hash(&mut self, previous_hash: H256Le) -> &mut Self {
        self.trasaction_input.previous_hash = previous_hash;
        self
    }

    pub fn with_previous_index(&mut self, previous_index: u32) -> &mut Self {
        self.trasaction_input.previous_index = previous_index;
        self
    }

    pub fn with_coinbase(&mut self, coinbase: bool) -> &mut Self {
        self.trasaction_input.coinbase = coinbase;
        self
    }

    pub fn with_script(&mut self, script: &[u8]) -> &mut Self {
        self.trasaction_input.script = Vec::from(script);
        self
    }

    pub fn with_height(&mut self, height: &[u8]) -> &mut Self {
        self.trasaction_input.height = Some(Vec::from(height));
        self
    }

    pub fn with_sequence(&mut self, sequence: u32) -> &mut Self {
        self.trasaction_input.sequence = sequence;
        self
    }

    pub fn add_witness(&mut self, witness: &[u8]) -> &mut Self {
        self.trasaction_input.witness.push(Vec::from(witness));
        self
    }

    pub fn build(&self) -> TransactionInput {
        self.trasaction_input.clone()
    }
}

#[cfg(test)]
mod tests {
    use mocktopus::mocking::*;

    use super::*;
    use sp_std::convert::TryInto;

    use crate::parser::parse_transaction;

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
    fn test_script_height() {
        assert_eq!(Script::height(100).len(), 4);
    }

    #[test]
    fn test_transaction_input_builder() {
        let input = TransactionInputBuilder::new()
            .with_sequence(10)
            .with_previous_hash(100.into())
            .build();
        assert_eq!(input.sequence, 10);
        let mut bytes: [u8; 32] = Default::default();
        bytes[0] = 100;
        assert_eq!(input.previous_hash, H256Le::from_bytes_le(&bytes));
    }

    #[test]
    fn test_transaction_builder() {
        let address: Address = "66c7060feb882664ae62ffad0051fe843e318e85"
            .try_into()
            .unwrap();
        let return_data = hex::decode("01a0").unwrap();
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(TransactionInputBuilder::new().with_coinbase(false).build())
            .add_output(TransactionOutput::p2pkh(100, &address))
            .add_output(TransactionOutput::op_return(0, &return_data))
            .build();
        assert_eq!(transaction.version, 2);
        assert_eq!(transaction.inputs.len(), 1);
        assert_eq!(transaction.outputs.len(), 2);
        assert_eq!(transaction.outputs[0].value, 100);
        assert_eq!(
            transaction.outputs[0].script.extract_address().unwrap(),
            address.as_bytes()
        );
        assert_eq!(transaction.outputs[1].value, 0);
        assert_eq!(
            transaction.outputs[1]
                .script
                .extract_op_return_data()
                .unwrap(),
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
                1 => H256Le::from_hex_be(
                    "8c14f0db3df150123e6f3dbbf30f8b955a8249b62ac1d1ff16284aefa3d06d87",
                ),
                2 => H256Le::from_hex_be(
                    "fff2525b8931402dd09222c50775608f75787bd2b87e56995a7bdd30f79702c4",
                ),
                3 => H256Le::from_hex_be(
                    "6359f0868171b1d194cbee1af2f16ea598ae8fad666d9b012c8ed2b79a236ec4",
                ),
                4 => H256Le::from_hex_be(
                    "e9a66845e05d5abc0ad04ec80f774a7e585c6e8db975962d069a522137b80c1d",
                ),
                _ => panic!("should not happen"),
            };
            MockResult::Return(txid)
        });
        let mut builder = BlockBuilder::new();
        for tx in transactions {
            builder.add_transaction(tx);
        }
        let merkle_root = builder.compute_merkle_root();
        let expected =
            H256Le::from_hex_be("f3e94742aca4b5ef85488dc37c06c3282295ffec960994b2c0d5ac2a25a95766");
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
                1 => H256Le::from_hex_be(
                    "a335b243f5e343049fccac2cf4d70578ad705831940d3eef48360b0ea3829ed4",
                ),
                2 => H256Le::from_hex_be(
                    "d5fd11cb1fabd91c75733f4cf8ff2f91e4c0d7afa4fd132f792eacb3ef56a46c",
                ),
                3 => H256Le::from_hex_be(
                    "0441cb66ef0cbf78c9ecb3d5a7d0acf878bfdefae8a77541b3519a54df51e7fd",
                ),
                4 => H256Le::from_hex_be(
                    "1a8a27d690889b28d6cb4dacec41e354c62f40d85a7f4b2d7a54ffc736c6ff35",
                ),
                5 => H256Le::from_hex_be(
                    "1d543d550676f82bf8bf5b0cc410b16fc6fc353b2a4fd9a0d6a2312ed7338701",
                ),
                _ => panic!("should not happen"),
            };
            MockResult::Return(txid)
        });
        let mut builder = BlockBuilder::new();
        for tx in transactions {
            builder.add_transaction(tx);
        }
        let merkle_root = builder.compute_merkle_root();
        let expected =
            H256Le::from_hex_be("5766798857e436d6243b46b5c1e0af5b6806aa9c2320b3ffd4ecff7b31fd4647");
        assert_eq!(merkle_root, expected);
    }

    #[test]
    fn test_mine_block() {
        clear_mocks();
        let address: Address = "66c7060feb882664ae62ffad0051fe843e318e85"
            .try_into()
            .unwrap();
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into()));
        assert_eq!(block.header.version, 2);
        assert_eq!(block.header.merkle_root, block.transactions[0].tx_id());
        // should be 3, might change if block is changed
        assert_eq!(block.header.nonce, 3);
        assert!(block.header.nonce > 0);
    }

    #[test]
    fn test_merkle_proof() {
        clear_mocks();
        let address: Address = "66c7060feb882664ae62ffad0051fe843e318e85"
            .try_into()
            .unwrap();

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(TransactionInputBuilder::new().with_coinbase(false).build())
            .add_output(TransactionOutput::p2pkh(100, &address))
            .build();

        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        // FIXME: flag_bits incorrect
        let proof = block.merkle_proof(&vec![transaction.tx_id()]);
        let bytes = proof.format();
        MerkleProof::parse(&bytes).unwrap();
    }
}
