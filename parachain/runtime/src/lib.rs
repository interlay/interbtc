//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    traits::StorageMapShim,
};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::H256;

use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, IdentityLookup, Verify},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, ModuleId, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{KeyOwnerProofSystem, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

// PolkaBTC exports
pub use btc_relay::{bitcoin, Call as RelayCall, TARGET_SPACING};
pub use module_exchange_rate_oracle_rpc_runtime_api::BalanceWrapper;

// XCM imports
#[cfg(feature = "cumulus-polkadot")]
use {
    frame_system::EnsureRoot,
    parachain_tokens::{CurrencyAdapter, NativeAsset},
    polkadot_parachain::primitives::Sibling,
    sp_runtime::traits::Convert,
    xcm::v0::{Junction, MultiLocation, NetworkId},
    xcm_builder::{
        AccountId32Aliases, LocationInverter, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
        SiblingParachainConvertsVia, SignedAccountId32AsNative, SovereignSignedViaLocation,
    },
    xcm_executor::{Config, XcmExecutor},
};

// Aura & GRANDPA imports
#[cfg(feature = "aura-grandpa")]
use {
    pallet_grandpa::fg_primitives,
    pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList},
    sp_consensus_aura::sr25519::AuthorityId as AuraId,
    sp_core::crypto::KeyTypeId,
    sp_runtime::traits::NumberFor,
};

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
}

pub type SessionHandlers = ();

#[cfg(feature = "aura-grandpa")]
impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}

#[cfg(feature = "cumulus-polkadot")]
impl_opaque_keys! {
    pub struct SessionKeys {}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("btc-parachain"),
    impl_name: create_runtime_str!("btc-parachain"),
    authoring_version: 1,
    spec_version: 9,
    impl_version: 1,
    transaction_version: 1,
    apis: RUNTIME_API_VERSIONS,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const EPOCH_DURATION_IN_BLOCKS: u32 = 10 * MINUTES;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

#[derive(codec::Encode, codec::Decode)]
pub enum XCMPMessage<XAccountId, XBalance> {
    /// Transfer tokens to the given account from the Parachain account.
    TransferToken(XAccountId, XBalance),
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
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
    /// Runtime version.
    type Version = Version;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    #[cfg(feature = "aura-grandpa")]
    type OnSetCode = ();
    #[cfg(feature = "cumulus-polkadot")]
    type OnSetCode = ParachainSystem;
}

#[cfg(feature = "aura-grandpa")]
impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
}

#[cfg(feature = "aura-grandpa")]
impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type KeyOwnerProofSystem = ();
    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type KeyOwnerIdentification =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;
    type HandleEquivocation = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Backing, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
    type Call = Call;
    type Event = Event;
}

impl pallet_utility::Config for Runtime {
    type Call = Call;
    type Event = Event;
    type WeightInfo = ();
}

#[cfg(feature = "cumulus-polkadot")]
impl cumulus_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type DownwardMessageHandlers = XcmHandler;
    type XcmpMessageHandlers = XcmHandler;
}

#[cfg(feature = "cumulus-polkadot")]
impl parachain_info::Config for Runtime {}

#[cfg(feature = "cumulus-polkadot")]
parameter_types! {
    pub const RococoLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
    pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: Origin = xcm_handler::Origin::Relay.into();
    pub Ancestry: MultiLocation = Junction::Parachain {
        id: ParachainInfo::parachain_id().into()
    }.into();
}

#[cfg(feature = "cumulus-polkadot")]
type LocationConverter = (
    ParentIsDefault<AccountId>,
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<RococoNetwork, AccountId>,
);

#[cfg(feature = "cumulus-polkadot")]
type LocalAssetTransactor = CurrencyAdapter<
    Backing,
    Issuing,
    // Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
    LocationConverter,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
>;

#[cfg(feature = "cumulus-polkadot")]
type LocalOriginConverter = (
    SovereignSignedViaLocation<LocationConverter, Origin>,
    RelayChainAsNative<RelayChainOrigin, Origin>,
    SiblingParachainAsNative<xcm_handler::Origin, Origin>,
    SignedAccountId32AsNative<RococoNetwork, Origin>,
);

#[cfg(feature = "cumulus-polkadot")]
pub struct XcmConfig;

#[cfg(feature = "cumulus-polkadot")]
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmHandler;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = LocalOriginConverter;
    type IsReserve = NativeAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
}

#[cfg(feature = "cumulus-polkadot")]
impl xcm_handler::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type UpwardMessageSender = ParachainSystem;
    type XcmpMessageSender = ParachainSystem;
    type SendXcmOrigin = EnsureRoot<AccountId>;
    type AccountIdConverter = LocationConverter;
}

#[cfg(feature = "cumulus-polkadot")]
pub struct AccountId32Convert;
#[cfg(feature = "cumulus-polkadot")]
impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
    fn convert(account_id: AccountId) -> [u8; 32] {
        account_id.into()
    }
}

#[cfg(feature = "cumulus-polkadot")]
impl parachain_tokens::Config for Runtime {
    type Event = Event;
    type AccountId32Convert = AccountId32Convert;
    type AccountIdConverter = LocationConverter;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ParaId = ParachainInfo;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
    pub const MaxLocks: u32 = 50;
}

/// Backing currency - e.g. DOT/KSM
impl pallet_balances::Config<pallet_balances::Instance1> for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Runtime, pallet_balances::Instance1>,
        frame_system::Provider<Runtime>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

/// Issuing currency - e.g. PolkaBTC
impl pallet_balances::Config<pallet_balances::Instance2> for Runtime {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Runtime, pallet_balances::Instance2>,
        frame_system::Provider<Runtime>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

impl btc_relay::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

parameter_types! {
    pub const BackingName: &'static [u8] = b"Polkadot";
    pub const BackingSymbol: &'static [u8] = b"DOT";
    pub const BackingDecimals: u8 = 10;
}

impl currency::Config<currency::Backing> for Runtime {
    type Event = Event;
    type Currency = Backing;
    type Name = BackingName;
    type Symbol = BackingSymbol;
    type Decimals = BackingDecimals;
}

parameter_types! {
    pub const IssuingName: &'static [u8] = b"PolkaBTC";
    pub const IssuingSymbol: &'static [u8] = b"PolkaBTC";
    pub const IssuingDecimals: u8 = 8;
}

impl currency::Config<currency::Issuing> for Runtime {
    type Event = Event;
    type Currency = Issuing;
    type Name = IssuingName;
    type Symbol = IssuingSymbol;
    type Decimals = IssuingDecimals;
}

impl reward::Config<reward::BackingVault> for Runtime {
    type Event = Event;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::IssuingVault> for Runtime {
    type Event = Event;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::BackingRelayer> for Runtime {
    type Event = Event;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::IssuingRelayer> for Runtime {
    type Event = Event;
    type SignedFixedPoint = FixedI128;
}

impl security::Config for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const MinimumDeposit: u32 = 10;
    pub const MinimumStake: u32 = 10;
    pub const VotingPeriod: BlockNumber = DAYS;
    pub const MaximumMessageSize: u32 = 256;
}

pub use staked_relayers::RawEvent as StakedRelayersEvent;

impl staked_relayers::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
    type MinimumDeposit = MinimumDeposit;
    type MinimumStake = MinimumStake;
    type VotingPeriod = VotingPeriod;
    type MaximumMessageSize = MaximumMessageSize;
}

parameter_types! {
    pub const VaultModuleId: ModuleId = ModuleId(*b"mod/vreg");
}

impl vault_registry::Config for Runtime {
    type ModuleId = VaultModuleId;
    type Event = Event;
    type RandomnessSource = RandomnessCollectiveFlip;
    type SignedFixedPoint = FixedI128;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = UncheckedExtrinsic;
}

pub use exchange_rate_oracle::RawEvent as RawExchangeRateOracleEvent;

impl exchange_rate_oracle::Config for Runtime {
    type Event = Event;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

parameter_types! {
    pub const FeeModuleId: ModuleId = ModuleId(*b"mod/fees");
}

impl fee::Config for Runtime {
    type ModuleId = FeeModuleId;
    type Event = Event;
    type WeightInfo = ();
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type UnsignedFixedPoint = FixedU128;
    type UnsignedInner = Balance;
    type BackingVaultRewards = BackingVaultRewards;
    type IssuingVaultRewards = IssuingVaultRewards;
    type BackingRelayerRewards = BackingRelayerRewards;
    type IssuingRelayerRewards = IssuingRelayerRewards;
}

impl sla::Config for Runtime {
    type Event = Event;
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type Balance = Balance;
    type BackingVaultRewards = BackingVaultRewards;
    type IssuingVaultRewards = IssuingVaultRewards;
    type BackingRelayerRewards = BackingRelayerRewards;
    type IssuingRelayerRewards = IssuingRelayerRewards;
}

pub use refund::{RawEvent as RawRefundEvent, RefundRequest};

impl refund::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use issue::{IssueRequest, RawEvent as RawIssueEvent};

impl issue::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use redeem::{RawEvent as RawRedeemEvent, RedeemRequest};

impl redeem::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use replace::{RawEvent as RawReplaceEvent, ReplaceRequest};

impl replace::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use nomination::RawEvent as RawNominationEvent;

impl nomination::Config for Runtime {
    type Event = Event;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

macro_rules! construct_polkabtc_runtime {
	($( $modules:tt )*) => {
		#[allow(clippy::large_enum_variant)]
		construct_runtime! {
			pub enum Runtime where
                Block = Block,
                NodeBlock = opaque::Block,
                UncheckedExtrinsic = UncheckedExtrinsic
            {
                System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
                Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
                Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
                Utility: pallet_utility::{Pallet, Call, Event},
                RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage},
                TransactionPayment: pallet_transaction_payment::{Pallet, Storage},

                // Tokens & Balances
                Backing: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
                Issuing: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},

                BackingCurrency: currency::<Instance1>::{Pallet, Call, Storage, Event<T>},
                IssuingCurrency: currency::<Instance2>::{Pallet, Call, Storage, Event<T>},

                BackingVaultRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
                IssuingVaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
                BackingRelayerRewards: reward::<Instance3>::{Pallet, Call, Storage, Event<T>},
                IssuingRelayerRewards: reward::<Instance4>::{Pallet, Call, Storage, Event<T>},

                // Bitcoin SPV
                BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},

                // Operational
                Security: security::{Pallet, Call, Storage, Event<T>},
                StakedRelayers: staked_relayers::{Pallet, Call, Storage, Event<T>},
                VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
                ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
                Issue: issue::{Pallet, Call, Config<T>, Storage, Event<T>},
                Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>},
                Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>},
                Fee: fee::{Pallet, Call, Config<T>, Storage, Event<T>},
                Sla: sla::{Pallet, Call, Config<T>, Storage, Event<T>},
                Refund: refund::{Pallet, Call, Config<T>, Storage, Event<T>},
                Nomination: nomination::{Pallet, Call, Config<T>, Storage, Event<T>},

				$($modules)*
			}
		}
	}
}

#[cfg(feature = "cumulus-polkadot")]
construct_polkabtc_runtime! {
    ParachainSystem: cumulus_parachain_system::{Pallet, Call, Storage, Inherent, Event},
    ParachainInfo: parachain_info::{Pallet, Storage, Config},
    ParachainTokens: parachain_tokens::{Pallet, Storage, Call, Event<T>},
    XcmHandler: xcm_handler::{Pallet, Event<T>, Origin, Call},
}

#[cfg(feature = "aura-grandpa")]
construct_polkabtc_runtime! {
    Aura: pallet_aura::{Pallet, Config<T>},
    Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
}

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
pub type Executive =
    frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPallets>;

#[cfg(not(feature = "disable-runtime-api"))]
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
            RandomnessCollectiveFlip::random_seed().0
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

    #[cfg(feature = "aura-grandpa")]
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }

        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }
    }

    #[cfg(feature = "aura-grandpa")]
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

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            impl frame_system_benchmarking::Config for Runtime {}

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
            add_benchmark!(params, batches, fee, Fee);
            add_benchmark!(params, batches, nomination, Nomination);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl module_exchange_rate_oracle_rpc_runtime_api::ExchangeRateOracleApi<
        Block,
        Balance,
        Balance,
    > for Runtime {
        fn issuing_to_backing(amount: BalanceWrapper<Balance>) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = ExchangeRateOracle::issuing_to_backing(amount.amount)?;
            Ok(BalanceWrapper{amount:result})
        }

        fn backing_to_issuing(amount: BalanceWrapper<Balance>) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = ExchangeRateOracle::backing_to_issuing(amount.amount)?;
            Ok(BalanceWrapper{amount:result})
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
        Balance,
        FixedU128
    > for Runtime {
        fn get_total_collateralization() -> Result<FixedU128, DispatchError> {
            VaultRegistry::get_total_collateralization()
        }

        fn get_first_vault_with_sufficient_collateral(amount: BalanceWrapper<Balance>) -> Result<AccountId, DispatchError> {
            VaultRegistry::get_first_vault_with_sufficient_collateral(amount.amount)
        }

        fn get_first_vault_with_sufficient_tokens(amount: BalanceWrapper<Balance>) -> Result<AccountId, DispatchError> {
            VaultRegistry::get_first_vault_with_sufficient_tokens(amount.amount)
        }

        fn get_premium_redeem_vaults() -> Result<Vec<(AccountId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_premium_redeem_vaults()?;
            Ok(result.iter().map(|v| (v.0.clone(), BalanceWrapper{amount:v.1})).collect())
        }

        fn get_vaults_with_issuable_tokens() -> Result<Vec<(AccountId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_vaults_with_issuable_tokens()?;
            Ok(result.into_iter().map(|v| (v.0, BalanceWrapper{amount:v.1})).collect())
        }

        fn get_vaults_with_redeemable_tokens() -> Result<Vec<(AccountId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_vaults_with_redeemable_tokens()?;
            Ok(result.into_iter().map(|v| (v.0, BalanceWrapper{amount:v.1})).collect())
        }

        fn get_issuable_tokens_from_vault(vault: AccountId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_issuable_tokens_from_vault(vault)?;
            Ok(BalanceWrapper{amount:result})
        }

        fn get_collateralization_from_vault(vault: AccountId, only_issued: bool) -> Result<FixedU128, DispatchError> {
            VaultRegistry::get_collateralization_from_vault(vault, only_issued)
        }

        fn get_collateralization_from_vault_and_collateral(vault: AccountId, collateral: BalanceWrapper<Balance>, only_issued: bool) -> Result<FixedU128, DispatchError> {
            VaultRegistry::get_collateralization_from_vault_and_collateral(vault, collateral.amount, only_issued)
        }

        fn get_required_collateral_for_issuing(amount_btc: BalanceWrapper<Balance>) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_required_collateral_for_issuing(amount_btc.amount)?;
            Ok(BalanceWrapper{amount:result})
        }

        fn get_required_collateral_for_vault(vault_id: AccountId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_required_collateral_for_vault(vault_id)?;
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

    impl module_refund_rpc_runtime_api::RefundApi<
        Block,
        AccountId,
        H256,
        RefundRequest<AccountId, Balance>
    > for Runtime {
        fn get_refund_requests(account_id: AccountId) -> Vec<(H256, RefundRequest<AccountId, Balance>)> {
            Refund::get_refund_requests_for_account(account_id)
        }

        fn get_refund_requests_by_issue_id(issue_id: H256) -> Option<(H256, RefundRequest<AccountId, Balance>)> {
            Refund::get_refund_requests_by_issue_id(issue_id)
        }

        fn get_vault_refund_requests(account_id: AccountId) -> Vec<(H256, RefundRequest<AccountId, Balance>)> {
            Refund::get_refund_requests_for_vault(account_id)
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

#[cfg(feature = "cumulus-polkadot")]
cumulus_parachain_system::register_validate_block!(Runtime, Executive);
