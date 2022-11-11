//! The interlay runtime. This can be compiled with `#[no_std]`, ready for Wasm.

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
        ConstU32, Contains, Currency as PalletCurrency, EitherOfDiverse, EnsureOrigin, EnsureOriginWithArg,
        EqualPrivilegeOnly, ExistenceRequirement, Imbalance, OnUnbalanced,
    },
    weights::ConstantMultiplier,
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureRootWithSuccess, RawOrigin,
};
pub use orml_asset_registry::AssetMetadata;
use orml_asset_registry::SequentialId;
use orml_traits::parameter_type_with_key;
use pallet_traits::OracleApi;
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
    construct_runtime,
    dispatch::DispatchClass,
    parameter_types,
    traits::{Everything, Get, KeyOwnerProofSystem, LockIdentifier, Nothing},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{FixedU128, Perbill, Permill};

// interBTC exports
pub use btc_relay::{bitcoin, Call as BtcRelayCall, TARGET_SPACING};
pub use constants::{currency::*, time::*};
pub use module_oracle_rpc_runtime_api::BalanceWrapper;
pub use security::StatusCode;

pub use primitives::{
    self, AccountId, Balance, BlockNumber, CurrencyInfo, Hash, Liquidity, Moment, Nonce, Rate, Ratio, Shortfall,
    Signature, SignedFixedPoint, SignedInner, UnsignedFixedPoint, UnsignedInner,
};

// XCM imports
use pallet_xcm::{EnsureXcm, IsMajorityOfBody};
use xcm::opaque::latest::BodyId;
use xcm_config::ParentLocation;

pub mod constants;
pub mod xcm_config;

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
    spec_version: 1019000,
    impl_version: 1,
    transaction_version: 2, // added preimage
    apis: RUNTIME_API_VERSIONS,
    state_version: 0,
};

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
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND.saturating_mul(2 as u64);

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
    pub const SS58Prefix: u16 = 2032;
}

pub struct BaseCallFilter;

impl Contains<RuntimeCall> for BaseCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        if matches!(
            call,
            RuntimeCall::System(_)
                | RuntimeCall::Authorship(_)
                | RuntimeCall::Session(_)
                | RuntimeCall::Timestamp(_)
                | RuntimeCall::ParachainSystem(_)
                | RuntimeCall::Democracy(_)
                | RuntimeCall::Escrow(_)
                | RuntimeCall::TechnicalCommittee(_)
        ) {
            // always allow core calls
            true
        } else if security::Pallet::<Runtime>::is_parachain_shutdown() {
            // in shutdown mode, all non-core calls are disallowed
            false
        } else if let RuntimeCall::PolkadotXcm(_) = call {
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
    type RuntimeCall = RuntimeCall;
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
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
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
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = collator_selection::IdentityCollator;
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
    EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<ParentLocation, ExecutiveBody>>>;

impl collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StakingCurrency = Escrow;
    type RewardsCurrency = NativeCurrency;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = CollatorPotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = collator_selection::IdentityCollator;
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
    pub const TransactionByteFee: Balance = MILLICENTS;
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
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction =
        pallet_transaction_payment::CurrencyAdapter<NativeCurrency, DealWithFees<Runtime, GetNativeCurrencyId>>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl pallet_utility::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
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
        // 5Fhn5mX4JGeDxikaxkJZYRYjxxbZ7DjxS5f9hsAVAzGXUNyG
        hex_literal::hex!["a0fb017d4b777bc2be8ad9e9dfe7bdf0a3db060644de499685adacd19f84df71"].into(),
        // 5GgS9vsF77Y7p2wZLEW1CW7vZpq8DSoXCf2sTdBoB51jpuan
        hex_literal::hex!["cc30e8cd03a20ce00f7dab8451a1d43047a43f50cdd0bc9b14dbaa78ed66bd1e"].into(),
        // 5GDzXqLxGiJV6A7mDp1SGRV6DB8xnnwauMEwR7PL4PW122FM
        hex_literal::hex!["b80646c2c305d0e8f1e3df9cf515a3cf1f5fc7e24a8205202fce65dfb8198345"].into(),
        // 5FgimgwW2s4V14NniQ6Nt145Sksb83xohW5LkMXYnMw3Racp
        hex_literal::hex!["a02c9cba51b7ec7c1717cdf0fd9044fa5228d9e8217a5a904871ce47627d8743"].into(),
    ];
}

pub struct EnsureKintsugiLabs;
impl EnsureOrigin<RuntimeOrigin> for EnsureKintsugiLabs {
    type Success = AccountId;

    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        Into::<Result<RawOrigin<AccountId>, RuntimeOrigin>>::into(o).and_then(|o| match o {
            RawOrigin::Signed(caller) => {
                if KintsugiLabsAccounts::get().contains(&caller) {
                    Ok(caller)
                } else {
                    Err(RuntimeOrigin::from(Some(caller)))
                }
            }
            r => Err(RuntimeOrigin::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> RuntimeOrigin {
        RuntimeOrigin::from(RawOrigin::None)
    }
}

impl orml_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
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
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
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
    type RuntimeEvent = RuntimeEvent;
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

type EnsureRootOrAllTechnicalCommittee = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeInstance, 1, 1>,
>;

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 7 * DAYS;
    pub const VotingPeriod: BlockNumber = 7 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    // Require 250 vINTR to make a proposal. Given the crowdloan airdrop, this qualifies about 7500
    // accounts to make a governance proposal if they lock their tokens for 2 years.
    pub MinimumDeposit: Balance = 250 * UNITS;
    pub const EnactmentPeriod: BlockNumber = DAYS;
    pub PreimageByteDeposit: Balance = 10 * MILLICENTS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
}

impl democracy::Config for Runtime {
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
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
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
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
    pub const SpendPeriod: BlockNumber = 1 * HOURS;
    pub const Burn: Permill = Permill::from_percent(0);
    pub const MaxApprovals: u32 = 100;
    pub const MaxSpend: Balance = Balance::MAX;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = NativeCurrency;
    type ApproveOrigin = EnsureRoot<AccountId>;
    type RejectOrigin = EnsureRoot<AccountId>;
    type SpendOrigin = EnsureRootWithSuccess<AccountId, MaxSpend>;
    type RuntimeEvent = RuntimeEvent;
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
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = TechnicalCommitteeMotionDuration;
    type MaxProposals = TechnicalCommitteeMaxProposals;
    type MaxMembers = TechnicalCommitteeMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
}

impl pallet_membership::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
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
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4 as u64);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4 as u64);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl orml_unknown_tokens::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = BITCOIN_BLOCK_SPACING;
}

impl btc_relay::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
}

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
    pub FeeAccount: AccountId = FeePalletId::get().into_account_truncating();
    // wd9yNSwR5jsJWJmaV4ccaRqdxXFTJUS4shu7RMuSVk3c5F3f4
    pub SupplyAccount: AccountId = SupplyPalletId::get().into_account_truncating();
    // wd9yNSwR495PKYxKfdeuMcNyu6kqay7wKeWcLMvQ8muuWVPYj
    pub EscrowAnnuityAccount: AccountId = EscrowAnnuityPalletId::get().into_account_truncating();
    // wd9yNSwR7YL4Y4PEtY4pUxYR2jeVdsgwyoN8fwVc9196VMAt4
    pub VaultAnnuityAccount: AccountId = VaultAnnuityPalletId::get().into_account_truncating();
    // wd9yNSwR5jsJWJoLHrMKt4K2T7R5392YmZoRdpqijnpLGzEcT
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
    // wd9yNSwR3jeqTuo91k53PUAcfaqX5ZCK4gCPHa5G3h1y8kBEe
    pub CollatorSelectionAccount: AccountId = CollatorPotId::get().into_account_truncating();
    // wd9yNSwR5jsJWJrtHcnS8Wf6D5zF2dbQhxwRuvAzg9jefbhuM
    pub VaultRegistryAccount: AccountId = VaultRegistryPalletId::get().into_account_truncating();
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
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type Amount = primitives::Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = orml_tokens::TransferDust<Runtime, FeeAccount>;
    type OnSlash = ();
    type OnDeposit = ();
    type OnTransfer = ();
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = DustRemovalWhitelist;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

pub struct AssetAuthority;
impl EnsureOriginWithArg<RuntimeOrigin, Option<u32>> for AssetAuthority {
    type Success = ();

    fn try_origin(origin: RuntimeOrigin, _asset_id: &Option<u32>) -> Result<Self::Success, RuntimeOrigin> {
        EnsureRoot::try_origin(origin)
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin(_asset_id: &Option<u32>) -> RuntimeOrigin {
        EnsureRoot::successful_origin()
    }
}

impl orml_asset_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type CustomMetadata = primitives::CustomMetadata;
    type AssetProcessor = SequentialId<Runtime>;
    type AssetId = primitives::ForeignAssetId;
    type AuthorityOrigin = AssetAuthority;
    type WeightInfo = ();
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
    type RuntimeEvent = RuntimeEvent;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Currency = NativeCurrency;
    type InflationPeriod = InflationPeriod;
    type OnInflation = DealWithRewards;
}

pub struct TotalWrapped;
impl Get<Balance> for TotalWrapped {
    fn get() -> Balance {
        orml_tokens::CurrencyAdapter::<Runtime, GetWrappedCurrencyId>::total_issuance()
    }
}

parameter_types! {
    pub const EmissionPeriod: BlockNumber = YEARS;
}

pub struct EscrowBlockRewardProvider;

impl annuity::BlockRewardProvider<AccountId> for EscrowBlockRewardProvider {
    type Currency = NativeCurrency;

    #[cfg(feature = "runtime-benchmarks")]
    fn deposit_stake(from: &AccountId, amount: Balance) -> DispatchResult {
        let current_stake = <EscrowRewards as reward::Rewards<AccountId, Balance, CurrencyId>>::get_stake(from)?;
        let new_stake = current_stake.saturating_add(amount);
        <EscrowRewards as reward::Rewards<AccountId, Balance, CurrencyId>>::set_stake(from, new_stake)
    }

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
    type RuntimeEvent = RuntimeEvent;
    type Currency = NativeCurrency;
    type BlockRewardProvider = EscrowBlockRewardProvider;
    type BlockNumberToBalance = BlockNumberToBalance;
    type EmissionPeriod = EmissionPeriod;
    type TotalWrapped = TotalWrapped;
    type WeightInfo = ();
}

pub struct VaultBlockRewardProvider;

impl annuity::BlockRewardProvider<AccountId> for VaultBlockRewardProvider {
    type Currency = NativeCurrency;

    #[cfg(feature = "runtime-benchmarks")]
    fn deposit_stake(_from: &AccountId, _amount: Balance) -> DispatchResult {
        // TODO: fix for vault id
        Ok(())
    }

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
    type RuntimeEvent = RuntimeEvent;
    type Currency = NativeCurrency;
    type BlockRewardProvider = VaultBlockRewardProvider;
    type BlockNumberToBalance = BlockNumberToBalance;
    type EmissionPeriod = EmissionPeriod;
    type TotalWrapped = TotalWrapped;
    type WeightInfo = ();
}

type EscrowRewardsInstance = reward::Instance1;

impl reward::Config<EscrowRewardsInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type RewardId = AccountId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
}

type VaultRewardsInstance = reward::Instance2;

impl reward::Config<VaultRewardsInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type RewardId = VaultId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
}

impl security::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

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

impl staking::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
}

parameter_types! {
    pub const Span: BlockNumber = WEEKS;
    pub const MaxPeriod: BlockNumber = WEEKS * 192;
}

pub struct BlockNumberToBalance;

impl Convert<BlockNumber, Balance> for BlockNumberToBalance {
    fn convert(a: BlockNumber) -> Balance {
        a.into()
    }
}

impl escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BlockNumberToBalance = BlockNumberToBalance;
    type Currency = NativeCurrency;
    type Span = Span;
    type MaxPeriod = MaxPeriod;
    type EscrowRewards = EscrowRewards;
    type WeightInfo = ();
}

// https://github.com/paritytech/polkadot/blob/be005938a64b9170a5d55887ce42004e1b086b7b/runtime/polkadot/src/lib.rs#L584-L592
parameter_types! {
    // Minimum 4 CENTS/byte
    pub const BasicDeposit: Balance = deposit(1, 258);
    pub const FieldDeposit: Balance = deposit(0, 66);
    pub const SubAccountDeposit: Balance = deposit(1, 53);
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = NativeCurrency;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRoot<AccountId>;
    type RegistrarOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

impl vault_registry::Config for Runtime {
    type PalletId = VaultRegistryPalletId;
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = UncheckedExtrinsic;
}

impl oracle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxExpectedValue: UnsignedFixedPoint = UnsignedFixedPoint::from_inner(<UnsignedFixedPoint as FixedPointNumber>::DIV);
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
    type MaxExpectedValue = MaxExpectedValue;
}

pub use issue::{Event as IssueEvent, IssueRequest};

impl issue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BlockNumberToBalance = BlockNumberToBalance;
    type WeightInfo = ();
}

pub use redeem::{Event as RedeemEvent, RedeemRequest};

impl redeem::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub use replace::{Event as ReplaceEvent, ReplaceRequest};

impl replace::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub use nomination::Event as NominationEvent;

impl nomination::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

impl clients_info::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
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
        Utility: pallet_utility::{Pallet, Call, Event} = 3,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 4,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 5,
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 6,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 7,
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 8,

        // # Tokens & Balances
        Currency: currency::{Pallet} = 20,
        Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>} = 21,
        Supply: supply::{Pallet, Storage, Call, Event<T>, Config<T>} = 22,
        Vesting: orml_vesting::{Pallet, Storage, Call, Event<T>, Config<T>} = 23,
        AssetRegistry: orml_asset_registry::{Pallet, Storage, Call, Event<T>, Config<T>} = 24,

        Escrow: escrow::{Pallet, Call, Storage, Event<T>} = 30,
        EscrowAnnuity: annuity::<Instance1>::{Pallet, Call, Storage, Event<T>} = 31,
        EscrowRewards: reward::<Instance1>::{Pallet, Storage, Event<T>} = 32,

        VaultAnnuity: annuity::<Instance2>::{Pallet, Call, Storage, Event<T>} = 40,
        VaultRewards: reward::<Instance2>::{Pallet, Storage, Event<T>} = 41,
        VaultStaking: staking::{Pallet, Storage, Event<T>} = 42,


        // # Bitcoin SPV
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>} = 50,
        // Relay: 51

        // # Operational
        Security: security::{Pallet, Call, Config, Storage, Event<T>} = 60,
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned} = 61,
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>} = 62,
        Issue: issue::{Pallet, Call, Config<T>, Storage, Event<T>} = 63,
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>} = 64,
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>} = 65,
        Fee: fee::{Pallet, Call, Config<T>, Storage} = 66,
        // Refund: 67
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>} = 68,
        ClientsInfo: clients_info::{Pallet, Call, Storage, Event<T>} = 69,

        // # Governance
        Democracy: democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 70,
        TechnicalCommittee: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 71,
        TechnicalMembership: pallet_membership::{Pallet, Call, Storage, Event<T>, Config<T>} = 72,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 73,

        Authorship: pallet_authorship::{Pallet, Call, Storage} = 80,
        CollatorSelection: collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 81,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 82,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 83,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 84,
        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>} = 85,
        ParachainInfo: parachain_info::{Pallet, Storage, Config} = 86,

        // # XCM Helpers
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 90,
        PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config} = 91,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 92,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 93,

        XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 94,
        UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 95,
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
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
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

    impl pallet_loans_rpc_runtime_api::LoansApi<
        Block,
        AccountId,
        Balance,
    > for Runtime {
        fn get_account_liquidity(_account: AccountId) -> Result<(Liquidity, Shortfall), DispatchError> {
           Err(DispatchError::Other("RPC Endpoint Not Implemented"))
        }

        fn get_market_status(_asset_id: CurrencyId) -> Result<(Rate, Rate, Rate, Ratio, Balance, Balance, FixedU128), DispatchError> {
           Err(DispatchError::Other("RPC Endpoint Not Implemented"))
        }

        fn get_liquidation_threshold_liquidity(_account: AccountId) -> Result<(Liquidity, Shortfall), DispatchError> {
           Err(DispatchError::Other("RPC Endpoint Not Implemented"))
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

    impl module_escrow_rpc_runtime_api::EscrowApi<
        Block,
        AccountId,
        BlockNumber,
        Balance
    > for Runtime {
        fn balance_at(account_id: AccountId, height: Option<BlockNumber>) -> BalanceWrapper<Balance> {
            BalanceWrapper{amount: Escrow::balance_at(&account_id, height)}
        }

        fn total_supply(height: Option<BlockNumber>) -> BalanceWrapper<Balance> {
            BalanceWrapper{amount: Escrow::total_supply(height)}
        }
    }

    impl module_reward_rpc_runtime_api::RewardApi<
        Block,
        AccountId,
        VaultId,
        CurrencyId,
        Balance
    > for Runtime {
        fn compute_escrow_reward(account_id: AccountId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount = <EscrowRewards as reward::Rewards<AccountId, Balance, CurrencyId>>::compute_reward(&account_id, currency_id)?;
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
        }

        fn compute_vault_reward(vault_id: VaultId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount = <VaultRewards as reward::Rewards<VaultId, Balance, CurrencyId>>::compute_reward(&vault_id, currency_id)?;
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
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

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade() -> (Weight, Weight) {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here. If any of the pre/post migration checks fail, we shall stop
            // right here and right now.
            let weight = Executive::try_runtime_upgrade().unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block_no_check(block: Block) -> Weight {
            Executive::execute_block_no_check(block)
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
