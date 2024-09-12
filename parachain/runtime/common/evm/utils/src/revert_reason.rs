extern crate alloc;

use crate::{EvmString, Writer};
use alloc::string::{String, ToString};
use frame_support::{
    dispatch::{DispatchError, PostDispatchInfo},
    sp_runtime::DispatchErrorWithPostInfo,
};
use pallet_evm::{ExitRevert, PrecompileFailure};
use sp_std::prelude::*;

const ERROR_SELECTOR: u32 = 0x08c379a0;

#[derive(Debug)]
pub enum RevertReason {
    Custom(String),
    ReadOutOfBounds { what: String },
    UnknownSelector,
    NotStart,
    CursorOverflow,
    NotSupported,
    ValueIsTooLarge,
    ReadFailed,
}

impl core::fmt::Display for RevertReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            RevertReason::Custom(s) => write!(f, "{s}"),
            RevertReason::ReadOutOfBounds { what } => {
                write!(f, "Tried to read {what} out of bounds")
            }
            RevertReason::UnknownSelector => write!(f, "Unknown selector"),
            RevertReason::NotStart => write!(f, "Could not read selector"),
            RevertReason::CursorOverflow => write!(f, "Reading cursor overflowed"),
            RevertReason::NotSupported => write!(f, "Not supported"),
            RevertReason::ValueIsTooLarge => write!(f, "Value is too large"),
            RevertReason::ReadFailed => write!(f, "Failed to read value"),
        }
    }
}

impl Into<Vec<u8>> for RevertReason {
    fn into(self) -> Vec<u8> {
        self.to_string().into()
    }
}

impl RevertReason {
    pub fn custom(s: impl Into<String>) -> Self {
        RevertReason::Custom(s.into())
    }

    pub fn read_out_of_bounds(what: impl Into<String>) -> Self {
        RevertReason::ReadOutOfBounds { what: what.into() }
    }

    pub fn to_encoded_bytes(self) -> Vec<u8> {
        let bytes: Vec<u8> = self.into();
        Writer::new()
            .write_selector(ERROR_SELECTOR)
            .write(EvmString(bytes))
            .build()
    }
}

impl From<RevertReason> for PrecompileFailure {
    fn from(err: RevertReason) -> Self {
        PrecompileFailure::Revert {
            exit_status: ExitRevert::Reverted,
            output: err.to_encoded_bytes(),
        }
    }
}

impl From<DispatchError> for RevertReason {
    fn from(err: DispatchError) -> Self {
        RevertReason::custom(alloc::format!("Runtime error: {err:?}"))
    }
}

impl From<DispatchErrorWithPostInfo<PostDispatchInfo>> for RevertReason {
    fn from(error_and_info: DispatchErrorWithPostInfo<PostDispatchInfo>) -> Self {
        RevertReason::custom(alloc::format!("Runtime error: {:?}", error_and_info.error))
    }
}
