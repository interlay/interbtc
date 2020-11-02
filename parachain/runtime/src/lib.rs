//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::StorageMapShim;
pub use module_vault_registry_rpc_runtime_api::BalanceWrapper;
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::H256;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, IdentifyAccount, IdentityLookup, NumberFor, Saturating, Verify,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use btc_relay::bitcoin;
pub use btc_relay::Call as RelayCall;
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{KeyOwnerProofSystem, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("btc-parachain"),
    impl_name: create_runtime_str!("btc-parachain"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    transaction_version: 1,
    apis: RUNTIME_API_VERSIONS,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 2 seconds of compute with a 6 second average block time.
    pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    /// Assume 10% of weight for average on_initialize calls.
    pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
        .saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
}

impl frame_system::Trait for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = ();
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Maximum weight of each block.
    type MaximumBlockWeight = MaximumBlockWeight;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// The weight of the overhead invoked on the block import process, independent of the
    /// extrinsics included in that block.
    type BlockExecutionWeight = BlockExecutionWeight;
    /// The base weight of any extrinsic processed by the runtime, independent of the
    /// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    /// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
    /// idependent of the logic of that extrinsics. (Roughly max block weight - average on
    /// initialize cost).
    type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
    /// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
    type MaximumBlockLength = MaximumBlockLength;
    /// Portion of the block weight that is available to all normal transactions.
    type AvailableBlockRatio = AvailableBlockRatio;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the frame_system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
}

impl pallet_aura::Trait for Runtime {
    type AuthorityId = AuraId;
}

impl pallet_grandpa::Trait for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProofSystem = ();

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type HandleEquivocation = ();

    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Trait for Runtime {
    type Currency = pallet_balances::Module<Runtime, pallet_balances::Instance1>;
    type OnTransactionPayment = ();
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;
}

/// DOT
impl pallet_balances::Trait<pallet_balances::Instance1> for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Runtime, pallet_balances::Instance1>,
        frame_system::CallOnCreatedAccount<Runtime>,
        frame_system::CallKillAccount<Runtime>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

/// PolkaBTC
impl pallet_balances::Trait<pallet_balances::Instance2> for Runtime {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Runtime, pallet_balances::Instance2>,
        frame_system::CallOnCreatedAccount<Runtime>,
        frame_system::CallKillAccount<Runtime>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

impl btc_relay::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

impl collateral::Trait for Runtime {
    type Event = Event;
    type DOT = pallet_balances::Module<Runtime, pallet_balances::Instance1>;
}

impl treasury::Trait for Runtime {
    type Event = Event;
    type PolkaBTC = pallet_balances::Module<Runtime, pallet_balances::Instance2>;
}

impl security::Trait for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const MaturityPeriod: u32 = 10;
    pub const MinimumDeposit: u32 = 10;
    pub const MinimumStake: u32 = 10;
    pub const MinimumParticipants: u32 = 3;
    pub const VoteThreshold: u32 = 50;
    pub const VotingPeriod: BlockNumber = DAYS;
    pub const MaximumMessageSize: u32 = 256;
}

impl staked_relayers::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
    type MaturityPeriod = MaturityPeriod;
    type MinimumDeposit = MinimumDeposit;
    type MinimumStake = MinimumStake;
    type MinimumParticipants = MinimumParticipants;
    type VoteThreshold = VoteThreshold;
    type VotingPeriod = VotingPeriod;
    type MaximumMessageSize = MaximumMessageSize;
}

impl vault_registry::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

impl exchange_rate_oracle::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use issue::IssueRequest;

impl issue::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use redeem::RedeemRequest;

impl redeem::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use replace::ReplaceRequest;

impl replace::Trait for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Aura: pallet_aura::{Module, Config<T>, Inherent},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},

        DOT: pallet_balances::<Instance1>::{Module, Call, Storage, Config<T>, Event<T>},
        PolkaBTC: pallet_balances::<Instance2>::{Module, Call, Storage, Config<T>, Event<T>},
        BTCRelay: btc_relay::{Module, Call, Config<T>, Storage, Event},
        Collateral: collateral::{Module, Call, Storage, Event<T>},
        Treasury: treasury::{Module, Call, Storage, Event<T>},
        Security: security::{Module, Call, Storage, Event},
        StakedRelayers: staked_relayers::{Module, Call, Config<T>, Storage, Event<T>},
        VaultRegistry: vault_registry::{Module, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Module, Call, Config<T>, Storage, Event<T>},
        Issue: issue::{Module, Call, Config<T>, Storage, Event<T>},
        Redeem: redeem::{Module, Call, Config<T>, Storage, Event<T>},
        Replace: replace::{Module, Call, Config<T>, Storage, Event<T>},
    }
);

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllModules,
>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            // NOTE: this is the only implementation possible since we've
            // defined our key owner proof type as a bottom type (i.e. a type
            // with no values).
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            // use frame_system_benchmarking::Module as SystemBench;
            impl frame_system_benchmarking::Trait for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, btc_relay, BTCRelay);
            add_benchmark!(params, batches, exchange_rate_oracle, ExchangeRateOracle);
            add_benchmark!(params, batches, issue, Issue);
            add_benchmark!(params, batches, redeem, Redeem);
            add_benchmark!(params, batches, replace, Replace);
            add_benchmark!(params, batches, staked_relayers, StakedRelayers);
            add_benchmark!(params, batches, vault_registry, VaultRegistry);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl module_exchange_rate_oracle_rpc_runtime_api::ExchangeRateOracleApi<
        Block,
        Balance,
        Balance,
    > for Runtime {
        fn btc_to_dots(amount: Balance) -> Result<Balance, DispatchError> {
            ExchangeRateOracle::btc_to_dots(amount)
        }
    }

    impl module_staked_relayers_rpc_runtime_api::StakedRelayersApi<
        Block,
        AccountId,
    > for Runtime {
        fn is_transaction_invalid(vault_id: AccountId, raw_tx: Vec<u8>) -> DispatchResult {
            StakedRelayers::is_transaction_invalid(&vault_id, raw_tx)
        }
    }

    impl module_vault_registry_rpc_runtime_api::VaultRegistryApi<
        Block,
        AccountId,
        Balance,
        Balance

    > for Runtime {
        fn get_first_vault_with_sufficient_collateral(amount: Balance) -> Result<AccountId, DispatchError> {
            VaultRegistry::get_first_vault_with_sufficient_collateral(amount)
        }

        fn get_first_vault_with_sufficient_tokens(amount: Balance) -> Result<AccountId, DispatchError> {
            VaultRegistry::get_first_vault_with_sufficient_tokens(amount)
        }

        fn get_issuable_tokens_from_vault(vault: AccountId) -> Result<Balance, DispatchError> {
            VaultRegistry::get_issuable_tokens_from_vault(vault)
        }

        fn get_collateralization_from_vault(vault: AccountId) -> Result<u64, DispatchError> {
            VaultRegistry::get_collateralization_from_vault(vault)
        }

        fn get_required_collateral_for_polkabtc(amount_btc: BalanceWrapper<Balance>) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_required_collateral_for_polkabtc(amount_btc.amount)?;
            Ok(BalanceWrapper{amount:result})
        }
    }

    impl module_issue_rpc_runtime_api::IssueApi<
        Block,
        AccountId,
        H256,
        IssueRequest<AccountId, BlockNumber, Balance, Balance>
    > for Runtime {
        fn get_issue_requests(account_id: AccountId) -> Vec<(H256, IssueRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Issue::get_issue_requests_for_account(account_id)
        }

        fn get_vault_issue_requests(account_id: AccountId) -> Vec<(H256, IssueRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Issue::get_issue_requests_for_vault(account_id)
        }
    }

    impl module_redeem_rpc_runtime_api::RedeemApi<
        Block,
        AccountId,
        H256,
        RedeemRequest<AccountId, BlockNumber, Balance, Balance>
    > for Runtime {
        fn get_redeem_requests(account_id: AccountId) -> Vec<(H256, RedeemRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Redeem::get_redeem_requests_for_account(account_id)
        }

        fn get_vault_redeem_requests(account_id: AccountId) -> Vec<(H256, RedeemRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Redeem::get_redeem_requests_for_vault(account_id)
        }
    }

    impl module_replace_rpc_runtime_api::ReplaceApi<
        Block,
        AccountId,
        H256,
        ReplaceRequest<AccountId, BlockNumber, Balance, Balance>
    > for Runtime {
        fn get_old_vault_replace_requests(account_id: AccountId) -> Vec<(H256, ReplaceRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Replace::get_replace_requests_for_old_vault(account_id)
        }

        fn get_new_vault_replace_requests(account_id: AccountId) -> Vec<(H256, ReplaceRequest<AccountId, BlockNumber, Balance, Balance>)> {
            Replace::get_replace_requests_for_new_vault(account_id)
        }
    }

}
