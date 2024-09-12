#![cfg_attr(not(feature = "std"), no_std)]

use pallet_evm::{ExitError, ExitSucceed, PrecompileFailure, PrecompileOutput};
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

// https://github.com/rust-blockchain/evm/blob/a33ac87ad7462b7e7029d12c385492b2a8311d1c/gasometer/src/costs.rs#L147-L163
fn log_cost_inner(topics: usize, data_len: usize) -> Option<u64> {
    const G_LOG: u64 = 375;
    const G_LOGDATA: u64 = 8;
    const G_LOGTOPIC: u64 = 375;
    let topic_cost = G_LOGTOPIC.checked_mul(topics as u64)?;
    let data_cost = G_LOGDATA.checked_mul(data_len as u64)?;
    G_LOG.checked_add(topic_cost)?.checked_add(data_cost)
}

pub fn log_cost(topics: usize, data_len: usize) -> EvmResult<u64> {
    log_cost_inner(topics, data_len).ok_or(PrecompileFailure::Error {
        exit_status: ExitError::OutOfGas,
    })
}
