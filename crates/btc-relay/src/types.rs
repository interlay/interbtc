use bitcoin::{
    parser::FromLeBytes,
    types::{BlockHeader, H256Le, RawBlockHeader},
    Error,
};
use codec::{Decode, Encode};

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RichBlockHeader<AccountId> {
    pub block_hash: H256Le,
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
    // required for fault attribution
    pub account_id: AccountId,
}

impl<AccountId> RichBlockHeader<AccountId> {
    /// Creates a new RichBlockHeader
    ///
    /// # Arguments
    ///
    /// * `raw_block_header` - 80 byte raw Bitcoin block header
    /// * `chain_ref` - chain reference
    /// * `block_height` - chain height
    /// * `account_id` - submitter
    #[allow(dead_code)]
    pub fn new(
        raw_block_header: RawBlockHeader,
        chain_ref: u32,
        block_height: u32,
        account_id: AccountId,
    ) -> Result<Self, Error> {
        Ok(RichBlockHeader {
            block_hash: raw_block_header.hash(),
            block_header: BlockHeader::from_le_bytes(raw_block_header.as_bytes())?,
            block_height,
            chain_ref,
            account_id,
        })
    }
}
