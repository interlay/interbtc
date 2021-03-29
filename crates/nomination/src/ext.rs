#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    // pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
    //     <security::Module<T>>::ensure_parachain_status_running()
    // }
}
