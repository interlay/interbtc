use bitcoin::{
    parser::FromLeBytes,
    types::{BlockHeader, H256Le, RawBlockHeader},
    Error,
};
use codec::{Decode, Encode};

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq, Eq, Debug)]

pub struct OpReturnPaymentData {
    pub recipient_amount: i64,
    pub op_return: Vec<u8>,
    /// (return_to_self_address, amount)
    pub return_to_self: Option((Address, i64)), 
}

pub struct RichBlockHeader<AccountId, BlockNumber> {
    pub block_hash: H256Le,
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
    // required for fault attribution
    pub account_id: AccountId,
    pub para_height: BlockNumber,
}

impl<AccountId, BlockNumber> RichBlockHeader<AccountId, BlockNumber> {
    /// Creates a new RichBlockHeader
    ///
    /// # Arguments
    ///
    /// * `raw_block_header` - 80 byte raw Bitcoin block header
    /// * `chain_ref` - chain reference
    /// * `block_height` - chain height
    /// * `account_id` - submitter
    /// * `para_height` - height of the parachain at submission
    #[allow(dead_code)]
    pub fn new(
        raw_block_header: RawBlockHeader,
        chain_ref: u32,
        block_height: u32,
        account_id: AccountId,
        para_height: BlockNumber,
    ) -> Result<Self, Error> {
        Ok(RichBlockHeader {
            block_hash: raw_block_header.hash(),
            block_header: BlockHeader::from_le_bytes(raw_block_header.as_bytes())?,
            block_height,
            chain_ref,
            account_id,
            para_height,
        })
    }
}
