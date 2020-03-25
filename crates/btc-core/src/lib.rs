use frame_support::dispatch::DispatchError;

#[derive(Clone, Copy)]
pub enum Error {
    AlreadyInitialized,
    NotMainChain,
    ForkPrevBlock,
    NotFork,
    InvalidForkId,
    MissingBlockHeight, //not in spec
    InvalidHeaderSize,
    DuplicateBlock,
    PrevBlock, // TODO: rename to self-explanatory
    LowDiff,
    DiffTargetHeader, // TODO: rename to self-explanatory
    MalformedTxid,
    Confirmations, // TODO: rename to self-explanatory
    InvalidMerkleProof,
    ForkIdNotFound,
    HeaderNotFound,
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
            Error::NotMainChain => "Main chain submission indicated, but submitted block is on a fork",
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
