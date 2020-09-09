use frame_support::dispatch::DispatchError;

#[derive(Debug)]
pub enum Error {
    MalformedMerkleProof,
    InvalidMerkleProof,
    EOS,
    MalformedHeader,
    MalformedTransaction,
    UnsupportedInputFormat,
    MalformedWitnessOutput,
    MalformedP2PKHOutput,
    MalformedP2SHOutput,
    UnsupportedOutputFormat,
    MalformedOpReturnOutput,
    InvalidHeaderSize,
    RuntimeError,
}

impl From<Error> for DispatchError {
    fn from(error: Error) -> Self {
        DispatchError::Module {
            // FIXME: this should be set to the module returning the error
            // It should be super easy to do if Substrate has an "after request"
            // kind of middleware but not sure if it does
            index: 0,
            error: error as u8,
            message: None,
        }
    }
}
