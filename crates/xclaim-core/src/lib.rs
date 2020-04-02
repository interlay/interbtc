use frame_support::dispatch::DispatchError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    MissingExchangeRate,
    InvalidOracleSource,

    /// use only for errors which means something
    ///  going very wrong and which do not match any other error
    RuntimeError,
}

impl Error {
    pub fn message(&self) -> &'static str {
        match self {
            Error::MissingExchangeRate => "Exchange rate not set",
            Error::InvalidOracleSource => "Invalid oracle account",
            Error::RuntimeError => "Runtim error",
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
