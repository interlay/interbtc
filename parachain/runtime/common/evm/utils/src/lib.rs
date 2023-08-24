#![cfg_attr(not(feature = "std"), no_std)]

use pallet_evm::{ExitSucceed, PrecompileFailure, PrecompileOutput};
use sp_std::prelude::*;

mod evm_codec;
mod precompile_set;
mod revert_reason;

pub use evm_codec::*;
pub use precompile_set::*;
pub use revert_reason::*;

pub type EvmResult<T = ()> = Result<T, PrecompileFailure>;
pub type MayRevert<T = ()> = Result<T, RevertReason>;

fn encode_return_data<T: EvmCodec>(value: T) -> Vec<u8> {
    Writer::new().write(value).build()
}

pub fn new_precompile_output<T: EvmCodec>(value: T) -> PrecompileOutput {
    PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: encode_return_data(value),
    }
}
