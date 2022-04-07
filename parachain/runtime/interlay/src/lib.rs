//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use bitcoin::types::H256Le;
use currency::Amount;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    traits::{
        Contains, Currency as PalletCurrency, EnsureOneOf, EnsureOrigin, EqualPrivilegeOnly, ExistenceRequirement,
        Imbalance, OnUnbalanced,
    },
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, RawOrigin,
};
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, H256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Convert, IdentityLookup, Zero},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perquintill,
};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{Everything, Get, KeyOwnerProofSystem, LockIdentifier, Nothing},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

// interBTC exports
pub use btc_relay::{bitcoin, Call as RelayCall, TARGET_SPACING};
pub use module_oracle_rpc_runtime_api::BalanceWrapper;
pub use security::StatusCode;

pub use primitives::{
    self, AccountId, Balance, BlockNumber, CurrencyId, CurrencyId::Token, CurrencyInfo, Hash, Moment, Nonce, Signature,
    SignedFixedPoint, SignedInner, UnsignedFixedPoint, UnsignedInner, DOT, IBTC, INTR,
};

// XCM imports
use cumulus_primitives_core::ParaId;
use orml_xcm_support::{DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
use pallet_xcm::{EnsureXcm, IsMajorityOfBody, XcmPassthrough};
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;

use xcm_builder::{
    AccountId32Aliases, AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds, LocationInverter,
    NativeAsset, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
    SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

type VaultId = primitives::VaultId<AccountId, CurrencyId>;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("interlay-parachain"),
    impl_name: create_runtime_str!("interlay-parachain"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    transaction_version: 2, // added preimage
    apis: RUNTIME_API_VERSIONS,
    state_version: 0,
};

// The relay chain is limited to 12s to include parachain blocks.
pub const MILLISECS_PER_BLOCK: u64 = 12000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
pub const YEARS: BlockNumber = DAYS * 365;

pub const BITCOIN_SPACING_MS: u32 = TARGET_SPACING * 1000;
pub const BITCOIN_BLOCK_SPACING: BlockNumber = BITCOIN_SPACING_MS / MILLISECS_PER_BLOCK as BlockNumber;

pub mod token_distribution {
    use super::*;

    // 1 billion INTR distributed over 4 years
    // INTR has 10 decimal places, same as DOT
    // See: https://wiki.polkadot.network/docs/learn-DOT#polkadot
    pub const INITIAL_ALLOCATION: Balance = 1_000_000_000 * UNITS;

    // multiplication is non-overflow by default
    pub const ESCROW_INFLATION_REWARDS: Permill = Permill::from_parts(67000); // 6.7%
    pub const TREASURY_INFLATION_REWARDS: Permill = Permill::from_parts(533000); // 53.3%
    pub const VAULT_INFLATION_REWARDS: Permill = Permill::from_percent(40); // 40%
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
/// We allow for 2 seconds of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = 0; // TODO: this is 0 so that we can do runtime upgrade without fees. Restore value afterwards!
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
    pub const SS58Prefix: u16 = 2032;
}

pub struct BaseCallFilter;

impl Contains<Call> for BaseCallFilter {
    fn contains(call: &Call) -> bool {
        if matches!(
            call,
            Call::System(_)
                | Call::Authorship(_)
                | Call::Session(_)
                | Call::Timestamp(_)
                | Call::ParachainSystem(_)
                | Call::Sudo(_)
                | Call::Democracy(_)
                | Call::Escrow(_)
                | Call::TechnicalCommittee(_)
        ) {
            // always allow core calls
            true
        } else if security::Pallet::<Runtime>::is_parachain_shutdown() {
            // in shutdown mode, all non-core calls are disallowed
            false
        } else if let Call::PolkadotXcm(_) = call {
            // For security reasons, disallow usage of the xcm package by users. Sudo and
            // governance are still able to call these (sudo is explicitly white-listed, while
            // governance bypasses this call filter).
            false
        } else {
            true
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
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
    pub const MaxAuthorities: u32 = 32;
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but lets be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxCandidates: u32 = 1000;
    pub const MinCandidates: u32 = 5;
    pub const SessionLength: BlockNumber = 6 * HOURS;
    pub const MaxInvulnerables: u32 = 100;
    pub const ExecutiveBody: BodyId = BodyId::Executive;
}

/// We allow root and the Relay Chain council to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin =
    EnsureOneOf<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<ParentLocation, ExecutiveBody>>>;

impl pallet_collator_selection::Config for Runtime {
    type Event = Event;
    type Currency = NativeCurrency;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = CollatorPotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
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

pub type SlowAdjustingFeeUpdate<R> =
    TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

parameter_types! {
    // TODO: this is 0 so that we can do runtime upgrade without fees. Restore value afterwards!
    pub const TransactionByteFee: Balance = 0;
    /// This value increases the priority of `Operational` transactions by adding
    /// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
    pub OperationalFeeMultiplier: u8 = 5;
    /// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
    /// than this will decrease the weight and more will increase.
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    /// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
    /// change the fees more rapidly.
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
    /// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
    /// that combined with `AdjustmentVariable`, we can recover from the minimum.
    /// See `multiplier_can_grow_from_zero`.
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1u128, 1_000_000u128);
}

type NegativeImbalance<T, GetCurrencyId> = <orml_tokens::CurrencyAdapter<T, GetCurrencyId> as PalletCurrency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub struct DealWithFees<T, GetCurrencyId>(PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> OnUnbalanced<NegativeImbalance<T, GetCurrencyId>> for DealWithFees<T, GetCurrencyId>
where
    T: pallet_authorship::Config + orml_tokens::Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<T, GetCurrencyId>>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            if let Some(author) = pallet_authorship::Pallet::<T>::author() {
                orml_tokens::CurrencyAdapter::<T, GetCurrencyId>::resolve_creating(&author, fees);
            }
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction =
        pallet_transaction_payment::CurrencyAdapter<NativeCurrency, DealWithFees<Runtime, GetNativeCurrencyId>>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
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
    type PalletsOrigin = OriginCaller;
}

parameter_types! {
    pub MinVestedTransfer: Balance = 0;
    // NOTE: per account, airdrop only needs one
    pub const MaxVestingSchedules: u32 = 1;
}

parameter_types! {
    pub KintsugiLabsAccounts: Vec<AccountId> = vec![
        hex_literal::hex!["249cf21ac84a06f2e1661e215e404530529f3932034abe9a5b8e3da5eee8b374"].into(),	// 5CtiCmFoHDSiLLoaBkqj9sGonciXgAvj2mnbZ8DZe4bpLLQ7
        hex_literal::hex!["4aa5c577b4dcfcd72e78a728ca52707eed424b7bfa6c584b3ad9caa8087bdd20"].into(),	// 5DkaeN4Rpq4cfyExfopCSEDiJAEpKR8A4szT398p5271bVaa
        hex_literal::hex!["accb25b6794d8efa88397ccc05017727f658494484525ae8a3bd4c0bc0316e16"].into(),	// 5FyGSWj5b6VcK7L9psiDE6RQBp1XC3SzbXNt5TU1Zqr56QGY
    ];
}

pub struct EnsureKintsugiLabs;
impl EnsureOrigin<Origin> for EnsureKintsugiLabs {
    type Success = AccountId;

    fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
        Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
            RawOrigin::Signed(caller) => {
                if KintsugiLabsAccounts::get().contains(&caller) {
                    Ok(caller)
                } else {
                    Err(Origin::from(Some(caller)))
                }
            }
            r => Err(Origin::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> Origin {
        Origin::from(RawOrigin::Signed(Default::default()))
    }
}

impl orml_vesting::Config for Runtime {
    type Event = Event;
    type Currency = NativeCurrency;
    type MinVestedTransfer = MinVestedTransfer;
    type VestedTransferOrigin = EnsureKintsugiLabs;
    type WeightInfo = ();
    type MaxVestingSchedules = MaxVestingSchedules;
    type BlockNumberProvider = System;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 30;
       // Retry a scheduled item every 25 blocks (5 minute) until the preimage exists.
    pub const NoPreimagePostponement: Option<u32> = Some(5 * MINUTES);
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
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type PreimageProvider = Preimage;
    type NoPreimagePostponement = NoPreimagePostponement;
}

parameter_types! {
    pub const PreimageMaxSize: u32 = 4096 * 1024;
    pub PreimageBaseDepositz: Balance = deposit(2, 64); // todo: rename
    pub PreimageByteDepositz: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = ();
    type Event = Event;
    type Currency = NativeCurrency;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type MaxSize = PreimageMaxSize;
    type BaseDeposit = PreimageBaseDepositz;
    type ByteDeposit = PreimageByteDepositz;
}

// Migration for scheduler pallet to move from a plain Call to a CallOrHash.
pub struct SchedulerMigrationV3;
impl frame_support::traits::OnRuntimeUpgrade for SchedulerMigrationV3 {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        Scheduler::migrate_v2_to_v3()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        Scheduler::pre_migrate_to_v3()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        Scheduler::post_migrate_to_v3()
    }
}

// https://github.com/paritytech/polkadot/blob/c4ee9d463adccfa3bf436433e3e26d0de5a4abbc/runtime/polkadot/src/constants.rs#L18
pub const UNITS: Balance = NATIVE_CURRENCY_ID.one();
pub const DOLLARS: Balance = UNITS; // 10_000_000_000
pub const CENTS: Balance = DOLLARS / 100; // 100_000_000
pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS
}

type EnsureRootOrAllTechnicalCommittee = EnsureOneOf<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeInstance, 1, 1>,
>;

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 7 * DAYS;
    pub const VotingPeriod: BlockNumber = 7 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    // TODO: update this once we have the crowdloan data in
    // Require 1 vINTR for now
    pub MinimumDeposit: Balance = 1 * UNITS;
    pub const EnactmentPeriod: BlockNumber = DAYS;
    pub PreimageByteDeposit: Balance = 10 * MILLICENTS;
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
    /// with a shorter voting period.
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
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const GetDepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const GetDepositFactor: Balance = deposit(0, 32);
    pub GetMaxSignatories: u16 = 100; // multisig of at most 100 accounts
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = NativeCurrency;
    type DepositBase = GetDepositBase;
    type DepositFactor = GetDepositFactor;
    type MaxSignatories = GetMaxSignatories;
    type WeightInfo = ();
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub ProposalBondMinimum: Balance = 5;
    pub ProposalBondMaximum: Option<Balance> = None;
    pub const SpendPeriod: BlockNumber = 7 * DAYS;
    pub const Burn: Permill = Permill::from_percent(0);
    pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = NativeCurrency;
    type ApproveOrigin = EnsureRoot<AccountId>;
    type RejectOrigin = EnsureRoot<AccountId>;
    type Event = Event;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type WeightInfo = ();
    type MaxApprovals = MaxApprovals;
}

parameter_types! {
    pub const TechnicalCommitteeMotionDuration: BlockNumber = 3 * DAYS;
    pub const TechnicalCommitteeMaxProposals: u32 = 100;
    pub const TechnicalCommitteeMaxMembers: u32 = 100;
}

type TechnicalCommitteeInstance = pallet_collective::Instance1;

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
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const ParentLocation: MultiLocation = MultiLocation::parent();
    pub const ParentNetwork: NetworkId = NetworkId::Kusama;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Means for transacting assets on this chain.
type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<ParentNetwork, AccountId>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, Origin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, Origin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<ParentNetwork, Origin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<Origin>,
);

parameter_types! {
    // One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer.
    pub UnitWeightCost: Weight = 200_000_000;
}

pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

parameter_types! {
    pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;

impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
    type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type Trader = UsingComponents<
        IdentityFee<Balance>,
        ParentLocation,
        AccountId,
        orml_tokens::CurrencyAdapter<Runtime, GetRelayChainCurrencyId>,
        (),
    >;
    type ResponseHandler = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, ParentNetwork>,);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Origin = Origin;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

pub type LocalAssetTransactor = MultiCurrencyAdapter<
    Tokens,
    UnknownTokens,
    IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
    AccountId,
    LocationToAccountId,
    CurrencyId,
    CurrencyIdConvert,
    DepositToAlternative<TreasuryAccount, Tokens, CurrencyId, AccountId, Balance>,
>;

pub use currency_id_convert::CurrencyIdConvert;

mod currency_id_convert {
    use super::*;
    use codec::{Decode, Encode};

    fn native_currency_location(id: CurrencyId) -> MultiLocation {
        MultiLocation::new(1, X2(Parachain(ParachainInfo::get().into()), GeneralKey(id.encode())))
    }

    pub struct CurrencyIdConvert;

    impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
        fn convert(id: CurrencyId) -> Option<MultiLocation> {
            match id {
                PARENT_CURRENCY_ID => Some(MultiLocation::parent()),
                WRAPPED_CURRENCY_ID => Some(native_currency_location(id)),
                NATIVE_CURRENCY_ID => Some(native_currency_location(id)),
                _ => None,
            }
        }
    }

    impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
        fn convert(location: MultiLocation) -> Option<CurrencyId> {
            fn decode_currency_id(key: Vec<u8>) -> Option<CurrencyId> {
                // decode the general key
                if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
                    // check `currency_id` is cross-chain asset
                    match currency_id {
                        WRAPPED_CURRENCY_ID => Some(currency_id),
                        NATIVE_CURRENCY_ID => Some(currency_id),
                        _ => None,
                    }
                } else {
                    None
                }
            }

            match location {
                x if x == MultiLocation::parent() => Some(PARENT_CURRENCY_ID),
                MultiLocation {
                    parents: 1,
                    interior: X2(Parachain(id), GeneralKey(key)),
                } if ParaId::from(id) == ParachainInfo::get() => decode_currency_id(key),
                MultiLocation {
                    // adapt for reanchor canonical location: https://github.com/paritytech/polkadot/pull/4470
                    parents: 0,
                    interior: X1(GeneralKey(key)),
                } => decode_currency_id(key),
                _ => None,
            }
        }
    }

    impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
        fn convert(asset: MultiAsset) -> Option<CurrencyId> {
            if let MultiAsset {
                id: Concrete(location), ..
            } = asset
            {
                Self::convert(location)
            } else {
                None
            }
        }
    }
}

parameter_types! {
    pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
    pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
    // Only used for transferring parachain tokens to other parachains using DOT as fee currency. Currently we do not support this, hence return MAX.
    // See: https://github.com/open-web3-stack/open-runtime-module-library/blob/cadcc9fb10b8212f92668138fc8f83dc0c53acf5/xtokens/README.md#transfer-multiple-currencies
    pub ParachainMinFee: |location: MultiLocation| -> u128 {
        #[allow(clippy::match_ref_pats)] // false positive
        match (location.parents, location.first_interior()) {
            _ => u128::MAX,
        }
    };
}

pub struct AccountIdToMultiLocation;

impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: NetworkId::Any,
            id: account.into(),
        })
        .into()
    }
}

impl orml_xtokens::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type CurrencyId = CurrencyId;
    type CurrencyIdConvert = CurrencyIdConvert;
    type AccountIdToMultiLocation = AccountIdToMultiLocation;
    type SelfLocation = SelfLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type BaseXcmWeight = UnitWeightCost;
    type LocationInverter = <XcmConfig as Config>::LocationInverter;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    type MinXcmFee = ParachainMinFee;
    type MultiLocationsFilter = Everything;
    type ReserveProvider = AbsoluteReserveProvider;
}

impl orml_unknown_tokens::Config for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = BITCOIN_BLOCK_SPACING;
}

impl btc_relay::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
}

const NATIVE_CURRENCY_ID: CurrencyId = Token(INTR);
const PARENT_CURRENCY_ID: CurrencyId = Token(DOT);
const WRAPPED_CURRENCY_ID: CurrencyId = Token(IBTC);

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
    pub const GetRelayChainCurrencyId: CurrencyId = PARENT_CURRENCY_ID;
    pub const GetWrappedCurrencyId: CurrencyId = WRAPPED_CURRENCY_ID;
}

type NativeCurrency = orml_tokens::CurrencyAdapter<Runtime, GetNativeCurrencyId>;

// Pallet accounts
parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
    pub const SupplyPalletId: PalletId = PalletId(*b"mod/supl");
    pub const EscrowAnnuityPalletId: PalletId = PalletId(*b"esc/annu");
    pub const VaultAnnuityPalletId: PalletId = PalletId(*b"vlt/annu");
    pub const TreasuryPalletId: PalletId = PalletId(*b"mod/trsy");
    pub const CollatorPotId: PalletId = PalletId(*b"col/slct");
    pub const VaultRegistryPalletId: PalletId = PalletId(*b"mod/vreg");
}

parameter_types! {
    // wd9yNSwR5jsJWJNNuqYxifekh2dwu9u15Sr1tP5kmKTJBLc4R
    pub FeeAccount: AccountId = FeePalletId::get().into_account();
    // wd9yNSwR5jsJWJmaV4ccaRqdxXFTJUS4shu7RMuSVk3c5F3f4
    pub SupplyAccount: AccountId = SupplyPalletId::get().into_account();
    // wd9yNSwR495PKYxKfdeuMcNyu6kqay7wKeWcLMvQ8muuWVPYj
    pub EscrowAnnuityAccount: AccountId = EscrowAnnuityPalletId::get().into_account();
    // wd9yNSwR7YL4Y4PEtY4pUxYR2jeVdsgwyoN8fwVc9196VMAt4
    pub VaultAnnuityAccount: AccountId = VaultAnnuityPalletId::get().into_account();
    // wd9yNSwR5jsJWJoLHrMKt4K2T7R5392YmZoRdpqijnpLGzEcT
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
    // wd9yNSwR3jeqTuo91k53PUAcfaqX5ZCK4gCPHa5G3h1y8kBEe
    pub CollatorSelectionAccount: AccountId = CollatorPotId::get().into_account();
    // wd9yNSwR5jsJWJrtHcnS8Wf6D5zF2dbQhxwRuvAzg9jefbhuM
    pub VaultRegistryAccount: AccountId = VaultRegistryPalletId::get().into_account();
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
    vec![
        FeeAccount::get(),
        SupplyAccount::get(),
        EscrowAnnuityAccount::get(),
        VaultAnnuityAccount::get(),
        TreasuryAccount::get(),
        CollatorSelectionAccount::get(),
        VaultRegistryAccount::get(),
    ]
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
    fn contains(a: &AccountId) -> bool {
        get_all_module_accounts().contains(a)
    }
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Zero::zero()
    };
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

parameter_types! {
    pub const InflationPeriod: BlockNumber = YEARS;
}

pub struct DealWithRewards;

impl supply::OnInflation<AccountId> for DealWithRewards {
    type Currency = NativeCurrency;
    // transfer will only fail if balance is too low
    // existential deposit is not required due to whitelist
    fn on_inflation(from: &AccountId, amount: Balance) {
        let vault_inflation = token_distribution::VAULT_INFLATION_REWARDS * amount;
        let escrow_inflation = token_distribution::ESCROW_INFLATION_REWARDS * amount;

        // vault block rewards
        let _ = Self::Currency::transfer(
            from,
            &VaultAnnuityAccount::get(),
            vault_inflation,
            ExistenceRequirement::KeepAlive,
        );
        VaultAnnuity::update_reward_per_block();

        // stake-to-vote block rewards
        let _ = Self::Currency::transfer(
            from,
            &EscrowAnnuityAccount::get(),
            escrow_inflation,
            ExistenceRequirement::KeepAlive,
        );
        EscrowAnnuity::update_reward_per_block();

        // remainder goes to treasury
        let _ = Self::Currency::transfer(
            from,
            &TreasuryAccount::get(),
            amount.saturating_sub(vault_inflation).saturating_sub(escrow_inflation),
            ExistenceRequirement::KeepAlive,
        );
    }
}

impl supply::Config for Runtime {
    type SupplyPalletId = SupplyPalletId;
    type Event = Event;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Currency = NativeCurrency;
    type InflationPeriod = InflationPeriod;
    type OnInflation = DealWithRewards;
}

parameter_types! {
    pub const EmissionPeriod: BlockNumber = YEARS;
}

pub struct EscrowBlockRewardProvider;

impl annuity::BlockRewardProvider<AccountId> for EscrowBlockRewardProvider {
    type Currency = NativeCurrency;
    fn distribute_block_reward(_from: &AccountId, amount: Balance) -> DispatchResult {
        <EscrowRewards as reward::Rewards<AccountId, Balance, CurrencyId>>::distribute_reward(
            amount,
            GetNativeCurrencyId::get(),
        )
    }
    fn withdraw_reward(who: &AccountId) -> Result<Balance, DispatchError> {
        <EscrowRewards as reward::Rewards<AccountId, Balance, CurrencyId>>::withdraw_reward(
            who,
            GetNativeCurrencyId::get(),
        )
    }
}

type EscrowAnnuityInstance = annuity::Instance1;

impl annuity::Config<EscrowAnnuityInstance> for Runtime {
    type AnnuityPalletId = EscrowAnnuityPalletId;
    type Event = Event;
    type Currency = NativeCurrency;
    type BlockRewardProvider = EscrowBlockRewardProvider;
    type BlockNumberToBalance = BlockNumberToBalance;
    type EmissionPeriod = EmissionPeriod;
    type WeightInfo = ();
}

pub struct VaultBlockRewardProvider;

impl annuity::BlockRewardProvider<AccountId> for VaultBlockRewardProvider {
    type Currency = NativeCurrency;
    fn distribute_block_reward(from: &AccountId, amount: Balance) -> DispatchResult {
        // TODO: remove fee pallet?
        Self::Currency::transfer(from, &FeeAccount::get(), amount, ExistenceRequirement::KeepAlive)?;
        <VaultRewards as reward::Rewards<VaultId, Balance, CurrencyId>>::distribute_reward(
            amount,
            GetNativeCurrencyId::get(),
        )
    }
    fn withdraw_reward(_: &AccountId) -> Result<Balance, DispatchError> {
        Err(sp_runtime::TokenError::Unsupported.into())
    }
}

type VaultAnnuityInstance = annuity::Instance2;

impl annuity::Config<VaultAnnuityInstance> for Runtime {
    type AnnuityPalletId = VaultAnnuityPalletId;
    type Event = Event;
    type Currency = NativeCurrency;
    type BlockRewardProvider = VaultBlockRewardProvider;
    type BlockNumberToBalance = BlockNumberToBalance;
    type EmissionPeriod = EmissionPeriod;
    type WeightInfo = ();
}

type EscrowRewardsInstance = reward::Instance1;

impl reward::Config<EscrowRewardsInstance> for Runtime {
    type Event = Event;
    type SignedFixedPoint = SignedFixedPoint;
    type RewardId = AccountId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
}

type VaultRewardsInstance = reward::Instance2;

impl reward::Config<VaultRewardsInstance> for Runtime {
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
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetRelayChainCurrencyId;
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
    pub const MaxPeriod: BlockNumber = WEEKS * 96;
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
    type Currency = NativeCurrency;
    type Span = Span;
    type MaxPeriod = MaxPeriod;
    type EscrowRewards = EscrowRewards;
    type WeightInfo = ();
}

impl vault_registry::Config for Runtime {
    type PalletId = VaultRegistryPalletId;
    type Event = Event;
    type Balance = Balance;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
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

impl fee::Config for Runtime {
    type FeePalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type UnsignedInner = UnsignedInner;
    type VaultRewards = VaultRewards;
    type VaultStaking = VaultStaking;
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
}

construct_runtime! {
    pub enum Runtime where
        Block = Block,
        NodeBlock = primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 0,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
        Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 2,
        Utility: pallet_utility::{Pallet, Call, Event} = 3,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 4,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 5,
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 6,
        MultiSig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 7,

        // Tokens & Balances
        Currency: currency::{Pallet} = 8,
        Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>} = 9,
        Escrow: escrow::{Pallet, Call, Storage, Event<T>} = 10,
        Vesting: orml_vesting::{Pallet, Storage, Call, Event<T>, Config<T>} = 11,

        EscrowAnnuity: annuity::<Instance1>::{Pallet, Call, Storage, Event<T>} = 12,
        EscrowRewards: reward::<Instance1>::{Pallet, Storage, Event<T>} = 13,

        VaultAnnuity: annuity::<Instance2>::{Pallet, Call, Storage, Event<T>} = 14,
        VaultRewards: reward::<Instance2>::{Pallet, Storage, Event<T>} = 15,
        VaultStaking: staking::{Pallet, Storage, Event<T>} = 16,

        Supply: supply::{Pallet, Storage, Call, Event<T>, Config<T>} = 17,

        // Bitcoin SPV
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>} = 18,
        Relay: relay::{Pallet, Call, Storage, Event<T>} = 19,

        // Operational
        Security: security::{Pallet, Call, Config, Storage, Event<T>} = 20,
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned} = 21,
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>} = 22,
        Issue: issue::{Pallet, Call, Config<T>, Storage, Event<T>} = 23,
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>} = 24,
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>} = 25,
        Fee: fee::{Pallet, Call, Config<T>, Storage} = 26,
        Refund: refund::{Pallet, Call, Config<T>, Storage, Event<T>} = 27,
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>} = 28,

        // Governance
        Democracy: democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 29,
        TechnicalCommittee: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 30,
        TechnicalMembership: pallet_membership::{Pallet, Call, Storage, Event<T>, Config<T>} = 31,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 32,

        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>} = 33,
        ParachainInfo: parachain_info::{Pallet, Storage, Config} = 34,

        Authorship: pallet_authorship::{Pallet, Call, Storage} = 35,
        CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 36,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 37,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 38,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 39,

        // XCM helpers.
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 40,
        PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config} = 41,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 42,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 43,

        XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 44,
        UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 45,
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
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    SchedulerMigrationV3,
>;

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

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
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
        fn wrapped_to_collateral( amount: BalanceWrapper<Balance>, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let result = Oracle::wrapped_to_collateral(amount.amount, currency_id)?;
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

        fn get_vault_redeem_requests(account_id: AccountId) -> Vec<H256> {
            Redeem::get_redeem_requests_for_vault(account_id)
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

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
    fn check_inherents(
        block: &Block,
        relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
    ) -> sp_inherents::CheckInherentsResult {
        let relay_chain_slot = relay_state_proof
            .read_slot()
            .expect("Could not read the relay chain slot from the proof");

        let inherent_data = cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
            relay_chain_slot,
            sp_std::time::Duration::from_secs(6),
        )
        .create_inherent_data()
        .expect("Could not create the timestamp inherent data");

        inherent_data.check_extrinsics(&block)
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
    CheckInherents = CheckInherents,
}
