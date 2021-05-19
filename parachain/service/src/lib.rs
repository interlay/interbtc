//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use btc_parachain_runtime::{opaque::Block, RuntimeApi};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_service::{TFullBackend, TFullClient};

// Native executor instance.
native_executor_instance!(
    pub Executor,
    btc_parachain_runtime::api::dispatch,
    btc_parachain_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

pub type FullClient = TFullClient<Block, RuntimeApi, Executor>;
pub type FullBackend = TFullBackend<Block>;

#[cfg(feature = "cumulus-polkadot")]
mod cumulus;

#[cfg(feature = "cumulus-polkadot")]
pub use cumulus::*;

#[cfg(feature = "aura-grandpa")]
mod grandpa;

#[cfg(feature = "aura-grandpa")]
pub use grandpa::*;

#[cfg(feature = "instant-seal")]
mod instant_seal;

#[cfg(feature = "instant-seal")]
pub use instant_seal::*;
