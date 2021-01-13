//! Minimal Pallet that injects a ParachainId into Runtime storage from

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_module, decl_storage, traits::Get};

use cumulus_primitives::ParaId;

/// Configuration trait of this pallet.
pub trait Config: frame_system::Config {}

impl<T: Config> Get<ParaId> for Module<T> {
    fn get() -> ParaId {
        Self::parachain_id()
    }
}

decl_storage! {
    trait Store for Module<T: Config> as ParachainUpgrade {
        ParachainId get(fn parachain_id) config(): ParaId = 100.into();
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {}
}
