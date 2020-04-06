#![deny(warnings)]
use frame_support::dispatch::DispatchError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    AlreadyInitialized,
    MissingBlockHeight, //not in spec
    InvalidHeaderSize,
    DuplicateBlock,
    PrevBlock, // TODO: rename to self-explanatory
    LowDiff,
    DiffTargetHeader, // TODO: rename to self-explanatory
    MalformedTxid,
    Confirmations,                   // TODO: rename to self-explanatory
    InsufficientStableConfirmations, //not in spec
    OngoingFork,                     //not in spec
    /// Format of the proof is not correct
    MalformedMerkleProof, // not in the spec
    /// Format of the proof is correct but does not yield the correct merkle root
    InvalidMerkleProof,
    Invalid,
    Shutdown,
    InvalidTxid,
    InsufficientValue,
    MalformedTransaction, // rename ERR_TX_FORMAT
    WrongRecipient,
    InvalidOutputFormat, // not in spec
    InvalidOpreturn,
    InvalidTxVersion,
    NotOpReturn,
    UnknownErrorcode,     // not in spec
    ForkIdNotFound,       // not in spec
    BlockNotFound,        // not in spec
    AlreadyReported,      // not in spec
    UnauthorizedRelayer,  // not in spec
    ChainCounterOverflow, // not in spec
    BlockHeightOverflow,  // not in spec
    ChainsUnderflow,      // not in spec
    /// Reached EOS without finishing to parse bytes
    EOS,
    /// Malformed header
    MalformedHeader,
    /// Format of the BIP141 witness transaction output is invalid
    MalformedWitnessOutput,
    // Format of the P2PKH transaction output is invalid
    MalformedP2PKHOutput,
    // Format of the P2SH transaction output is invalid
    MalformedP2SHOutput,
    /// Format of the OP_RETURN transaction output is invalid
    MalformedOpReturnOutput,
    // Output does not match format of supported output types (Witness, P2PKH, P2SH)
    UnsupportedOutputFormat,
}

impl Error {
    pub fn message(&self) -> &'static str {
        match self {
            Error::AlreadyInitialized => "Already initialized",
            Error::MissingBlockHeight => "Missing the block at this height",
            Error::InvalidHeaderSize => "Invalid block header size",
            Error::DuplicateBlock => "Block already stored", 
            Error::PrevBlock => "Previous block hash not found", 
            Error::LowDiff => "PoW hash does not meet difficulty target of header",
            Error::DiffTargetHeader => "Incorrect difficulty target specified in block header",
            Error::MalformedTxid => "Malformed transaction identifier", 
            Error::Confirmations => "Transaction has less confirmations than requested",
            Error::InsufficientStableConfirmations => "Transaction has less confirmations than the global STABLE_TRANSACTION_CONFIRMATIONS parameter",
            Error::OngoingFork => "Current fork ongoing",
            Error::MalformedMerkleProof => "Merkle proof is malformed",
            Error::InvalidMerkleProof => "Invalid Merkle Proof",
            Error::Invalid => "BTC Parachain is halted",
            Error::Shutdown => "BTC Parachain has shut down",
            Error::InvalidTxid => "Transaction hash does not match given txid", 
            Error::InsufficientValue => "Value of payment below requested amount",
            Error::MalformedTransaction => "Transaction has incorrect format",
            Error::WrongRecipient => "Incorrect recipient Bitcoin address",
            Error::InvalidOutputFormat => "Incorrect transaction output format",
            Error::InvalidOpreturn => "Incorrect identifier in OP_RETURN field",
            Error::InvalidTxVersion => "Invalid transaction version",
            Error::NotOpReturn => "Expecting OP_RETURN output, but got another type",
            Error::UnknownErrorcode => "Error code not applicable to blocks",
            Error::ForkIdNotFound => "Blockchain with requested ID not found",
            Error::BlockNotFound => "Block header not found for given hash",
            Error::AlreadyReported => "Error code already reported", 
            Error::UnauthorizedRelayer => "Unauthorized staked relayer", 
            Error::ChainCounterOverflow => "Overflow of chain counter", 
            Error::BlockHeightOverflow => "Overflow of block height", 
            Error::ChainsUnderflow => "Underflow of stored blockchains counter", 
            Error::EOS => "EOS reached while parsing",
            Error::MalformedHeader => "Format of the header is invalid",
            Error::MalformedWitnessOutput => "Format of the witness output is invalid",
            Error::MalformedP2PKHOutput => "Format of the P2PKH output is invalid",
            Error::MalformedP2SHOutput => "Format of the P2SH output is invalid",
            Error::MalformedOpReturnOutput => "Format of the OP_RETURN transaction output is invalid",
            Error::UnsupportedOutputFormat => "Unsupported output format. Currently supported: Witness, P2PKH, P2SH,",
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        String::from(self.message())
    }
}

impl std::convert::From<Error> for DispatchError {
    fn from(error: Error) -> Self {
        DispatchError::Module {
            // FIXME: this should be set to the module returning the error
            // It should be super easy to do if Substrate has an "after request"
            // kind of middleware but not sure if it does
            index: 0,
            error: error as u8,
            message: Some(error.message()),
        }
    }
}
