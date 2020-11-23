use bech32::Error as Bech32Error;
use bs58::decode::Error as Base58Error;

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
    InvalidBtcHash,
    InvalidScript,
    InvalidBtcAddress,
    Base58(Base58Error),
    Bech32(Bech32Error),
    EmptyBech32Payload,
    InvalidWitnessVersion(u8),
    InvalidWitnessProgramLength(usize),
    InvalidSegWitV0ProgramLength(usize),
}

impl From<Bech32Error> for Error {
    fn from(error: Bech32Error) -> Self {
        Error::Bech32(error)
    }
}

impl From<Base58Error> for Error {
    fn from(error: Base58Error) -> Self {
        Error::Base58(error)
    }
}
