use impl_trait_for_tuples::impl_for_tuples;
use pallet_evm::{IsPrecompileResult, PrecompileHandle, PrecompileResult, PrecompileSet};
use sp_core::H160;
use sp_std::{marker::PhantomData, vec, vec::Vec};

pub trait PartialPrecompileSet {
    fn new() -> Self;
    fn execute<R: pallet_evm::Config>(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult>;
    fn is_precompile(&self, address: H160, gas: u64) -> IsPrecompileResult;
    fn used_addresses(&self) -> Vec<H160>;
}

#[impl_for_tuples(1, 100)]
impl PartialPrecompileSet for Tuple {
    #[inline(always)]
    fn new() -> Self {
        (for_tuples!(#(
			Tuple::new()
		),*))
    }

    #[inline(always)]
    fn execute<R: pallet_evm::Config>(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        for_tuples!(#(
			if let Some(res) = self.Tuple.execute::<R>(handle) {
				return Some(res);
			}
		)*);

        None
    }

    #[inline(always)]
    fn is_precompile(&self, address: H160, gas: u64) -> IsPrecompileResult {
        for_tuples!(#(
			match self.Tuple.is_precompile(address, gas) {
				IsPrecompileResult::Answer {
					is_precompile: true,
					..
				} => return IsPrecompileResult::Answer {
					is_precompile: true,
					extra_cost: 0,
				},
				_ => {}
			};
		)*);
        IsPrecompileResult::Answer {
            is_precompile: false,
            extra_cost: 0,
        }
    }

    #[inline(always)]
    fn used_addresses(&self) -> Vec<H160> {
        let mut used_addresses = vec![];

        for_tuples!(#(
			let mut inner = self.Tuple.used_addresses();
			used_addresses.append(&mut inner);
		)*);

        used_addresses
    }
}

pub struct FullPrecompileSet<R, P> {
    inner: P,
    _phantom: PhantomData<R>,
}

impl<R: pallet_evm::Config, P: PartialPrecompileSet> PrecompileSet for FullPrecompileSet<R, P> {
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        self.inner.execute::<R>(handle)
    }

    fn is_precompile(&self, address: H160, gas: u64) -> IsPrecompileResult {
        self.inner.is_precompile(address, gas)
    }
}

impl<R: pallet_evm::Config, P: PartialPrecompileSet> FullPrecompileSet<R, P> {
    pub fn new() -> Self {
        Self {
            inner: P::new(),
            _phantom: PhantomData,
        }
    }

    pub fn used_addresses() -> Vec<H160> {
        Self::new().inner.used_addresses()
    }
}
