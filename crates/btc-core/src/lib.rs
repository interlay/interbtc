use frame_support::dispatch::DispatchError;

#[derive(Clone, Copy)]
pub enum Error {
    AlreadyInitialized,
    NotMainChain,
    ForkPrevBlock,
    NotFork,
    InvalidForkId,
    MissingBlockHeight,
    InvalidHeaderSize,
    DuplicateBlock,
    PrevBlock,
    LowDiff,
    DiffTargetHeader,
    MalformedTxid,
    Confirmations,
    InvalidMerkleProof,
    ForkIdNotFound,
    HeaderNotFound,
    Partial,
    Invalid,
    Shutdown,
    InvalidTxid,
    InsufficientValue,
    TxFormat,
    WrongRecipient,
    InvalidOutputFormat,
    InvalidOpreturn,
    InvalidTxVersion,
    NotOpReturn,
    UnknownErrorcode,
    BlockNotFound,
    AlreadyReported,
    UnauthorizedRelayer,
    ChainCounterOverflow,
    BlockHeightOverflow,
    ChainsUnderflow,
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
