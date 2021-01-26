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

#[cfg(not(feature = "standalone"))]
mod cumulus;

#[cfg(not(feature = "standalone"))]
pub use cumulus::*;

#[cfg(feature = "standalone")]
mod grandpa;

#[cfg(feature = "standalone")]
pub use grandpa::*;
