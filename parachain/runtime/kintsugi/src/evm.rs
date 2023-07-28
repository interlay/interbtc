use crate::{
    AccountId, Aura, BaseFee, EVMChainId, NativeCurrency, Runtime, RuntimeEvent, Timestamp, CENTS,
    MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
};
use frame_support::{
    parameter_types,
    traits::{ConstU32, FindAuthor},
    weights::Weight,
    ConsensusEngineId,
};
use pallet_ethereum::PostLogContent;
use pallet_evm::{EnsureAddressRoot, EnsureAddressTruncated, FixedGasWeightMapping, HashedAddressMapping};
use sp_core::{crypto::ByteArray, H160, U256};
use sp_runtime::{traits::BlakeTwo256, Permill};
use sp_std::marker::PhantomData;

pub use runtime_common::evm::{
    precompiles::InterBtcPrecompiles, BaseFeeThreshold, GetTransactionAction, SetEvmChainId, WEIGHT_PER_GAS,
};

pub type Precompiles = InterBtcPrecompiles<Runtime>;
pub type AccountConverter = HashedAddressMapping<BlakeTwo256>;

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(CENTS * 10);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

impl pallet_base_fee::Config for Runtime {
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
    // 0x96c899190652984ee67b82942ff38f43
    pub storage EnableCreate: bool = false;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let authority_id = Aura::authorities()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
        }
        None
    }
}

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
    pub PrecompilesValue: InterBtcPrecompiles<Runtime> = InterBtcPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
    /// The amount of gas per pov, taken from Moonbeam:
    /// ceil(MAXIMUM_BLOCK_WEIGHT.ref_time() / MAXIMUM_BLOCK_WEIGHT.proof_size() / WEIGHT_PER_GAS)
    pub const GasLimitPovSizeRatio: u64 = 4;
}

impl pallet_evm::Config for Runtime {
    type AddressMapping = AccountConverter;
    type BlockGasLimit = BlockGasLimit;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressTruncated;
    type ChainId = EVMChainId;
    type Currency = NativeCurrency;
    type FeeCalculator = BaseFee;
    type FindAuthor = FindAuthorTruncated<Aura>;
    type GasWeightMapping = FixedGasWeightMapping<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type PrecompilesType = InterBtcPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type RuntimeEvent = RuntimeEvent;
    type WeightPerGas = WeightPerGas;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

impl pallet_evm_chain_id::Config for Runtime {}
