use frame_support::{
    traits::OnRuntimeUpgrade,
    weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_ethereum::{Transaction, TransactionAction};
use sp_core::Get;
use sp_runtime::Permill;
use sp_std::marker::PhantomData;

pub mod precompiles;

/// Current approximation of the gas/s consumption (Moonbeam)
pub const GAS_PER_SECOND: u64 = 40_000_000;
/// Approximate ratio of the amount of Weight per Gas (Moonbeam)
pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND;

/// Sets the ideal block fullness to 50%.
/// If the block weight is between:
/// - 0-50% the gas fee will decrease
/// - 50-100% the gas fee will increase
pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

/// Get the "action" (call or create) of an Ethereum transaction
pub trait GetTransactionAction {
    fn action(&self) -> TransactionAction;
}

impl GetTransactionAction for Transaction {
    fn action(&self) -> TransactionAction {
        match self {
            Transaction::Legacy(transaction) => transaction.action,
            Transaction::EIP2930(transaction) => transaction.action,
            Transaction::EIP1559(transaction) => transaction.action,
        }
    }
}

/// Set the EVM chain ID based on the parachain ID
pub struct SetEvmChainId<T>(PhantomData<T>);
impl<T> OnRuntimeUpgrade for SetEvmChainId<T>
where
    T: frame_system::Config + parachain_info::Config + pallet_evm_chain_id::Config,
{
    fn on_runtime_upgrade() -> Weight {
        let para_id: u32 = parachain_info::Pallet::<T>::parachain_id().into();
        let evm_id: u64 = para_id.into();
        pallet_evm_chain_id::ChainId::<T>::put(evm_id);
        <T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, &'static str> {
        Ok(Default::default())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_: sp_std::vec::Vec<u8>) -> Result<(), &'static str> {
        Ok(())
    }
}
