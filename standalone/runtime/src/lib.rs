//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use bitcoin::types::H256Le;
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_system::{EnsureOneOf, EnsureRoot, EnsureSigned};
use sp_core::{u32_trait::_1, H256};

use frame_support::{traits::Contains, PalletId};
use orml_traits::parameter_type_with_key;
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Convert, IdentityLookup, Zero},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{Everything, Get, KeyOwnerProofSystem, LockIdentifier},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

// interBTC exports
pub use btc_relay::{bitcoin, Call as RelayCall, TARGET_SPACING};
pub use module_oracle_rpc_runtime_api::BalanceWrapper;
pub use security::StatusCode;

use currency::Amount;
pub use primitives::{
    self, AccountId, Balance, BlockNumber, CurrencyId, CurrencyInfo, Hash, Moment, Nonce, Signature, SignedFixedPoint,
    SignedInner, UnsignedFixedPoint, UnsignedInner, DOT, INTERBTC, INTR, KINT, KSM,
};

use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_core::crypto::KeyTypeId;
use sp_runtime::traits::NumberFor;

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

type VaultId = primitives::VaultId<AccountId, CurrencyId>;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("interbtc-standalone"),
    impl_name: create_runtime_str!("interbtc-standalone"),
    authoring_version: 1,
    spec_version: 1,
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
pub const WEEKS: BlockNumber = DAYS * 7;

pub const ROC: Balance = 1_000_000_000_000;
pub const MILLIROC: Balance = 1_000_000_000;
pub const MICROROC: Balance = 1_000_000;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

pub const BITCOIN_SPACING_MS: u32 = TARGET_SPACING * 1000;
pub const BITCOIN_BLOCK_SPACING: BlockNumber = BITCOIN_SPACING_MS / MILLISECS_PER_BLOCK as BlockNumber;

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
            weights.base_extrinsic = 0;  // TODO: this is 0 so that we can do runtime upgrade without fees. Restore value afterwards!
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

pub struct BaseCallFilter;

impl Contains<Call> for BaseCallFilter {
    fn contains(call: &Call) -> bool {
        if matches!(
            call,
            Call::System(_) | Call::Timestamp(_) | Call::Sudo(_) | Call::Security(_) // to unset shutdown
        ) {
            // always allow core calls
            true
        } else {
            // disallow everything if shutdown
            !security::Pallet::<Runtime>::is_parachain_shutdown()
        }
    }
}

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Nonce;
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
    type BaseCallFilter = BaseCallFilter;
    type SystemWeightInfo = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const MaxAuthorities: u32 = 32;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type KeyOwnerProofSystem = ();
    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type KeyOwnerIdentification =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;
    type HandleEquivocation = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 0;  // TODO: this is 0 so that we can do runtime upgrade without fees. Restore value afterwards!
    pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    // throwaway fees, not necessary for testing
    type OnChargeTransaction =
        pallet_transaction_payment::CurrencyAdapter<orml_tokens::CurrencyAdapter<Runtime, GetCollateralCurrencyId>, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
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

parameter_types! {
    pub MinVestedTransfer: Balance = 0;
    pub const MaxVestingSchedules: u32 = 10;
}

impl orml_vesting::Config for Runtime {
    type Event = Event;
    type Currency = orml_tokens::CurrencyAdapter<Runtime, GetNativeCurrencyId>;
    type MinVestedTransfer = MinVestedTransfer;
    // anyone can transfer vested tokens
    type VestedTransferOrigin = EnsureSigned<AccountId>;
    type WeightInfo = ();
    type MaxVestingSchedules = MaxVestingSchedules;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 30;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const GetDepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const GetDepositFactor: Balance = deposit(0, 32);
    pub GetMaxSignatories: u16 = 100; // multisig of at most 100 accounts
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = orml_tokens::CurrencyAdapter<Runtime, GetCollateralCurrencyId>; // pay for execution in DOT/KSM
    type DepositBase = GetDepositBase;
    type DepositFactor = GetDepositFactor;
    type MaxSignatories = GetMaxSignatories;
    type WeightInfo = ();
}

// https://github.com/paritytech/polkadot/blob/ece7544b40d8b29897f5aa799f27840dcc32f24d/runtime/polkadot/src/constants.rs#L18
pub const UNITS: Balance = 10_000_000_000;
pub const DOLLARS: Balance = UNITS; // 10_000_000_000
pub const CENTS: Balance = UNITS / 100; // 100_000_000
pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS
}

type EnsureRootOrAllTechnicalCommittee = EnsureOneOf<
    AccountId,
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCommitteeInstance>,
>;

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 2 * MINUTES;
    pub const VotingPeriod: BlockNumber = 5 * MINUTES;
    pub const FastTrackVotingPeriod: BlockNumber = 1 * MINUTES;
    pub MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 3 * MINUTES;
    pub const CooloffPeriod: BlockNumber = 1 * DAYS;
    pub PreimageByteDeposit: Balance = 1 * CENTS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
}

impl democracy::Config for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Escrow;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// The technical committee can have any proposal be tabled immediately
    /// with a shorter voting/enactment period.
    type FastTrackOrigin = EnsureRootOrAllTechnicalCommittee;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = ();
    type MaxProposals = MaxProposals;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"mod/trsy");
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub ProposalBondMinimum: Balance = 5;
    pub const SpendPeriod: BlockNumber = 7 * DAYS;
    pub const Burn: Permill = Permill::from_percent(0);
    pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = orml_tokens::CurrencyAdapter<Runtime, GetNativeCurrencyId>;
    type ApproveOrigin = EnsureRoot<AccountId>;
    type RejectOrigin = EnsureRoot<AccountId>;
    type Event = Event;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type WeightInfo = ();
    type MaxApprovals = MaxApprovals;
}

parameter_types! {
    pub const TechnicalCommitteeMotionDuration: BlockNumber = 10 * MINUTES;
    pub const TechnicalCommitteeMaxProposals: u32 = 100;
    pub const TechnicalCommitteeMaxMembers: u32 = 100;
}

pub type TechnicalCommitteeInstance = pallet_collective::Instance1;

impl pallet_collective::Config<TechnicalCommitteeInstance> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalCommitteeMotionDuration;
    type MaxProposals = TechnicalCommitteeMaxProposals;
    type MaxMembers = TechnicalCommitteeMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
}

impl pallet_membership::Config for Runtime {
    type Event = Event;
    type AddOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type SwapOrigin = EnsureRoot<AccountId>;
    type ResetOrigin = EnsureRoot<AccountId>;
    type PrimeOrigin = EnsureRoot<AccountId>;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalCommitteeMaxMembers;
    type WeightInfo = ();
}

parameter_types! {
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = BITCOIN_BLOCK_SPACING;
}

impl btc_relay::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
}

parameter_types! {
    pub const GetCollateralCurrencyId: CurrencyId = DOT;
    pub const GetWrappedCurrencyId: CurrencyId = INTERBTC;
    pub const GetNativeCurrencyId: CurrencyId = INTR;
    pub const MaxLocks: u32 = 50;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Zero::zero()
    };
}

parameter_types! {
    pub FeeAccount: AccountId = FeePalletId::get().into_account();
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
    vec![FeePalletId::get().into_account(), VaultPalletId::get().into_account()]
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
    fn contains(a: &AccountId) -> bool {
        get_all_module_accounts().contains(a)
    }
}

impl orml_tokens::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type Amount = primitives::Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = orml_tokens::TransferDust<Runtime, FeeAccount>;
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = DustRemovalWhitelist;
}

impl reward::Config for Runtime {
    type Event = Event;
    type SignedFixedPoint = SignedFixedPoint;
    type RewardId = VaultId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
}

impl security::Config for Runtime {
    type Event = Event;
}

pub use relay::Event as RelayEvent;

pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Runtime>, CurrencyId> for CurrencyConvert {
    fn convert(amount: &currency::Amount<Runtime>, to: CurrencyId) -> Result<currency::Amount<Runtime>, DispatchError> {
        Oracle::convert(amount, to)
    }
}

impl currency::Config for Runtime {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = CurrencyConvert;
}

impl relay::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

impl staking::Config for Runtime {
    type Event = Event;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
}

parameter_types! {
    pub const Span: BlockNumber = WEEKS;
    pub const MaxPeriod: BlockNumber = DAYS * 365 * 4;
}

pub struct BlockNumberToBalance;

impl Convert<BlockNumber, Balance> for BlockNumberToBalance {
    fn convert(a: BlockNumber) -> Balance {
        a.into()
    }
}

impl escrow::Config for Runtime {
    type Event = Event;
    type BlockNumberToBalance = BlockNumberToBalance;
    type Currency = orml_tokens::CurrencyAdapter<Runtime, GetNativeCurrencyId>;
    type Span = Span;
    type MaxPeriod = MaxPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

impl vault_registry::Config for Runtime {
    type PalletId = VaultPalletId;
    type Event = Event;
    type Balance = Balance;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetCollateralCurrencyId;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = UncheckedExtrinsic;
}

impl oracle::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
}

impl fee::Config for Runtime {
    type FeePalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type UnsignedInner = UnsignedInner;
    type VaultRewards = reward::RewardsCurrencyAdapter<Runtime>;
    type VaultStaking = staking::StakingCurrencyAdapter<Runtime>;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type OnSweep = currency::SweepFunds<Runtime, FeeAccount>;
}

pub use refund::{Event as RefundEvent, RefundRequest};

impl refund::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use issue::{Event as IssueEvent, IssueRequest};

impl issue::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use redeem::{Event as RedeemEvent, RedeemRequest};

impl redeem::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use replace::{Event as ReplaceEvent, ReplaceRequest};

impl replace::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}

pub use nomination::Event as NominationEvent;

impl nomination::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
    type VaultRewards = reward::RewardsCurrencyAdapter<Runtime>;
}

construct_runtime! {
    pub enum Runtime where
        Block = Block,
        NodeBlock = primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
        Utility: pallet_utility::{Pallet, Call, Event},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
        MultiSig: pallet_multisig::{Pallet, Call, Storage, Event<T>},

        // Tokens & Balances
        Currency: currency::{Pallet},
        Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>},
        Rewards: reward::{Pallet, Storage, Event<T>},
        Staking: staking::{Pallet, Storage, Event<T>},
        Escrow: escrow::{Pallet, Call, Storage, Event<T>},
        Vesting: orml_vesting::{Pallet, Storage, Call, Event<T>, Config<T>},

        // Bitcoin SPV
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Config, Storage, Event<T>},
        Relay: relay::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Issue: issue::{Pallet, Call, Config<T>, Storage, Event<T>},
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>},
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
        Refund: refund::{Pallet, Call, Config<T>, Storage, Event<T>},
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>},

        // Governance
        Democracy: democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
        TechnicalCommittee: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
        TechnicalMembership: pallet_membership::{Pallet, Call, Storage, Event<T>, Config<T>},
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},

        Aura: pallet_aura::{Pallet, Config<T>},
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
    }
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
            OpaqueMetadata::new(Runtime::metadata().into())
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
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
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

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
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

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
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
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, btc_relay, BTCRelay);
            list_benchmark!(list, extra, escrow, Escrow);
            list_benchmark!(list, extra, fee, Fee);
            list_benchmark!(list, extra, issue, Issue);
            list_benchmark!(list, extra, nomination, Nomination);
            list_benchmark!(list, extra, oracle, Oracle);
            list_benchmark!(list, extra, redeem, Redeem);
            list_benchmark!(list, extra, refund, Refund);
            list_benchmark!(list, extra, relay, Relay);
            list_benchmark!(list, extra, replace, Replace);
            list_benchmark!(list, extra, vault_registry, VaultRegistry);

            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

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
            add_benchmark!(params, batches, escrow, Escrow);
            add_benchmark!(params, batches, fee, Fee);
            add_benchmark!(params, batches, issue, Issue);
            add_benchmark!(params, batches, nomination, Nomination);
            add_benchmark!(params, batches, oracle, Oracle);
            add_benchmark!(params, batches, redeem, Redeem);
            add_benchmark!(params, batches, refund, Refund);
            add_benchmark!(params, batches, relay, Relay);
            add_benchmark!(params, batches, replace, Replace);
            add_benchmark!(params, batches, vault_registry, VaultRegistry);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl module_btc_relay_rpc_runtime_api::BtcRelayApi<
        Block,
        H256Le,
    > for Runtime {
        fn verify_block_header_inclusion(block_hash: H256Le) -> Result<(), DispatchError> {
            BTCRelay::verify_block_header_inclusion(block_hash, None).map(|_| ())
        }
    }

    impl module_oracle_rpc_runtime_api::OracleApi<
        Block,
        Balance,
        CurrencyId
    > for Runtime {
        fn wrapped_to_collateral(amount: BalanceWrapper<Balance>, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = Oracle::wrapped_to_collateral(amount.amount,currency_id)?;
            Ok(BalanceWrapper{amount:result})
        }

        fn collateral_to_wrapped(amount: BalanceWrapper<Balance>, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = Oracle::collateral_to_wrapped(amount.amount, currency_id)?;
            Ok(BalanceWrapper{amount:result})
        }
    }

    impl module_relay_rpc_runtime_api::RelayApi<
        Block,
        VaultId,
    > for Runtime {
        fn is_transaction_invalid(vault_id: VaultId, raw_tx: Vec<u8>) -> DispatchResult {
            Relay::is_transaction_invalid(&vault_id, raw_tx)
        }
    }

    impl module_vault_registry_rpc_runtime_api::VaultRegistryApi<
        Block,
        VaultId,
        Balance,
        UnsignedFixedPoint,
        CurrencyId,
        AccountId,
    > for Runtime {
        fn get_vault_collateral(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::compute_collateral(&vault_id)?;
            Ok(BalanceWrapper{amount:result.amount()})
        }

        fn get_vaults_by_account_id(account_id: AccountId) -> Result<Vec<VaultId>, DispatchError> {
            VaultRegistry::get_vaults_by_account_id(account_id)
        }

        fn get_vault_total_collateral(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_backing_collateral(&vault_id)?;
            Ok(BalanceWrapper{amount:result.amount()})
        }

        fn get_premium_redeem_vaults() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_premium_redeem_vaults()?;
            Ok(result.iter().map(|v| (v.0.clone(), BalanceWrapper{amount:v.1.amount()})).collect())
        }

        fn get_vaults_with_issuable_tokens() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_vaults_with_issuable_tokens()?;
            Ok(result.into_iter().map(|v| (v.0, BalanceWrapper{amount:v.1.amount()})).collect())
        }

        fn get_vaults_with_redeemable_tokens() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError> {
            let result = VaultRegistry::get_vaults_with_redeemable_tokens()?;
            Ok(result.into_iter().map(|v| (v.0, BalanceWrapper{amount:v.1.amount()})).collect())
        }

        fn get_issuable_tokens_from_vault(vault: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_issuable_tokens_from_vault(&vault)?;
            Ok(BalanceWrapper{amount:result.amount()})
        }

        fn get_collateralization_from_vault(vault: VaultId, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError> {
            VaultRegistry::get_collateralization_from_vault(vault, only_issued)
        }

        fn get_collateralization_from_vault_and_collateral(vault: VaultId, collateral: BalanceWrapper<Balance>, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError> {
            let amount = Amount::new(collateral.amount, vault.collateral_currency());
            VaultRegistry::get_collateralization_from_vault_and_collateral(vault, &amount, only_issued)
        }

        fn get_required_collateral_for_wrapped(amount_btc: BalanceWrapper<Balance>, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount_btc = Amount::new(amount_btc.amount, GetWrappedCurrencyId::get());
            let result = VaultRegistry::get_required_collateral_for_wrapped(&amount_btc, currency_id)?;
            Ok(BalanceWrapper{amount:result.amount()})
        }

        fn get_required_collateral_for_vault(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = VaultRegistry::get_required_collateral_for_vault(vault_id)?;
            Ok(BalanceWrapper{amount:result.amount()})
        }
    }

    impl module_issue_rpc_runtime_api::IssueApi<
        Block,
        AccountId,
        H256,
        IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>
    > for Runtime {
        fn get_issue_requests(account_id: AccountId) -> Vec<H256> {
            Issue::get_issue_requests_for_account(account_id)
        }

        fn get_vault_issue_requests(vault_id: AccountId) -> Vec<H256> {
            Issue::get_issue_requests_for_vault(vault_id)
        }
    }

    impl module_redeem_rpc_runtime_api::RedeemApi<
        Block,
        AccountId,
        H256,
        RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>
    > for Runtime {
        fn get_redeem_requests(account_id: AccountId) -> Vec<H256> {
            Redeem::get_redeem_requests_for_account(account_id)
        }

        fn get_vault_redeem_requests(vault_account_id: AccountId) -> Vec<H256> {
            Redeem::get_redeem_requests_for_vault(vault_account_id)
        }
    }

    impl module_refund_rpc_runtime_api::RefundApi<
        Block,
        AccountId,
        H256,
        RefundRequest<AccountId, Balance, CurrencyId>
    > for Runtime {
        fn get_refund_requests(account_id: AccountId) -> Vec<H256> {
            Refund::get_refund_requests_for_account(account_id)
        }

        fn get_refund_requests_by_issue_id(issue_id: H256) -> Option<H256> {
            Refund::get_refund_requests_by_issue_id(issue_id)
        }

        fn get_vault_refund_requests(vault_id: AccountId) -> Vec<H256> {
            Refund::get_refund_requests_for_vault(vault_id)
        }
    }

    impl module_replace_rpc_runtime_api::ReplaceApi<
        Block,
        AccountId,
        H256,
        ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>
    > for Runtime {
        fn get_old_vault_replace_requests(vault_id: AccountId) -> Vec<H256> {
            Replace::get_replace_requests_for_old_vault(vault_id)
        }

        fn get_new_vault_replace_requests(vault_id: AccountId) -> Vec<H256> {
            Replace::get_replace_requests_for_new_vault(vault_id)
        }
    }
}
