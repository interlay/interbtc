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
    Confirmations, // TODO: rename to self-explanatory
    InsufficientStableConfirmations,
    InvalidMerkleProof,
    Invalid,
    Shutdown,
    InvalidTxid,
    InsufficientValue,
    TxFormat,
    WrongRecipient,
    InvalidOutputFormat, // not in spec
    InvalidOpreturn,
    InvalidTxVersion,
    NotOpReturn,
    UnknownErrorcode,
    ForkIdNotFound, // not in spec
    BlockNotFound, // not in spec
    AlreadyReported, // not in spec
    UnauthorizedRelayer, // not in spec
    ChainCounterOverflow, // not in spec
    BlockHeightOverflow, // not in spec
    ChainsUnderflow, // not in spec
}

impl Error {
    pub fn message(&self) -> &'static str {
        match self {
            Error::AlreadyInitialized => "Already initialized",
            Error::InvalidHeaderSize => "Invalid block header size",
            Error::DuplicateBlock => "Block already stored", 
            Error::PrevBlock => "Previous block hash not found", 
            Error::LowDiff => "PoW hash does not meet difficulty target of header",
            Error::DiffTargetHeader => "Incorrect difficulty target specified in block header",
            Error::MalformedTxid => "Malformed transaction identifier", 
            Error::Confirmations => "Transaction has less confirmations than requested",
            Error::InsufficientStableConfirmations => "Transaction has less confirmations than the global STABLE_TRANSACTION_CONFIRMATIONS parameter",
            Error::InvalidMerkleProof => "Invalid Merkle Proof",
            Error::BlockNotFound => "Block header not found for given hash",
            Error::TxFormat => "Transaction has incorrect format",
            Error::InsufficientValue => "Value of payment below requested amount",
            Error::WrongRecipient => "Incorrect recipient Bitcoin address",
            Error::InvalidTxid => "Transaction hash does not match given txid", 
            Error::InvalidOpreturn => "Incorrect identifier in OP_RETURN field",
            Error::NotOpReturn => "Expecting OP_RETURN output, but got another type",
            
            // TODO: add other error messages
            _ => "internal error",
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
