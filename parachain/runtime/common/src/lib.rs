#![cfg_attr(not(feature = "std"), no_std)]
use currency::Amount;
use primitives::BlockNumber;
use sp_runtime::{traits::Get as _, DispatchError, FixedPointNumber};
use sp_std::prelude::*;

// The relay chain is limited to 12s to include parachain blocks.
pub const MILLISECS_PER_BLOCK: u64 = 12000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
pub const YEARS: BlockNumber = DAYS * 365;
use primitives::UnsignedFixedPoint;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type VaultId<T> = primitives::VaultId<AccountId<T>, currency::CurrencyId<T>>;
pub use currency::CurrencyId;
use primitives::{Balance, Nonce};

fn native_currency_id<T: currency::Config>() -> CurrencyId<T> {
    T::GetNativeCurrencyId::get()
}

pub fn estimate_vault_reward_rate<T, VaultAnnuityInstance, VaultStakingApi, VaultCapacityApi, VaultAnnuityCurrency>(
    vault_id: VaultId<T>,
) -> Result<UnsignedFixedPoint, DispatchError>
where
    T: oracle::Config
        + currency::Config<UnsignedFixedPoint = UnsignedFixedPoint, Balance = Balance>
        + fee::Config<UnsignedFixedPoint = UnsignedFixedPoint>
        + annuity::Config<VaultAnnuityInstance, Currency = VaultAnnuityCurrency>,
    VaultStakingApi: reward::RewardsApi<(Option<Nonce>, VaultId<T>), AccountId<T>, Balance, CurrencyId = CurrencyId<T>>,
    VaultCapacityApi: reward::RewardsApi<(), CurrencyId<T>, Balance, CurrencyId = CurrencyId<T>>,
    VaultAnnuityInstance: 'static,
    VaultAnnuityCurrency:
        frame_support::traits::tokens::currency::Currency<<T as frame_system::Config>::AccountId, Balance = Balance>,
{
    // distribute and withdraw previous rewards
    let native_currency = native_currency_id::<T>();
    fee::Pallet::<T>::distribute_vault_rewards(&vault_id, native_currency)?;
    // distribute rewards accrued over block count
    VaultStakingApi::withdraw_reward(&(None, vault_id.clone()), &vault_id.account_id, native_currency)?;
    let reward = annuity::Pallet::<T, VaultAnnuityInstance>::min_reward_per_block().saturating_mul(YEARS.into());
    VaultCapacityApi::distribute_reward(&(), native_currency, reward)?;
    Amount::<T>::new(reward, native_currency).mint_to(&fee::Pallet::<T>::fee_pool_account_id())?;
    // compute and convert rewards
    let received = fee::Pallet::<T>::compute_vault_rewards(&vault_id, &vault_id.account_id, native_currency)?;
    let received_as_wrapped = oracle::Pallet::<T>::collateral_to_wrapped(received, native_currency)?;
    // convert collateral stake to same currency
    let collateral = VaultStakingApi::get_stake(&(None, vault_id.clone()), &vault_id.account_id)?;
    let collateral_as_wrapped = oracle::Pallet::<T>::collateral_to_wrapped(collateral, vault_id.collateral_currency())?; // rate is received / collateral
    Ok(UnsignedFixedPoint::checked_from_rational(received_as_wrapped, collateral_as_wrapped).unwrap_or_default())
}

#[macro_export]
macro_rules! impl_issue_config {
    () => {
        impl issue::Config for Runtime {
            type TreasuryPalletId = TreasuryPalletId;
            type RuntimeEvent = RuntimeEvent;
            type BlockNumberToBalance = BlockNumberToBalance;
            type WeightInfo = ();
        }
    };
}

// #[macro_export]
// macro_rules! impl_issue_config {
//     () => {
//     }
// }

#[macro_export]
macro_rules! impl_redeem_config {
    () => {
        impl redeem::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_replace_config {
    () => {
        impl replace::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_nomination_config {
    () => {
        impl nomination::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_clients_info_config {
    () => {
        impl clients_info::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_frame_system_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_authorship_config {
    () => {
        impl pallet_authorship::Config for Runtime {
            type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
            type UncleGenerations = UncleGenerations;
            type FilterUncle = ();
            type EventHandler = (CollatorSelection,);
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_session_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_collator_selection_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_aura_config {
    () => {
        impl pallet_aura::Config for Runtime {
            type AuthorityId = AuraId;
            type DisabledValidators = ();
            type MaxAuthorities = MaxAuthorities;
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_timestamp_config {
    () => {
        impl pallet_timestamp::Config for Runtime {
            /// A timestamp: milliseconds since the unix epoch.
            type Moment = Moment;
            type OnTimestampSet = Aura;
            type MinimumPeriod = MinimumPeriod;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_transaction_payment_config {
    () => {
        impl pallet_transaction_payment::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type OnChargeTransaction =
                pallet_transaction_payment::CurrencyAdapter<NativeCurrency, DealWithFees<Runtime, GetNativeCurrencyId>>;
            type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
            type WeightToFee = IdentityFee<Balance>;
            type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
            type OperationalFeeMultiplier = OperationalFeeMultiplier;
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_sudo_config {
    () => {
        impl pallet_sudo::Config for Runtime {
            type RuntimeCall = RuntimeCall;
            type RuntimeEvent = RuntimeEvent;
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_utility_config {
    () => {
        impl pallet_utility::Config for Runtime {
            type RuntimeCall = RuntimeCall;
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
            type PalletsOrigin = OriginCaller;
        }
    };
}

#[macro_export]
macro_rules! impl_orml_vesting_config {
    () => {
        impl orml_vesting::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type Currency = NativeCurrency;
            type MinVestedTransfer = MinVestedTransfer;
            type VestedTransferOrigin = EnsureKintsugiLabs;
            type WeightInfo = ();
            type MaxVestingSchedules = MaxVestingSchedules;
            type BlockNumberProvider = System;
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_scheduler_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_preimage_config {
    () => {
        impl pallet_preimage::Config for Runtime {
            type WeightInfo = ();
            type RuntimeEvent = RuntimeEvent;
            type Currency = NativeCurrency;
            type ManagerOrigin = EnsureRoot<AccountId>;
            type BaseDeposit = PreimageBaseDepositz;
            type ByteDeposit = PreimageByteDepositz;
        }
    };
}

#[macro_export]
macro_rules! impl_democracy_config {
    () => {
        impl democracy::Config for Runtime {
            type Proposal = RuntimeCall;
            type RuntimeEvent = RuntimeEvent;
            type Currency = Escrow;
            type EnactmentPeriod = EnactmentPeriod;
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
            type UnixTime = Timestamp;
            type Moment = Moment;
            type LaunchOffsetMillis = LaunchOffsetMillis;
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_multisig_config {
    () => {
        impl pallet_multisig::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type RuntimeCall = RuntimeCall;
            type Currency = NativeCurrency;
            type DepositBase = GetDepositBase;
            type DepositFactor = GetDepositFactor;
            type MaxSignatories = GetMaxSignatories;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_treasury_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_collective_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_membership_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_cumulus_pallet_parachain_system_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_parachain_info_config {
    () => {
        impl parachain_info::Config for Runtime {}
    };
}

#[macro_export]
macro_rules! impl_cumulus_pallet_aura_ext_config {
    () => {
        impl cumulus_pallet_aura_ext::Config for Runtime {}
    };
}

#[macro_export]
macro_rules! impl_orml_unknown_tokens_config {
    () => {
        impl orml_unknown_tokens::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
        }
    };
}

#[macro_export]
macro_rules! impl_btc_relay_config {
    () => {
        impl btc_relay::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type WeightInfo = ();
            type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
        }
    };
}

#[macro_export]
macro_rules! impl_orml_tokens_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_orml_asset_registry_config {
    () => {
        impl orml_asset_registry::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type Balance = Balance;
            type CustomMetadata = primitives::CustomMetadata;
            type AssetProcessor = SequentialId<Runtime>;
            type AssetId = primitives::ForeignAssetId;
            type AuthorityOrigin = AssetAuthority;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_supply_config {
    () => {
        impl supply::Config for Runtime {
            type SupplyPalletId = SupplyPalletId;
            type RuntimeEvent = RuntimeEvent;
            type UnsignedFixedPoint = UnsignedFixedPoint;
            type Currency = NativeCurrency;
            type InflationPeriod = InflationPeriod;
            type OnInflation = DealWithRewards;
        }
    };
}

#[macro_export]
macro_rules! impl_reward_config {
    () => {
        impl reward::Config<EscrowRewardsInstance> for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type SignedFixedPoint = SignedFixedPoint;
            type PoolId = ();
            type StakeId = AccountId;
            type CurrencyId = CurrencyId;
            type GetNativeCurrencyId = GetNativeCurrencyId;
            type GetWrappedCurrencyId = GetWrappedCurrencyId;
        }
    };
}

#[macro_export]
macro_rules! impl_security_config {
    () => {
        impl security::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
        }
    };
}

#[macro_export]
macro_rules! impl_currency_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_staking_config {
    () => {
        impl staking::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type SignedFixedPoint = SignedFixedPoint;
            type SignedInner = SignedInner;
            type CurrencyId = CurrencyId;
            type GetNativeCurrencyId = GetNativeCurrencyId;
        }
    };
}

#[macro_export]
macro_rules! impl_escrow_config {
    () => {
        impl escrow::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type BlockNumberToBalance = BlockNumberToBalance;
            type Currency = NativeCurrency;
            type Span = Span;
            type MaxPeriod = MaxPeriod;
            type EscrowRewards = EscrowRewards;
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_pallet_identity_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_pallet_proxy_config {
    () => {
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
    };
}

#[macro_export]
macro_rules! impl_vault_registry_config {
    () => {
        impl vault_registry::Config for Runtime {
            type PalletId = VaultRegistryPalletId;
            type RuntimeEvent = RuntimeEvent;
            type Balance = Balance;
            type WeightInfo = ();
            type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
            type NominationApi = Nomination;
        }
    };
}

#[macro_export]
macro_rules! impl_oracle_config {
    () => {
        impl oracle::Config for Runtime {
            type RuntimeEvent = RuntimeEvent;
            type OnExchangeRateChange = ();
            type WeightInfo = ();
        }
    };
}

#[macro_export]
macro_rules! impl_fee_config {
    () => {
        impl fee::Config for Runtime {
            type FeePalletId = FeePalletId;
            type WeightInfo = ();
            type SignedFixedPoint = SignedFixedPoint;
            type SignedInner = SignedInner;
            type UnsignedFixedPoint = UnsignedFixedPoint;
            type UnsignedInner = UnsignedInner;
            type CapacityRewards = VaultCapacity;
            type VaultRewards = VaultRewards;
            type VaultStaking = VaultStaking;
            type OnSweep = currency::SweepFunds<Runtime, FeeAccount>;
            type MaxExpectedValue = MaxExpectedValue;
        }
    };
}
#[macro_export]
macro_rules! impl_tx_pause_config {
    () => {
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
    }
}