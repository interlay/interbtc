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
        ConstU32, Contains, Currency as PalletCurrency, EitherOfDiverse, EnsureOrigin, EnsureOriginWithArg,
        ExistenceRequirement, Imbalance, InstanceFilter, OnUnbalanced,
    },
    weights::ConstantMultiplier,
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned,
};
use loans::{OnSlashHook, PostDeposit, PostTransfer, PreDeposit, PreTransfer};
use orml_asset_registry::SequentialId;
use orml_traits::{currency::MutationHooks, parameter_type_with_key};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Bounded, Convert, IdentityLookup, NumberFor, Zero},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perquintill,
};
use sp_std::{marker::PhantomData, prelude::*};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
use frame_support::traits::Nothing;
pub use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    parameter_types,
    traits::{EqualPrivilegeOnly, Everything, FindAuthor, Get, KeyOwnerProofSystem, LockIdentifier},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{FixedU128, Perbill, Permill};

pub use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

// interBTC exports
pub use btc_relay::{bitcoin, Call as BtcRelayCall, TARGET_SPACING};
pub use oracle_rpc_runtime_api::BalanceWrapper;
pub use security::StatusCode;

pub use primitives::{
    self, AccountId, Balance, BlockNumber, CurrencyId,
    CurrencyId::{ForeignAsset, LendToken, Token},
    CurrencyInfo, Hash, Liquidity, Moment, Nonce, PriceDetail, Rate, Ratio, Shortfall, Signature, SignedFixedPoint,
    SignedInner, StablePoolId, TokenSymbol, UnsignedFixedPoint, UnsignedInner, DOT, IBTC, INTR, KBTC, KINT, KSM,
};

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
    state_version: 0,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

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
/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(2), u64::MAX);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 4096;
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

pub struct BaseCallFilter;

impl Contains<RuntimeCall> for BaseCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        if matches!(
            call,
            RuntimeCall::System(_)
                | RuntimeCall::Timestamp(_)
                | RuntimeCall::Sudo(_)
                | RuntimeCall::Democracy(_)
                | RuntimeCall::Escrow(_)
                | RuntimeCall::TechnicalCommittee(_)
        ) {
            // always allow core calls
            true
        } else {
            // normal operation: allow all calls that are not explicitly paused
            TxPause::contains(call)
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
    type DbWeight = RocksDbWeight;
    type BaseCallFilter = BaseCallFilter;
    type SystemWeightInfo = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = AuraAccountAdapter;
    type EventHandler = ();
}

pub struct AuraAccountAdapter;

impl FindAuthor<AccountId> for AuraAccountAdapter {
    fn find_author<'a, I>(digests: I) -> Option<AccountId>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        pallet_aura::AuraAuthorId::<Runtime>::find_author(digests).and_then(|k| AccountId::try_from(k.as_ref()).ok())
    }
}

parameter_types! {
    pub const MaxAuthorities: u32 = 32;
    pub const MaxSetIdSessionEntries: u32 = 0;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type KeyOwnerProofSystem = ();
    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type KeyOwnerIdentification =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;
    type HandleEquivocation = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

pub type SlowAdjustingFeeUpdate<R> =
    TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier, MaximumMultiplier>;

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
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
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

impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_utility::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type PalletsOrigin = OriginCaller;
}

parameter_types! {
    pub MinVestedTransfer: Balance = 0;
    pub const MaxVestingSchedules: u32 = 10;
}

impl orml_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = NativeCurrency;
    type MinVestedTransfer = MinVestedTransfer;
    // anyone can transfer vested tokens
    type VestedTransferOrigin = EnsureSigned<AccountId>;
    type WeightInfo = ();
    type MaxVestingSchedules = MaxVestingSchedules;
    type BlockNumberProvider = System;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 30;
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
    type Preimages = Preimage;
}

parameter_types! {
    pub PreimageBaseDepositz: Balance = deposit(2, 64); // todo: rename
    pub PreimageByteDepositz: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type Currency = NativeCurrency;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type BaseDeposit = PreimageBaseDepositz;
    type ByteDeposit = PreimageByteDepositz;
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

// https://github.com/paritytech/polkadot/blob/ece7544b40d8b29897f5aa799f27840dcc32f24d/runtime/polkadot/src/constants.rs#L18
pub const UNITS: Balance = NATIVE_TOKEN_ID.one();
pub const DOLLARS: Balance = UNITS; // 10_000_000_000
pub const CENTS: Balance = UNITS / 100; // 100_000_000
pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS
}

type EnsureRootOrAllTechnicalCommittee = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeInstance, 1, 1>,
>;

parameter_types! {
    pub const VotingPeriod: BlockNumber = 5 * MINUTES;
    pub const FastTrackVotingPeriod: BlockNumber = 1 * MINUTES;
    pub MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 3 * MINUTES;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
    pub LaunchOffsetMillis: u64 = 9 * 60 * 60 * 1000; // 9 hours offset, i.e. MON 9 AM
}

impl democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Scheduler = Scheduler;
    type Preimages = Preimage;
    type Currency = Escrow;
    type EnactmentPeriod = EnactmentPeriod;
    type VotingPeriod = VotingPeriod;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    type MinimumDeposit = MinimumDeposit;
    type MaxVotes = MaxVotes;
    type MaxProposals = MaxProposals;
    type MaxDeposits = ConstU32<100>;
    /// The technical committee can have any proposal be tabled immediately
    /// with a shorter voting period.
    type FastTrackOrigin = EnsureRootOrAllTechnicalCommittee;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
    type UnixTime = Timestamp;
    type Moment = Moment;
    type LaunchOffsetMillis = LaunchOffsetMillis;
    type TreasuryAccount = TreasuryAccount;
    type TreasuryCurrency = NativeCurrency;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub ProposalBondMinimum: Balance = 5;
    pub ProposalBondMaximum: Option<Balance> = None;
    pub const SpendPeriod: BlockNumber = 7 * DAYS;
    pub const Burn: Permill = Permill::from_percent(0);
    pub const MaxApprovals: u32 = 100;
    pub const MaxSpend: Balance = Balance::MAX;
}

parameter_types! {
    pub const TechnicalCommitteeMotionDuration: BlockNumber = 10 * MINUTES;
    pub const TechnicalCommitteeMaxProposals: u32 = 100;
    pub const TechnicalCommitteeMaxMembers: u32 = 100;
}

pub type TechnicalCommitteeInstance = pallet_collective::Instance1;

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
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = BITCOIN_BLOCK_SPACING;
}

impl btc_relay::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
}

const NATIVE_TOKEN_ID: TokenSymbol = INTR;
const NATIVE_CURRENCY_ID: CurrencyId = Token(NATIVE_TOKEN_ID);
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
    pub const VaultRegistryPalletId: PalletId = PalletId(*b"mod/vreg");
    pub const LoansPalletId: PalletId = PalletId(*b"mod/loan");
    pub const FarmingPalletId: PalletId = PalletId(*b"mod/farm");
}

parameter_types! {
    // 5EYCAe5i8QbRr5WN1PvaAVqPbfXsqazk9ocaxuzcTjgXPM1e
    pub FeeAccount: AccountId = FeePalletId::get().into_account_truncating();
    // 5EYCAe5i8QbRrUhwETaRvgif6H3HA84YQri7wjgLtKzRJCML
    pub SupplyAccount: AccountId = SupplyPalletId::get().into_account_truncating();
    // 5EYCAe5gXcgF6fT7oVsD7E4bfnRZeovzMUD2wkdyvCHrYQQE
    pub EscrowAnnuityAccount: AccountId = EscrowAnnuityPalletId::get().into_account_truncating();
    // 5EYCAe5jvsMTc6NLhunLTPVjJg5cZNweWKjNXKqz9RUqQJDY
    pub VaultAnnuityAccount: AccountId = VaultAnnuityPalletId::get().into_account_truncating();
    // 5EYCAe5i8QbRrWTk2CHjZA79gSf1piYSGm2LQfxaw6id3M88
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
    // 5EYCAe5i8QbRra1jndPz1WAuf1q1KHQNfu2cW1EXJ231emTd
    pub VaultRegistryAccount: AccountId = VaultRegistryPalletId::get().into_account_truncating();
    // 5EYCAe5i8QbRr5Umqbak9Zsapq3nkGBb2KjN9sBxTZjwf4eG
    pub FarmingAccount: AccountId = FarmingPalletId::get().into_account_truncating();
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
    vec![
        FeeAccount::get(),
        SupplyAccount::get(),
        EscrowAnnuityAccount::get(),
        VaultAnnuityAccount::get(),
        TreasuryAccount::get(),
        VaultRegistryAccount::get(),
        FarmingAccount::get(),
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

pub struct CurrencyHooks<T>(PhantomData<T>);
impl<T: orml_tokens::Config + loans::Config>
    MutationHooks<T::AccountId, T::CurrencyId, <T as currency::Config>::Balance> for CurrencyHooks<T>
where
    T::AccountId: From<sp_runtime::AccountId32>,
{
    type OnDust = orml_tokens::TransferDust<T, FeeAccount>;
    type OnSlash = OnSlashHook<T>;
    type PreDeposit = PreDeposit<T>;
    type PostDeposit = PostDeposit<T>;
    type PreTransfer = PreTransfer<T>;
    type PostTransfer = PostTransfer<T>;
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

impl orml_tokens::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type Amount = primitives::Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type CurrencyHooks = CurrencyHooks<Runtime>;
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = DustRemovalWhitelist;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
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
    type WeightInfo = ();
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
        let current_stake = <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::get_stake(&(), from)?;
        let new_stake = current_stake.saturating_add(amount);
        <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::set_stake(&(), from, new_stake)
    }

    fn distribute_block_reward(_from: &AccountId, amount: Balance) -> DispatchResult {
        <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::distribute_reward(
            &(),
            GetNativeCurrencyId::get(),
            amount,
        )
    }

    fn withdraw_reward(who: &AccountId) -> Result<Balance, DispatchError> {
        <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::withdraw_reward(
            &(),
            who,
            GetNativeCurrencyId::get(),
        )
    }
}

pub type EscrowAnnuityInstance = annuity::Instance1;

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
    fn deposit_stake(_from: &AccountId, amount: Balance) -> DispatchResult {
        // since this is only used for benchmarking
        // deposit stake for the native currency
        <VaultCapacity as reward::RewardsApi<(), CurrencyId, Balance>>::deposit_stake(
            &(),
            &GetNativeCurrencyId::get(),
            amount,
        )
    }

    fn distribute_block_reward(from: &AccountId, amount: Balance) -> DispatchResult {
        // TODO: remove fee pallet?
        Self::Currency::transfer(from, &FeeAccount::get(), amount, ExistenceRequirement::KeepAlive)?;
        <VaultCapacity as reward::RewardsApi<(), CurrencyId, Balance>>::distribute_reward(
            &(),
            GetNativeCurrencyId::get(),
            amount,
        )
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn can_withdraw_reward() -> bool {
        false
    }

    fn withdraw_reward(_: &AccountId) -> Result<Balance, DispatchError> {
        Err(DispatchError::Other("Unsupported"))
    }
}

pub type VaultAnnuityInstance = annuity::Instance2;

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

pub type EscrowRewardsInstance = reward::Instance1;

impl reward::Config<EscrowRewardsInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = ();
    type StakeId = AccountId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

pub type VaultRewardsInstance = reward::Instance2;

impl reward::Config<VaultRewardsInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = CurrencyId;
    type StakeId = VaultId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

pub type VaultCapacityInstance = reward::Instance3;

impl reward::Config<VaultCapacityInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = ();
    type StakeId = CurrencyId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

type FarmingRewardsInstance = reward::Instance4;

impl reward::Config<FarmingRewardsInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = CurrencyId;
    type StakeId = AccountId;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

parameter_types! {
    pub const RewardPeriod: BlockNumber = MINUTES;
}

impl farming::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type FarmingPalletId = FarmingPalletId;
    type TreasuryAccountId = TreasuryAccount;
    type RewardPeriod = RewardPeriod;
    type RewardPools = FarmingRewards;
    type MultiCurrency = Tokens;
    type WeightInfo = ();
}

impl security::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxErrors = ConstU32<1>;
}

impl currency::Config for Runtime {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetRelayChainCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = currency::CurrencyConvert<Runtime, Oracle, Loans>;
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
    pub const MaxPeriod: BlockNumber = WEEKS * 52 * 4;
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

parameter_types! {
    pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
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
    type Slashed = runtime_common::ToTreasury<Runtime, TreasuryAccount, NativeCurrency>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type RegistrarOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 8);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const MaxProxies: u16 = 32;
    pub const MaxPending: u16 = 32;
    pub const AnnouncementDepositBase: Balance = deposit(1, 8);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    codec::Encode,
    codec::Decode,
    sp_runtime::RuntimeDebug,
    codec::MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    Any,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            // Nested calls get checked against this filter during
            // execution (i.e. not before) this will result in a
            // `BadOrigin` error if the proxy does not allow the call
            _ if matches!(c, RuntimeCall::Utility(..)) => true,
            ProxyType::Any => true,
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (x, y) if x == y => true,
            (ProxyType::Any, _) => true,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = NativeCurrency;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = ();
    type MaxPending = MaxPending;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

impl vault_registry::Config for Runtime {
    type PalletId = VaultRegistryPalletId;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
    type NominationApi = Nomination;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = UncheckedExtrinsic;
}

pub type OracleName = oracle::NameOf<Runtime>;

impl oracle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnExchangeRateChange = (vault_registry::PoolManager<Runtime>, Loans);
    type WeightInfo = ();
    type MaxNameLength = ConstU32<255>;
}

parameter_types! {
    pub const MaxExpectedValue: UnsignedFixedPoint = UnsignedFixedPoint::from_inner(<UnsignedFixedPoint as FixedPointNumber>::DIV);
}

impl fee::Config for Runtime {
    type FeePalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CapacityRewards = VaultCapacity;
    type VaultRewards = VaultRewards;
    type VaultStaking = VaultStaking;
    type OnSweep = currency::SweepFunds<Runtime, FeeAccount>;
    type MaxExpectedValue = MaxExpectedValue;
}

pub use issue::{Event as IssueEvent, IssueRequest};

impl issue::Config for Runtime {
    type TreasuryPalletId = TreasuryPalletId;
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

pub struct AssetAuthority;
impl EnsureOriginWithArg<RuntimeOrigin, Option<u32>> for AssetAuthority {
    type Success = ();

    fn try_origin(origin: RuntimeOrigin, _asset_id: &Option<u32>) -> Result<Self::Success, RuntimeOrigin> {
        EnsureRoot::try_origin(origin)
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_: &Option<u32>) -> Result<RuntimeOrigin, ()> {
        EnsureRoot::try_successful_origin()
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

impl clients_info::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxNameLength = ConstU32<255>;
    type MaxUriLength = ConstU32<255>;
}

impl loans::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = LoansPalletId;
    type ReserveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type UnixTime = Timestamp;
    type RewardAssetId = GetNativeCurrencyId;
    type ReferenceAssetId = GetWrappedCurrencyId;
    type OnExchangeRateChange = vault_registry::PoolManager<Runtime>;
}

parameter_types! {
    pub const MaxNameLen: u32 = 128;
    pub const PauseTooLongNames: bool = false;
}

impl tx_pause::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PauseOrigin = EnsureRoot<AccountId>;
    type UnpauseOrigin = EnsureRoot<AccountId>;
    type WhitelistCallNames = Nothing;
    type MaxNameLen = MaxNameLen;
    type PauseTooLongNames = PauseTooLongNames;
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
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 4,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 5,
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 6,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 7,
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 8,
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 9,
        TxPause: tx_pause::{Pallet, Call, Storage, Event<T>} = 10,

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
        VaultCapacity: reward::<Instance3>::{Pallet, Storage, Event<T>} = 43,

        Farming: farming::{Pallet, Call, Storage, Event<T>} = 44,
        FarmingRewards: reward::<Instance4>::{Pallet, Storage, Event<T>} = 45,

        // # Bitcoin SPV
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>} = 50,

        // # Operational
        Security: security::{Pallet, Call, Config, Storage, Event<T>} = 60,
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned} = 61,
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>} = 62,
        Issue: issue::{Pallet, Call, Config<T>, Storage, Event<T>} = 63,
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>} = 64,
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>} = 65,
        Fee: fee::{Pallet, Call, Config<T>, Storage} = 66,
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>} = 67,
        ClientsInfo: clients_info::{Pallet, Call, Storage, Event<T>} = 68,

        // # Governance
        Democracy: democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 70,
        TechnicalCommittee: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 71,
        TechnicalMembership: pallet_membership::{Pallet, Call, Storage, Event<T>, Config<T>} = 72,

        Authorship: pallet_authorship::{Pallet, Storage} = 80,
        Aura: pallet_aura::{Pallet, Config<T>} = 81,
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event} = 82,

        // # Lending
        Loans: loans::{Pallet, Call, Storage, Event<T>, Config} = 100,
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
pub type Executive =
    frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

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

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
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

            list_benchmark!(list, extra, annuity, EscrowAnnuity);
            list_benchmark!(list, extra, btc_relay, BTCRelay);
            list_benchmark!(list, extra, escrow, Escrow);
            list_benchmark!(list, extra, fee, Fee);
            list_benchmark!(list, extra, farming, Farming);
            list_benchmark!(list, extra, issue, Issue);
            list_benchmark!(list, extra, nomination, Nomination);
            list_benchmark!(list, extra, oracle, Oracle);
            list_benchmark!(list, extra, redeem, Redeem);
            list_benchmark!(list, extra, replace, Replace);
            list_benchmark!(list, extra, vault_registry, VaultRegistry);
            list_benchmark!(list, extra, security, Security);

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

            add_benchmark!(params, batches, annuity, EscrowAnnuity);
            add_benchmark!(params, batches, btc_relay, BTCRelay);
            add_benchmark!(params, batches, escrow, Escrow);
            add_benchmark!(params, batches, fee, Fee);
            add_benchmark!(params, batches, farming, Farming);
            add_benchmark!(params, batches, issue, Issue);
            add_benchmark!(params, batches, nomination, Nomination);
            add_benchmark!(params, batches, oracle, Oracle);
            add_benchmark!(params, batches, redeem, Redeem);
            add_benchmark!(params, batches, replace, Replace);
            add_benchmark!(params, batches, vault_registry, VaultRegistry);
            add_benchmark!(params, batches, security, Security);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl btc_relay_rpc_runtime_api::BtcRelayApi<
        Block,
        H256Le,
    > for Runtime {
        fn verify_block_header_inclusion(block_hash: H256Le) -> Result<(), DispatchError> {
            BTCRelay::verify_block_header_inclusion(block_hash, None).map(|_| ())
        }
    }

    impl oracle_rpc_runtime_api::OracleApi<
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

    impl vault_registry_rpc_runtime_api::VaultRegistryApi<
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

    impl escrow_rpc_runtime_api::EscrowApi<
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

    impl reward_rpc_runtime_api::RewardApi<
        Block,
        AccountId,
        VaultId,
        CurrencyId,
        Balance,
        BlockNumber,
        UnsignedFixedPoint
    > for Runtime {
        fn compute_escrow_reward(account_id: AccountId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount = <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::compute_reward(&(), &account_id, currency_id)?;
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
        }

        fn compute_farming_reward(account_id: AccountId, pool_currency_id: CurrencyId, reward_currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount = <FarmingRewards as reward::RewardsApi<CurrencyId, AccountId, Balance>>::compute_reward(&pool_currency_id, &account_id, reward_currency_id)?;
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
        }

        fn compute_vault_reward(vault_id: VaultId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError> {
            let amount = Fee::compute_vault_rewards(&vault_id, &vault_id.account_id, currency_id)?.amount();
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
        }

        fn estimate_escrow_reward_rate(
            account_id: AccountId,
            amount: Option<Balance>,
            lock_time: Option<BlockNumber>,
        ) -> Result<UnsignedFixedPoint, DispatchError> {
            // withdraw previous rewards
            <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::withdraw_reward(&(), &account_id, NATIVE_CURRENCY_ID)?;
            // increase amount and/or lock_time
            Escrow::round_height_and_deposit_for(&account_id, amount.unwrap_or_default(), lock_time.unwrap_or_default())?;
            // distribute rewards accrued over block count
            let reward = EscrowAnnuity::min_reward_per_block().saturating_mul(YEARS.into());
            <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::distribute_reward(&(), NATIVE_CURRENCY_ID, reward)?;
            let received = <EscrowRewards as reward::RewardsApi<(), AccountId, Balance>>::compute_reward(&(), &account_id, NATIVE_CURRENCY_ID)?;
            // NOTE: total_locked is same currency as rewards
            let total_locked = Escrow::locked_balance(&account_id).amount;
            // rate is received / total_locked
            Ok(UnsignedFixedPoint::checked_from_rational(received, total_locked).unwrap_or_default())
        }

        fn estimate_farming_reward(
            account_id: AccountId,
            pool_currency_id: CurrencyId,
            reward_currency_id: CurrencyId,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            <FarmingRewards as reward::RewardsApi<CurrencyId, AccountId, Balance>>::withdraw_reward(&pool_currency_id, &account_id, reward_currency_id)?;
            <FarmingRewards as reward::RewardsApi<CurrencyId, AccountId, Balance>>::distribute_reward(&pool_currency_id, reward_currency_id, Farming::total_rewards(&pool_currency_id, &reward_currency_id))?;
            let amount = <FarmingRewards as reward::RewardsApi<CurrencyId, AccountId, Balance>>::compute_reward(&pool_currency_id, &account_id, reward_currency_id)?;
            let balance = BalanceWrapper::<Balance> { amount };
            Ok(balance)
        }

        fn estimate_vault_reward_rate(
            vault_id: VaultId,
        ) -> Result<UnsignedFixedPoint, DispatchError> {
            runtime_common::estimate_vault_reward_rate::<Runtime, VaultAnnuityInstance, VaultStaking, VaultCapacity, _>(vault_id)
        }
    }

    impl issue_rpc_runtime_api::IssueApi<
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

    impl redeem_rpc_runtime_api::RedeemApi<
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

    impl replace_rpc_runtime_api::ReplaceApi<
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

    impl loans_rpc_runtime_api::LoansApi<
        Block,
        AccountId,
        Balance,
    > for Runtime {
        fn get_account_liquidity(account: AccountId) -> Result<(Liquidity, Shortfall), DispatchError> {
            Loans::get_account_liquidity(&account)
            .and_then(|liquidity| liquidity.to_rpc_tuple())
        }

        fn get_market_status(asset_id: CurrencyId) -> Result<(Rate, Rate, Rate, Ratio, Balance, Balance, FixedU128), DispatchError> {
            Loans::get_market_status(asset_id)
        }

        fn get_liquidation_threshold_liquidity(account: AccountId) -> Result<(Liquidity, Shortfall), DispatchError> {
            Loans::get_account_liquidation_threshold_liquidity(&account)
            .and_then(|liquidity| liquidity.to_rpc_tuple())
        }
    }

    impl dex_general_rpc_runtime_api::DexGeneralApi<Block, AccountId, CurrencyId> for Runtime {
        fn get_pair_by_asset_id(
            _asset_0: CurrencyId,
            _asset_1: CurrencyId
        ) -> Option<dex_general::PairInfo<AccountId, dex_general::AssetBalance, CurrencyId>> {
            Default::default()
        }

        fn get_amount_in_price(
            _supply: dex_general::AssetBalance,
            _path: Vec<CurrencyId>
        ) -> dex_general::AssetBalance {
            Default::default()
        }

        fn get_amount_out_price(
            _supply: dex_general::AssetBalance,
            _path: Vec<CurrencyId>
        ) -> dex_general::AssetBalance {
            Default::default()
        }

        fn get_estimate_lptoken(
            _asset_0: CurrencyId,
            _asset_1: CurrencyId,
            _amount_0_desired: dex_general::AssetBalance,
            _amount_1_desired: dex_general::AssetBalance,
            _amount_0_min: dex_general::AssetBalance,
            _amount_1_min: dex_general::AssetBalance,
        ) -> dex_general::AssetBalance {
            Default::default()
        }

        fn calculate_remove_liquidity(
            _asset_0: CurrencyId,
            _asset_1: CurrencyId,
            _amount: dex_general::AssetBalance,
        ) -> Option<(dex_general::AssetBalance, dex_general::AssetBalance)> {
            Default::default()
        }
    }

    impl dex_stable_rpc_runtime_api::DexStableApi<Block, CurrencyId, Balance, AccountId, StablePoolId> for Runtime {
        fn get_virtual_price(_pool_id: StablePoolId) -> Balance {
            Default::default()
        }

        fn get_a(_pool_id: StablePoolId) -> Balance {
            Default::default()
        }

        fn get_a_precise(_pool_id: StablePoolId) -> Balance {
            Default::default()
        }

        fn get_currencies(_pool_id: StablePoolId) -> Vec<CurrencyId> {
            Default::default()
        }

        fn get_currency(_pool_id: StablePoolId, _index: u32) -> Option<CurrencyId> {
            Default::default()
        }

        fn get_lp_currency(_pool_id: StablePoolId) -> Option<CurrencyId> {
            Default::default()
        }

        fn get_currency_precision_multipliers(_pool_id: StablePoolId) -> Vec<Balance> {
            Default::default()
        }

        fn get_currency_balances(_pool_id: StablePoolId) -> Vec<Balance> {
            Default::default()
        }

        fn get_number_of_currencies(_pool_id: StablePoolId) -> u32 {
            Default::default()
        }

        fn get_admin_balances(_pool_id: StablePoolId) -> Vec<Balance> {
            Default::default()
        }

        fn calculate_currency_amount(_pool_id: StablePoolId, _amounts: Vec<Balance>, _deposit: bool) -> Balance {
            Default::default()
        }

        fn calculate_swap(_pool_id: StablePoolId, _in_index: u32, _out_index: u32, _in_amount: Balance) -> Balance {
            Default::default()
        }

        fn calculate_remove_liquidity(_pool_id: StablePoolId, _amount: Balance) -> Vec<Balance> {
            Default::default()
        }

        fn calculate_remove_liquidity_one_currency(_pool_id: StablePoolId, _amount: Balance, _index: u32) -> Balance {
            Default::default()
        }
    }
}
