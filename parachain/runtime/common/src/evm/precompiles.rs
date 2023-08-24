use super::multi_currency::MultiCurrencyPrecompiles;
use evm_utils::*;
use pallet_evm::{IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult};
use sp_core::H160;
use sp_std::{vec, vec::Vec};

use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}

pub struct EthereumPrecompiles;
impl PartialPrecompileSet for EthereumPrecompiles {
    fn new() -> Self {
        Self
    }

    fn execute<R: pallet_evm::Config>(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Ethereum precompiles:
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: self.used_addresses().contains(&address),
            extra_cost: 0,
        }
    }

    fn used_addresses(&self) -> Vec<H160> {
        vec![hash(1), hash(2), hash(3), hash(4), hash(5)]
    }
}

pub type InterBtcPrecompiles<R> = FullPrecompileSet<R, (EthereumPrecompiles, MultiCurrencyPrecompiles<R>)>;
