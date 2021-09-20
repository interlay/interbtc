#[derive(Debug, PartialEq)]
pub enum Error {
    MalformedMerkleProof,
    InvalidMerkleProof,
    EndOfFile,
    MalformedHeader,
<<<<<<< HEAD
    InvalidBlockVersion,
=======
    BlockHeaderVersionBelow4,
>>>>>>> a00c2f56 (feat: add fork testing from bitcoin core and update bitcoin testdata set)
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
    ArithmeticOverflow,
    ArithmeticUnderflow,
    InvalidCompact,
}
