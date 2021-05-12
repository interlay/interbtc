//! # Vault Registry Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod types;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::types::{
    Backing, BtcAddress, DefaultSystemVault, DefaultVault, Inner, Issuing, RichSystemVault, RichVault,
    UnsignedFixedPoint, UpdatableVault, VaultStatusV1, VaultV1, Version,
};
#[doc(inline)]
pub use crate::types::{BtcPublicKey, CurrencySource, SystemVault, Vault, VaultStatus, Wallet};
use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Get, Randomness},
    transactional,
};
use frame_system::{
    ensure_signed,
    offchain::{SendTransactionTypes, SubmitTransaction},
};
use primitive_types::U256;
use sp_arithmetic::{traits::*, FixedPointNumber};
use sp_core::H256;
use sp_runtime::{
    traits::AccountIdConversion,
    transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction},
    ModuleId,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    vec::Vec,
};

// value taken from https://github.com/substrate-developer-hub/recipes/blob/master/pallets/ocw-demo/src/lib.rs
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RegisterRequest<AccountId, DateTime> {
    registration_id: H256,
    vault: AccountId,
    timeout: DateTime,
}

#[derive(Encode, Decode, Eq, PartialEq)]
pub enum LiquidationTarget {
    OperatorsOnly,
    NonOperatorsOnly,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + currency::Config<currency::Collateral>
        + currency::Config<currency::Treasury>
        + exchange_rate_oracle::Config
        + security::Config
        + SendTransactionTypes<Call<Self>>
    {
        /// The vault module id, used for deriving its sovereign account ID.
        #[pallet::constant] // put the constant in metadata
        type ModuleId: Get<ModuleId>;

        /// The overarching event type.
        type Event: From<Event<Self>>
            + Into<<Self as frame_system::Config>::Event>
            + IsType<<Self as frame_system::Config>::Event>;

        /// The source of (pseudo) randomness. Set to collective flip
        type RandomnessSource: Randomness<H256, Self::BlockNumber>;

        /// the type of unsigned fixedpoint to use for the different thresholds. Set to FixedU128
        type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode + MaybeSerializeDeserialize;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(n: T::BlockNumber) {
            log::info!("Off-chain worker started on block {:?}", n);
            Self::_offchain_worker();
        }

        fn on_runtime_upgrade() -> Weight {
            use frame_support::{migration::StorageKeyIterator, storage::generator::StorageMap, Blake2_128Concat};

            if matches!(Self::storage_version(), Version::V0 | Version::V1) {
                StorageKeyIterator::<
                    T::AccountId,
                    VaultV1<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>,
                    Blake2_128Concat,
                >::new(Vaults::<T>::module_prefix(), b"Vaults")
                .drain()
                .for_each(|(id, legacy_vault)| {
                    let new_vault = Vault {
                        id: legacy_vault.id,
                        to_be_issued_tokens: legacy_vault.to_be_issued_tokens,
                        issued_tokens: legacy_vault.issued_tokens,
                        to_be_redeemed_tokens: legacy_vault.to_be_redeemed_tokens,
                        wallet: legacy_vault.wallet,
                        backing_collateral: legacy_vault.backing_collateral,
                        banned_until: legacy_vault.banned_until,

                        // unaccepted V1 replace requests are closed upon upgrade
                        replace_collateral: 0u32.into(),
                        to_be_replaced_tokens: 0u32.into(),

                        status: {
                            match legacy_vault.status {
                                VaultStatusV1::Active => VaultStatus::Active(true),
                                VaultStatusV1::Liquidated => VaultStatus::Liquidated,
                                VaultStatusV1::CommittedTheft => VaultStatus::CommittedTheft,
                            }
                        },
                    };
                    Vaults::<T>::insert(id, new_vault);
                });

                let next_account_id = Self::liquidation_vault_account_id();
                let prev_account_id = LiquidationVaultAccountId::<T>::take();

                // transfer collateral
                let amount_backing = ext::collateral::get_free_balance::<T>(&prev_account_id);
                ext::collateral::transfer::<T>(&prev_account_id, &next_account_id, amount_backing)
                    .expect("failed to transfer collateral");

                // tranfer tokens
                let amount_btc = ext::treasury::get_free_balance::<T>(prev_account_id.clone());
                ext::treasury::transfer::<T>(prev_account_id, next_account_id, amount_btc)
                    .expect("failed to transfer tokens");

                StorageVersion::<T>::put(Version::V2);
            }

            0
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match source {
                TransactionSource::External => {
                    // receiving unsigned transaction from network - disallow
                    return InvalidTransaction::Call.into();
                }
                TransactionSource::Local => {}   // produced by off-chain worker
                TransactionSource::InBlock => {} // some other node included it in a block
            };

            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("vault-registry")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(false)
                    .build()
            };

            match call {
                Call::report_undercollateralized_vault(_) => valid_tx(b"report_undercollateralized_vault".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initiates the registration procedure for a new Vault.
        /// The Vault provides its BTC address and locks up collateral,
        /// which is to be used to the issuing process.
        ///
        /// # Arguments
        /// * `collateral` - the amount of collateral to lock
        /// * `public_key` - the BTC public key of the vault to register
        ///
        /// # Errors
        /// * `InsufficientVaultCollateralAmount` - if the collateral is below the minimum threshold
        /// * `VaultAlreadyRegistered` - if a vault is already registered for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[pallet::weight(<T as Config>::WeightInfo::register_vault())]
        #[transactional]
        pub fn register_vault(
            origin: OriginFor<T>,
            #[pallet::compact] collateral: Backing<T>,
            public_key: BtcPublicKey,
        ) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            Self::_register_vault(&ensure_signed(origin)?, collateral, public_key)?;
            Ok(().into())
        }

        /// Locks additional collateral as a security against stealing the
        /// Bitcoin locked with it.
        ///
        /// # Arguments
        /// * `amount` - the amount of extra collateral to lock
        ///
        /// # Errors
        /// * `VaultNotFound` - if no vault exists for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[pallet::weight(<T as Config>::WeightInfo::lock_additional_collateral())]
        #[transactional]
        pub fn lock_additional_collateral(
            origin: OriginFor<T>,
            #[pallet::compact] amount: Backing<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            Self::_lock_additional_collateral(&sender, amount)?;

            let vault = Self::get_active_rich_vault_from_id(&sender)?;

            Self::deposit_event(Event::<T>::LockAdditionalCollateral(
                vault.id(),
                amount,
                vault.get_collateral(),
                vault.get_free_collateral()?,
            ));
            Ok(().into())
        }

        /// Withdraws `amount` of the collateral from the amount locked by
        /// the vault corresponding to the origin account
        /// The collateral left after withdrawal must be more
        /// (free or used in backing issued tokens) than MinimumCollateralVault
        /// and above the SecureCollateralThreshold. Collateral that is currently
        /// being used to back issued tokens remains locked until the Vault
        /// is used for a redeem request (full release can take multiple redeem requests).
        ///
        /// # Arguments
        /// * `amount` - the amount of collateral to withdraw
        ///
        /// # Errors
        /// * `VaultNotFound` - if no vault exists for the origin account
        /// * `InsufficientCollateralAvailable` - if the vault does not own enough collateral
        #[pallet::weight(<T as Config>::WeightInfo::withdraw_collateral())]
        #[transactional]
        pub fn withdraw_collateral(
            origin: OriginFor<T>,
            #[pallet::compact] amount: Backing<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let is_operator = IsNominationOperator::<T>::get(sender.clone());
            ensure!(!is_operator, Error::<T>::NominationOperatorCannotWithdrawDirectly);
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            Self::try_withdraw_collateral(&sender, amount)?;

            let vault = Self::get_active_rich_vault_from_id(&sender)?;

            Self::deposit_event(Event::<T>::WithdrawCollateral(sender, amount, vault.get_collateral()));
            Ok(().into())
        }

        /// Registers a new Bitcoin address for the vault.
        ///
        /// # Arguments
        /// * `public_key` - the BTC public key of the vault to update
        #[pallet::weight(<T as Config>::WeightInfo::update_public_key())]
        #[transactional]
        pub fn update_public_key(origin: OriginFor<T>, public_key: BtcPublicKey) -> DispatchResultWithPostInfo {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let mut vault = Self::get_active_rich_vault_from_id(&account_id)?;
            vault.update_public_key(public_key.clone());
            Self::deposit_event(Event::<T>::UpdatePublicKey(account_id, public_key));
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::register_address())]
        #[transactional]
        pub fn register_address(origin: OriginFor<T>, btc_address: BtcAddress) -> DispatchResultWithPostInfo {
            let account_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            Self::insert_vault_deposit_address(&account_id, btc_address)?;
            Self::deposit_event(Event::<T>::RegisterAddress(account_id, btc_address));
            Ok(().into())
        }

        /// Configures whether or not the vault accepts new issues.
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction (i.e. the vault)
        /// * `accept_new_issues` - true indicates that the vault accepts new issues
        ///
        /// # Weight: `O(1)`
        #[pallet::weight(<T as Config>::WeightInfo::accept_new_issues())]
        #[transactional]
        pub fn accept_new_issues(origin: OriginFor<T>, accept_new_issues: bool) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;
            let vault_id = ensure_signed(origin)?;
            let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
            vault.set_accept_new_issues(accept_new_issues)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::report_undercollateralized_vault())]
        #[transactional]
        fn report_undercollateralized_vault(
            _origin: OriginFor<T>,
            vault_id: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            log::info!("Vault reported");
            let vault = Self::get_vault_from_id(&vault_id)?;
            let liquidation_threshold = <LiquidationCollateralThreshold<T>>::get();
            if Self::is_vault_below_liquidation_threshold(&vault, liquidation_threshold)? {
                Self::liquidate_vault(&vault_id)?;
                Ok(().into())
            } else {
                log::info!("Not liquidating; vault not below liquidation threshold");
                Err(Error::<T>::VaultNotBelowLiquidationThreshold.into())
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", Backing<T> = "Backing", Issuing<T> = "Issuing")]
    pub enum Event<T: Config> {
        RegisterVault(T::AccountId, Backing<T>),
        /// vault_id, new collateral, total collateral, free collateral
        LockAdditionalCollateral(T::AccountId, Backing<T>, Backing<T>, Backing<T>),
        /// vault_id, withdrawn collateral, total collateral
        WithdrawCollateral(T::AccountId, Backing<T>, Backing<T>),
        /// vault_id, new public key
        UpdatePublicKey(T::AccountId, BtcPublicKey),
        /// vault_id, new address
        RegisterAddress(T::AccountId, BtcAddress),
        /// vault_id, additional to-be-issued tokens
        IncreaseToBeIssuedTokens(T::AccountId, Issuing<T>),
        /// vault_id, decrease in to-be-issued tokens
        DecreaseToBeIssuedTokens(T::AccountId, Issuing<T>),
        /// vault_id, additional number of issued tokens
        IssueTokens(T::AccountId, Issuing<T>),
        /// vault_id, additional to-be-redeemed tokens
        IncreaseToBeRedeemedTokens(T::AccountId, Issuing<T>),
        /// vault_id, decrease in to-be-redeemed tokens
        DecreaseToBeRedeemedTokens(T::AccountId, Issuing<T>),
        /// vault_id, additional to-be-replaced tokens
        IncreaseToBeReplacedTokens(T::AccountId, Issuing<T>),
        /// vault_id, user_id, amount of tokens reduced in issued & to-be-redeemed
        DecreaseTokens(T::AccountId, T::AccountId, Issuing<T>),
        /// vault_id, amount of newly redeemed tokens
        RedeemTokens(T::AccountId, Issuing<T>),
        /// vault_id, amount of newly redeemed tokens, amount of backing transferred, user_id
        RedeemTokensPremium(T::AccountId, Issuing<T>, Backing<T>, T::AccountId),
        /// vault_id, amount of newly redeemed tokens, slashed collateral
        RedeemTokensLiquidatedVault(T::AccountId, Issuing<T>, Backing<T>),
        /// vault_id, amount of burned tokens, transferred collateral
        RedeemTokensLiquidation(T::AccountId, Issuing<T>, Backing<T>),
        /// old_vault_id, new_vault_id, transferred tokens, additional collateral locked by new_vault
        ReplaceTokens(T::AccountId, T::AccountId, Issuing<T>, Backing<T>),
        /// vault_id, issued_tokens, to_be_issued_tokens, to_be_redeemed_tokens,
        /// to_be_replaced_tokens, backing_collateral, status, replace_collateral
        LiquidateVault(
            T::AccountId,
            Issuing<T>,
            Issuing<T>,
            Issuing<T>,
            Issuing<T>,
            Backing<T>,
            VaultStatus,
            Backing<T>,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientCollateral,
        /// The amount of tokens to be issued is higher than the issuable amount by the vault
        ExceedingVaultLimit,
        InsufficientTokensCommitted,
        VaultBanned,
        /// Returned if the collateral amount to register a vault was too low
        InsufficientVaultCollateralAmount,
        // FIXME: ERR_MIN_AMOUNT in spec
        /// Returned if a vault tries to register while already being registered
        VaultAlreadyRegistered,
        VaultNotFound,
        /// The Bitcoin Address has already been registered
        ReservedDepositAddress,
        /// Collateralization is infinite if no tokens are issued
        NoTokensIssued,
        NoVaultWithSufficientCollateral,
        NoVaultWithSufficientTokens,
        NoVaultUnderThePremiumRedeemThreshold,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        /// Unable to convert value
        TryIntoIntError,
        InvalidSecretKey,
        InvalidPublicKey,
        NominationOperatorCannotWithdrawDirectly,
        VaultIsNotANominationOperator,
        VaultNotBelowLiquidationThreshold,
    }

    /// The minimum collateral (e.g. DOT/KSM) a Vault needs to provide
    /// to participate in the issue process.
    #[pallet::storage]
    #[pallet::getter(fn minimum_collateral_vault)]
    pub(super) type MinimumCollateralVault<T: Config> = StorageValue<_, Backing<T>, ValueQuery>;

    /// If a Vault fails to execute a correct redeem or replace,
    /// it is temporarily banned from further issue, redeem or replace requests.
    #[pallet::storage]
    #[pallet::getter(fn punishment_delay)]
    pub(super) type PunishmentDelay<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Determines the over-collateralization rate for collateral locked
    /// by Vaults, necessary for issuing tokens. Must to be strictly
    /// greater than 100000 and LiquidationCollateralThreshold.
    #[pallet::storage]
    #[pallet::getter(fn secure_collateral_threshold)]
    pub(super) type SecureCollateralThreshold<T: Config> = StorageValue<_, UnsignedFixedPoint<T>, ValueQuery>;

    /// Determines the rate for the collateral rate of Vaults,
    /// at which users receive a premium, allocated from the
    /// Vault's collateral, when performing a redeem with this Vault.
    /// Must to be strictly greater than 100000 and LiquidationCollateralThreshold.
    #[pallet::storage]
    #[pallet::getter(fn premium_redeem_threshold)]
    pub(super) type PremiumRedeemThreshold<T: Config> = StorageValue<_, UnsignedFixedPoint<T>, ValueQuery>;

    /// Determines the lower bound for the collateral rate in issued tokens.
    /// Must be strictly greater than 100000. If a Vault’s collateral rate
    /// drops below this, automatic liquidation (forced Redeem) is triggered.
    #[pallet::storage]
    #[pallet::getter(fn liquidation_collateral_threshold)]
    pub(super) type LiquidationCollateralThreshold<T: Config> = StorageValue<_, UnsignedFixedPoint<T>, ValueQuery>;

    /// Account identifier of an artificial Vault maintained by the VaultRegistry
    /// to handle issued balances and collateral of liquidated Vaults.
    /// That is, when a Vault is liquidated, its balances are transferred to
    /// LiquidationVault and claims are later handled via the LiquidationVault.
    #[pallet::storage]
    pub(super) type LiquidationVaultAccountId<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn liquidation_vault)]
    pub(super) type LiquidationVault<T: Config> = StorageValue<_, SystemVault<Issuing<T>>, ValueQuery>;

    /// Mapping of Vaults, using the respective Vault account identifier as key.
    #[pallet::storage]
    pub(super) type Vaults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>,
        ValueQuery,
    >;

    /// Mapping of reserved BTC addresses to the registered account
    #[pallet::storage]
    pub(super) type ReservedAddresses<T: Config> =
        StorageMap<_, Blake2_128Concat, BtcAddress, T::AccountId, ValueQuery>;

    /// Total collateral used for backing tokens issued by active vaults, excluding the liquidation vault
    #[pallet::storage]
    pub(super) type TotalUserVaultBackingCollateral<T: Config> = StorageValue<_, Backing<T>, ValueQuery>;

    #[pallet::type_value]
    pub(super) fn DefaultForStorageVersion() -> Version {
        Version::V0
    }

    /// Build storage at V1 (requires default 0).
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(super) type StorageVersion<T: Config> = StorageValue<_, Version, ValueQuery, DefaultForStorageVersion>;

    /// Map of Vault Operators
    #[pallet::storage]
    pub(super) type IsNominationOperator<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// The minimum collateral (e.g. DOT/KSM) a Vault needs to provide
        /// to participate in the issue process.
        pub minimum_collateral_vault: Backing<T>,
        /// If a Vault fails to execute a correct redeem or replace,
        /// it is temporarily banned from further issue, redeem or replace requests.
        pub punishment_delay: T::BlockNumber,
        /// Determines the over-collateralization rate for collateral locked
        /// by Vaults, necessary for issuing tokens. Must to be strictly
        /// greater than 100000 and LiquidationCollateralThreshold.
        pub secure_collateral_threshold: UnsignedFixedPoint<T>,
        /// Determines the rate for the collateral rate of Vaults,
        /// at which users receive a premium, allocated from the
        /// Vault\'s collateral, when performing a redeem with this Vault.
        /// Must to be strictly greater than 100000 and LiquidationCollateralThreshold.
        pub premium_redeem_threshold: UnsignedFixedPoint<T>,
        /// Determines the lower bound for the collateral rate in issued tokens.
        /// Must be strictly greater than 100000. If a Vault’s collateral rate
        /// drops below this, automatic liquidation (forced Redeem) is triggered.
        pub liquidation_collateral_threshold: UnsignedFixedPoint<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                minimum_collateral_vault: Default::default(),
                punishment_delay: Default::default(),
                secure_collateral_threshold: Default::default(),
                premium_redeem_threshold: Default::default(),
                liquidation_collateral_threshold: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            MinimumCollateralVault::<T>::put(self.minimum_collateral_vault);
            PunishmentDelay::<T>::put(self.punishment_delay);
            SecureCollateralThreshold::<T>::put(self.secure_collateral_threshold);
            PremiumRedeemThreshold::<T>::put(self.premium_redeem_threshold);
            LiquidationCollateralThreshold::<T>::put(self.liquidation_collateral_threshold);
            StorageVersion::<T>::put(Version::V1);
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn _on_runtime_upgrade() {
        // initialize TotalUserVaultBackingCollateral
        let total = Vaults::<T>::iter()
            .map(|(_, vault)| vault.backing_collateral)
            .fold(Some(0u32.into()), |total: Option<Backing<T>>, x| total?.checked_add(&x))
            .unwrap_or(0u32.into());

        TotalUserVaultBackingCollateral::<T>::set(total);
    }

    fn _offchain_worker() {
        for vault in Self::undercollateralized_vaults() {
            log::info!("Reporting vault {:?}", vault);
            let call = Call::report_undercollateralized_vault(vault);
            let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
        }
    }

    /// The account ID of the liquidation vault.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn liquidation_vault_account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// Public functions

    pub fn _register_vault(
        vault_id: &T::AccountId,
        collateral: Backing<T>,
        public_key: BtcPublicKey,
    ) -> DispatchResult {
        ensure!(
            collateral >= Self::get_minimum_collateral_vault(),
            Error::<T>::InsufficientVaultCollateralAmount
        );
        ensure!(!Self::vault_exists(vault_id), Error::<T>::VaultAlreadyRegistered);

        let vault = Vault::new(vault_id.clone(), public_key);
        Self::insert_vault(vault_id, vault);

        Self::_lock_additional_collateral(vault_id, collateral)?;

        Self::deposit_event(Event::<T>::RegisterVault(vault_id.clone(), collateral));

        Ok(())
    }

    pub fn get_vault_from_id(vault_id: &T::AccountId) -> Result<DefaultVault<T>, DispatchError> {
        ensure!(Self::vault_exists(&vault_id), Error::<T>::VaultNotFound);
        let vault = Vaults::<T>::get(vault_id);
        Ok(vault)
    }

    pub fn get_backing_collateral(vault_id: &T::AccountId) -> Result<Backing<T>, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;
        Ok(vault.backing_collateral)
    }

    /// Like get_vault_from_id, but additionally checks that the vault is active
    pub fn get_active_vault_from_id(vault_id: &T::AccountId) -> Result<DefaultVault<T>, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;
        ensure!(
            matches!(vault.status, VaultStatus::Active(_)),
            Error::<T>::VaultNotFound
        );
        Ok(vault)
    }

    pub fn get_liquidation_vault() -> DefaultSystemVault<T> {
        LiquidationVault::<T>::get()
    }

    pub fn try_increase_backing_collateral(vault_id: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        ensure!(
            Self::is_nomination_operator(vault_id),
            Error::<T>::VaultIsNotANominationOperator
        );
        ensure!(
            Self::is_allowed_to_withdraw_collateral(vault_id, amount)?,
            Error::<T>::InsufficientCollateral
        );
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.increase_backing_collateral(amount)
    }

    pub fn try_decrease_backing_collateral(vault_id: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        ensure!(
            Self::is_nomination_operator(vault_id),
            Error::<T>::VaultIsNotANominationOperator
        );
        ensure!(
            Self::is_allowed_to_withdraw_collateral(vault_id, amount)?,
            Error::<T>::InsufficientCollateral
        );
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.decrease_backing_collateral(amount)
    }

    /// Locks an `amount` of collateral to be used for backing tokens
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-issued tokens
    /// * `amount` - the amount of collateral to be locked
    pub fn _lock_additional_collateral(vault_id: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        Self::_lock_additional_collateral_from_address(vault_id, amount, vault_id)
    }

    pub fn _lock_additional_collateral_from_address(
        vault_id: &T::AccountId,
        amount: Backing<T>,
        depositor_id: &T::AccountId,
    ) -> DispatchResult {
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        if !vault_id.eq(depositor_id) {
            Self::slash_collateral(
                CurrencySource::FreeBalance(depositor_id.clone()),
                CurrencySource::Backing(vault_id.clone()),
                amount,
            )?;
        } else {
            ext::collateral::lock::<T>(depositor_id, amount)?;
            vault.increase_backing_collateral(amount)?;
        }
        Pallet::<T>::increase_total_backing_collateral(amount)?;

        Ok(())
    }

    pub fn force_withdraw_collateral(vault_id: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        Self::force_withdraw_collateral_to_address(vault_id, amount, vault_id)
    }

    pub fn force_withdraw_collateral_to_address(
        vault_id: &T::AccountId,
        amount: Backing<T>,
        payee_id: &T::AccountId,
    ) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(vault_id)?;
        if !vault_id.eq(payee_id) {
            Self::slash_collateral(
                CurrencySource::Backing(vault_id.clone()),
                CurrencySource::FreeBalance(payee_id.clone()),
                amount,
            )?;
        } else {
            ext::collateral::release_collateral::<T>(vault_id, amount)?;
            vault.decrease_backing_collateral(amount)?;
        }
        Pallet::<T>::decrease_total_backing_collateral(amount)?;

        Ok(())
    }

    pub fn try_withdraw_collateral(vault_id: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        Self::try_withdraw_collateral_to_address(vault_id, amount, vault_id)
    }

    pub fn try_withdraw_collateral_to_address(
        vault_id: &T::AccountId,
        amount: Backing<T>,
        payee_id: &T::AccountId,
    ) -> DispatchResult {
        ensure!(
            Self::is_allowed_to_withdraw_collateral(vault_id, amount)?,
            Error::<T>::InsufficientCollateral
        );

        Self::force_withdraw_collateral_to_address(vault_id, amount, payee_id)?;

        Ok(())
    }

    /// checks if the vault would be above the secure threshold after withdrawing collateral
    pub fn is_allowed_to_withdraw_collateral(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<bool, DispatchError> {
        let vault = Self::get_vault_from_id(vault_id)?;

        let new_collateral = match vault.backing_collateral.checked_sub(&amount) {
            Some(x) => x,
            None => return Ok(false),
        };

        let tokens = vault
            .issued_tokens
            .checked_add(&vault.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let is_below_threshold = Pallet::<T>::is_collateral_below_secure_threshold(new_collateral, tokens)?;

        Ok(!is_below_threshold)
    }

    pub fn slash_collateral_saturated(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: Backing<T>,
    ) -> Result<Backing<T>, DispatchError> {
        let available_amount = from.current_balance()?;
        let amount = if available_amount < amount {
            available_amount
        } else {
            amount
        };
        Self::slash_collateral(from, to, amount)?;
        Ok(amount)
    }

    pub fn slash_collateral(from: CurrencySource<T>, to: CurrencySource<T>, amount: Backing<T>) -> DispatchResult {
        match from {
            CurrencySource::Backing(ref account) => {
                Self::force_withdraw_collateral(account, amount)?;
            }
            CurrencySource::Griefing(_) | CurrencySource::LiquidationVault => {
                ext::collateral::release_collateral::<T>(&from.account_id(), amount)?;
            }
            CurrencySource::FreeBalance(_) => {
                // do nothing
            }
        };

        // move from sender's free balance to receiver's free balance
        ext::collateral::transfer::<T>(&from.account_id(), &to.account_id(), amount)?;

        // move receiver funds from free balance to specified currency source
        match to {
            CurrencySource::Backing(ref account) => {
                Self::_lock_additional_collateral(account, amount)?;
            }
            CurrencySource::Griefing(_) | CurrencySource::LiquidationVault => {
                ext::collateral::lock::<T>(&to.account_id(), amount)?;
            }
            CurrencySource::FreeBalance(_) => {
                // do nothing
            }
        };

        Ok(())
    }
    /// Checks if the vault has sufficient collateral to increase the to-be-issued tokens, and
    /// if so, increases it
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-issued tokens
    /// * `tokens` - the amount of tokens to be reserved
    pub fn try_increase_to_be_issued_tokens(vault_id: &T::AccountId, tokens: Issuing<T>) -> Result<(), DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;

        let issuable_tokens = vault.issuable_tokens()?;
        ensure!(issuable_tokens >= tokens, Error::<T>::ExceedingVaultLimit);
        vault.increase_to_be_issued(tokens)?;

        Self::deposit_event(Event::<T>::IncreaseToBeIssuedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Registers a btc address
    ///
    /// # Arguments
    /// * `issue_id` - secure id for generating deposit address
    pub fn register_deposit_address(vault_id: &T::AccountId, issue_id: H256) -> Result<BtcAddress, DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let btc_address = vault.new_deposit_address(issue_id)?;
        Self::deposit_event(Event::<T>::RegisterAddress(vault.id(), btc_address));
        Ok(btc_address)
    }

    /// returns the amount of tokens that a vault can request to be replaced on top of the
    /// current to-be-replaced tokens
    pub fn requestable_to_be_replaced_tokens(vault_id: &T::AccountId) -> Result<Issuing<T>, DispatchError> {
        let vault = Self::get_active_vault_from_id(&vault_id)?;

        let requestable_increase = vault
            .issued_tokens
            .checked_sub(&vault.to_be_replaced_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_sub(&vault.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        Ok(requestable_increase)
    }

    /// returns the new total to-be-replaced and replace-collateral
    pub fn try_increase_to_be_replaced_tokens(
        vault_id: &T::AccountId,
        tokens: Issuing<T>,
        griefing_collateral: Backing<T>,
    ) -> Result<(Issuing<T>, Backing<T>), DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;

        let new_to_be_replaced = vault
            .data
            .to_be_replaced_tokens
            .checked_add(&tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let new_collateral = vault
            .data
            .replace_collateral
            .checked_add(&griefing_collateral)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let total_decreasing_tokens = new_to_be_replaced
            .checked_add(&vault.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        ensure!(
            total_decreasing_tokens <= vault.data.issued_tokens,
            Error::<T>::InsufficientTokensCommitted
        );

        vault.set_to_be_replaced_amount(new_to_be_replaced, new_collateral)?;

        Self::deposit_event(Event::<T>::IncreaseToBeReplacedTokens(vault.id(), tokens));

        Ok((new_to_be_replaced, new_collateral))
    }

    pub fn decrease_to_be_replaced_tokens(
        vault_id: &T::AccountId,
        tokens: Issuing<T>,
    ) -> Result<(Issuing<T>, Backing<T>), DispatchError> {
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;

        let initial_to_be_replaced = vault.data.to_be_replaced_tokens;
        let initial_griefing_collateral = vault.data.replace_collateral;

        let used_tokens = tokens.min(vault.data.to_be_replaced_tokens);

        let used_collateral =
            Self::calculate_collateral(initial_griefing_collateral, used_tokens, initial_to_be_replaced)?;

        // make sure we don't use too much if a rounding error occurs
        let used_collateral = used_collateral.min(initial_griefing_collateral);

        let new_to_be_replaced = initial_to_be_replaced
            .checked_sub(&used_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        let new_collateral = initial_griefing_collateral
            .checked_sub(&used_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        vault.set_to_be_replaced_amount(new_to_be_replaced, new_collateral)?;

        Ok((used_tokens, used_collateral))
    }

    /// Decreases the amount of tokens to be issued in the next issue request from the
    /// vault, or from the liquidation vault if the vault is liquidated
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease to-be-issued tokens
    /// * `tokens` - the amount of tokens to be unreserved
    pub fn decrease_to_be_issued_tokens(vault_id: &T::AccountId, tokens: Issuing<T>) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(vault_id)?;
        vault.decrease_to_be_issued(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseToBeIssuedTokens(vault_id.clone(), tokens));
        Ok(())
    }

    /// Issues an amount of `tokens` tokens for the given `vault_id`
    /// At this point, the to-be-issued tokens assigned to a vault are decreased
    /// and the issued tokens balance is increased by the amount of issued tokens.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to issue tokens
    /// * `tokens` - the amount of tokens to issue
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of tokens reserved is too low
    pub fn issue_tokens(vault_id: &T::AccountId, tokens: Issuing<T>) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.issue_tokens(tokens)?;
        Self::deposit_event(Event::<T>::IssueTokens(vault.id(), tokens));
        Ok(())
    }

    /// Adds an amount tokens to the to-be-redeemed tokens balance of a vault.
    /// This function serves as a prevention against race conditions in the
    /// redeem and replace procedures. If, for example, a vault would receive
    /// two redeem requests at the same time that have a higher amount of tokens
    ///  to be issued than his issuedTokens balance, one of the two redeem
    /// requests should be rejected.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to increase to-be-redeemed tokens
    /// * `tokens` - the amount of tokens to be redeemed
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of redeemable tokens is too low
    pub fn try_increase_to_be_redeemed_tokens(vault_id: &T::AccountId, tokens: Issuing<T>) -> DispatchResult {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let redeemable = vault
            .data
            .issued_tokens
            .checked_sub(&vault.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        ensure!(redeemable >= tokens, Error::<T>::InsufficientTokensCommitted);

        vault.increase_to_be_redeemed(tokens)?;

        Self::deposit_event(Event::<T>::IncreaseToBeRedeemedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Subtracts an amount tokens from the to-be-redeemed tokens balance of a vault.
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease to-be-redeemed tokens
    /// * `tokens` - the amount of tokens to be redeemed
    ///
    /// # Errors
    /// * `VaultNotFound` - if no vault exists for the given `vault_id`
    /// * `InsufficientTokensCommitted` - if the amount of to-be-redeemed tokens is too low
    pub fn decrease_to_be_redeemed_tokens(vault_id: &T::AccountId, tokens: Issuing<T>) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.decrease_to_be_redeemed(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseToBeRedeemedTokens(vault.id(), tokens));
        Ok(())
    }

    /// Decreases the amount of tokens f a redeem request is not fulfilled
    /// Removes the amount of tokens assigned to the to-be-redeemed tokens.
    /// At this point, we consider the tokens lost and the issued tokens are
    /// removed from the vault
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to decrease tokens
    /// * `tokens` - the amount of tokens to be decreased
    /// * `user_id` - the id of the user making the redeem request
    pub fn decrease_tokens(vault_id: &T::AccountId, user_id: &T::AccountId, tokens: Issuing<T>) -> DispatchResult {
        // decrease to-be-redeemed and issued
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;
        vault.decrease_tokens(tokens)?;

        Self::deposit_event(Event::<T>::DecreaseTokens(vault.id(), user_id.clone(), tokens));
        Ok(())
    }

    /// Reduces the to-be-redeemed tokens when a redeem request completes
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault from which to redeem tokens
    /// * `tokens` - the amount of tokens to be decreased
    /// * `premium` - amount of collateral to be rewarded to the redeemer if the vault is not liquidated yet
    /// * `redeemer_id` - the id of the redeemer
    pub fn redeem_tokens(
        vault_id: &T::AccountId,
        tokens: Issuing<T>,
        premium: Backing<T>,
        redeemer_id: &T::AccountId,
    ) -> DispatchResult {
        let mut vault = Self::get_rich_vault_from_id(&vault_id)?;

        // need to read before we decrease it
        let to_be_redeemed_tokens = vault.data.to_be_redeemed_tokens;

        vault.decrease_to_be_redeemed(tokens)?;
        vault.decrease_issued(tokens)?;

        if !vault.data.is_liquidated() {
            if premium.is_zero() {
                Self::deposit_event(Event::<T>::RedeemTokens(vault.id(), tokens));
            } else {
                Self::slash_collateral(
                    CurrencySource::Backing(vault_id.clone()),
                    CurrencySource::FreeBalance(redeemer_id.clone()),
                    premium,
                )?;

                Self::deposit_event(Event::<T>::RedeemTokensPremium(
                    vault_id.clone(),
                    tokens,
                    premium,
                    redeemer_id.clone(),
                ));
            }
        } else {
            // withdraw vault collateral
            let amount = Self::calculate_collateral(
                CurrencySource::Backing::<T>(vault_id.clone()).current_balance()?,
                tokens,
                to_be_redeemed_tokens,
            )?;
            Self::force_withdraw_collateral(vault_id, amount)?;

            Self::deposit_event(Event::<T>::RedeemTokensLiquidatedVault(
                vault_id.clone(),
                tokens,
                amount,
            ));
        }

        Ok(())
    }

    /// Handles redeem requests which are executed against the LiquidationVault.
    /// Reduces the issued token of the LiquidationVault and slashes the
    /// corresponding amount of collateral.
    ///
    /// # Arguments
    /// * `redeemer_id` - the account of the user redeeming issued tokens
    /// * `tokens` - the amount of tokens to be redeemed in collateral with the LiquidationVault, denominated in BTC
    ///
    /// # Errors
    /// * `InsufficientTokensCommitted` - if the amount of tokens issued by the liquidation vault is too low
    /// * `InsufficientFunds` - if the liquidation vault does not have enough collateral to transfer
    pub fn redeem_tokens_liquidation(redeemer_id: &T::AccountId, amount_btc: Issuing<T>) -> DispatchResult {
        let mut liquidation_vault = Self::get_rich_liquidation_vault();

        ensure!(
            liquidation_vault.redeemable_tokens()? >= amount_btc,
            Error::<T>::InsufficientTokensCommitted
        );

        // transfer liquidated collateral to redeemer
        let to_transfer = Self::calculate_collateral(
            CurrencySource::<T>::LiquidationVault.current_balance()?,
            amount_btc,
            liquidation_vault.backed_tokens()?,
        )?;

        Self::slash_collateral(
            CurrencySource::LiquidationVault,
            CurrencySource::FreeBalance(redeemer_id.clone()),
            to_transfer,
        )?;

        liquidation_vault.decrease_issued(amount_btc)?;

        Self::deposit_event(Event::<T>::RedeemTokensLiquidation(
            redeemer_id.clone(),
            amount_btc,
            to_transfer,
        ));

        Ok(())
    }

    /// Replaces the old vault by the new vault by transferring tokens
    /// from the old vault to the new one
    ///
    /// # Arguments
    /// * `old_vault_id` - the id of the old vault
    /// * `new_vault_id` - the id of the new vault
    /// * `tokens` - the amount of tokens to be transferred from the old to the new vault
    /// * `collateral` - the collateral to be locked by the new vault
    ///
    /// # Errors
    /// * `VaultNotFound` - if either the old or new vault does not exist
    /// * `InsufficientTokensCommitted` - if the amount of tokens of the old vault is too low
    /// * `InsufficientFunds` - if the new vault does not have enough collateral to lock
    pub fn replace_tokens(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: Issuing<T>,
        collateral: Backing<T>,
    ) -> DispatchResult {
        let mut old_vault = Self::get_rich_vault_from_id(&old_vault_id)?;
        let mut new_vault = Self::get_rich_vault_from_id(&new_vault_id)?;

        if old_vault.data.is_liquidated() {
            // release old-vault's collateral
            let current_backing = CurrencySource::<T>::Backing(old_vault_id.clone()).current_balance()?;
            let to_be_released =
                Self::calculate_collateral(current_backing, tokens, old_vault.data.to_be_redeemed_tokens)?;
            Self::force_withdraw_collateral(&old_vault_id, to_be_released)?;
        }

        old_vault.decrease_tokens(tokens)?;
        new_vault.issue_tokens(tokens)?;

        Self::deposit_event(Event::<T>::ReplaceTokens(
            old_vault_id.clone(),
            new_vault_id.clone(),
            tokens,
            collateral,
        ));
        Ok(())
    }

    /// Cancels a replace - which in the normal case decreases the old-vault's
    /// to-be-redeemed tokens, and the new-vault's to-be-issued tokens.
    /// When one or both of the vaults have been liquidated, this function also
    /// updates the liquidation vault.
    ///
    /// # Arguments
    /// * `old_vault_id` - the id of the old vault
    /// * `new_vault_id` - the id of the new vault
    /// * `tokens` - the amount of tokens to be transferred from the old to the new vault
    pub fn cancel_replace_tokens(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: Issuing<T>,
    ) -> DispatchResult {
        let mut old_vault = Self::get_rich_vault_from_id(&old_vault_id)?;
        let mut new_vault = Self::get_rich_vault_from_id(&new_vault_id)?;

        if old_vault.data.is_liquidated() {
            let old_vault_backing = CurrencySource::<T>::Backing(old_vault_id.clone());

            // transfer old-vault's collateral to liquidation_vault
            let to_be_transfered_collateral = Self::calculate_collateral(
                old_vault_backing.current_balance()?,
                tokens,
                old_vault.data.to_be_redeemed_tokens,
            )?;
            Self::slash_collateral(
                old_vault_backing,
                CurrencySource::LiquidationVault,
                to_be_transfered_collateral,
            )?;
        }

        old_vault.decrease_to_be_redeemed(tokens)?;
        new_vault.decrease_to_be_issued(tokens)?;

        Ok(())
    }

    fn undercollateralized_vaults() -> impl Iterator<Item = T::AccountId> {
        <Vaults<T>>::iter().filter_map(|(vault_id, vault)| {
            let liquidation_threshold = <LiquidationCollateralThreshold<T>>::get();
            if Self::is_vault_below_liquidation_threshold(&vault, liquidation_threshold).unwrap_or(false) {
                Some(vault_id)
            } else {
                None
            }
        })
    }

    /// Liquidates a vault, transferring all of its token balances to the `LiquidationVault`.
    /// Delegates to `liquidate_vault_with_status`, using `Liquidated` status
    pub fn liquidate_vault(vault_id: &T::AccountId) -> Result<Backing<T>, DispatchError> {
        Self::liquidate_vault_with_status(vault_id, VaultStatus::Liquidated)
    }

    /// Liquidates a vault, transferring all of its token balances to the
    /// `LiquidationVault`, as well as the collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to liquidate
    /// * `status` - status with which to liquidate the vault
    ///
    /// # Errors
    /// * `VaultNotFound` - if the vault to liquidate does not exist
    pub fn liquidate_vault_with_status(
        vault_id: &T::AccountId,
        status: VaultStatus,
    ) -> Result<Backing<T>, DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let vault_orig = vault.data.clone();
        let mut liquidation_vault = Self::get_rich_liquidation_vault();

        let to_slash = vault.liquidate(&mut liquidation_vault, status)?;

        Self::deposit_event(Event::<T>::LiquidateVault(
            vault_id.clone(),
            vault_orig.issued_tokens,
            vault_orig.to_be_issued_tokens,
            vault_orig.to_be_redeemed_tokens,
            vault_orig.to_be_replaced_tokens,
            vault_orig.backing_collateral,
            status,
            vault_orig.replace_collateral,
        ));
        Ok(to_slash)
    }

    pub(crate) fn increase_total_backing_collateral(amount: Backing<T>) -> DispatchResult {
        let new = TotalUserVaultBackingCollateral::<T>::get()
            .checked_add(&amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        TotalUserVaultBackingCollateral::<T>::set(new);

        Ok(())
    }

    pub(crate) fn decrease_total_backing_collateral(amount: Backing<T>) -> DispatchResult {
        let new = TotalUserVaultBackingCollateral::<T>::get()
            .checked_sub(&amount)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        TotalUserVaultBackingCollateral::<T>::set(new);

        Ok(())
    }

    /// returns the total number of issued tokens
    pub fn get_total_issued_tokens(include_liquidation_vault: bool) -> Result<Issuing<T>, DispatchError> {
        if include_liquidation_vault {
            Ok(ext::treasury::total_issued::<T>())
        } else {
            ext::treasury::total_issued::<T>()
                .checked_sub(&Self::get_liquidation_vault().issued_tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow.into())
        }
    }

    /// returns the total locked collateral, _
    pub fn get_total_backing_collateral(include_liquidation_vault: bool) -> Result<Backing<T>, DispatchError> {
        let liquidated_collateral = CurrencySource::<T>::LiquidationVault.current_balance()?;
        let total = if include_liquidation_vault {
            TotalUserVaultBackingCollateral::<T>::get()
                .checked_add(&liquidated_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?
        } else {
            TotalUserVaultBackingCollateral::<T>::get()
        };

        Ok(total)
    }

    pub fn insert_vault(id: &T::AccountId, vault: Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>) {
        Vaults::<T>::insert(id, vault)
    }

    pub fn ban_vault(vault_id: T::AccountId) -> DispatchResult {
        let height = ext::security::active_block_number::<T>();
        let mut vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.ban_until(height + Self::punishment_delay());
        Ok(())
    }

    pub fn _ensure_not_banned(vault_id: &T::AccountId) -> DispatchResult {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        vault.ensure_not_banned()
    }

    /// Threshold checks
    pub fn is_vault_below_secure_threshold(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, SecureCollateralThreshold::<T>::get())
    }

    pub fn is_vault_liquidated(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(Self::get_vault_from_id(&vault_id)?.is_liquidated())
    }

    pub fn is_vault_below_premium_threshold(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Self::is_vault_below_threshold(&vault_id, PremiumRedeemThreshold::<T>::get())
    }

    /// check if the vault is below the liquidation threshold. In contrast to other thresholds,
    /// this is checked as ratio of `collateral / (issued - to_be_redeemed)`.
    pub fn is_vault_below_liquidation_threshold(
        vault: &Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>,
        liquidation_threshold: UnsignedFixedPoint<T>,
    ) -> Result<bool, DispatchError> {
        // the currently issued tokens
        let tokens = vault
            .issued_tokens
            .checked_sub(&vault.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        Self::is_collateral_below_threshold(vault.backing_collateral, tokens, liquidation_threshold)
    }

    pub fn is_collateral_below_secure_threshold(
        collateral: Backing<T>,
        btc_amount: Issuing<T>,
    ) -> Result<bool, DispatchError> {
        let threshold = SecureCollateralThreshold::<T>::get();
        Self::is_collateral_below_threshold(collateral, btc_amount, threshold)
    }

    pub fn set_is_nomination_operator(vault_id: &T::AccountId, is_operator: bool) {
        IsNominationOperator::<T>::insert(vault_id, is_operator);
    }

    pub fn set_secure_collateral_threshold(threshold: UnsignedFixedPoint<T>) {
        SecureCollateralThreshold::<T>::set(threshold);
    }

    pub fn set_premium_redeem_threshold(threshold: UnsignedFixedPoint<T>) {
        PremiumRedeemThreshold::<T>::set(threshold);
    }

    pub fn set_liquidation_collateral_threshold(threshold: UnsignedFixedPoint<T>) {
        LiquidationCollateralThreshold::<T>::set(threshold);
    }

    pub fn get_premium_redeem_threshold() -> UnsignedFixedPoint<T> {
        PremiumRedeemThreshold::<T>::get()
    }

    pub fn get_liquidation_collateral_threshold() -> UnsignedFixedPoint<T> {
        LiquidationCollateralThreshold::<T>::get()
    }

    pub fn is_over_minimum_collateral(amount: Backing<T>) -> bool {
        amount > Self::get_minimum_collateral_vault()
    }

    pub fn is_nomination_operator(vault_id: &T::AccountId) -> bool {
        IsNominationOperator::<T>::get(vault_id)
    }

    /// return (collateral * Numerator) / denominator, used when dealing with liquidated vaults
    pub fn calculate_collateral(
        collateral: Backing<T>,
        numerator: Issuing<T>,
        denominator: Issuing<T>,
    ) -> Result<Backing<T>, DispatchError> {
        if numerator.is_zero() && denominator.is_zero() {
            return Ok(collateral);
        }

        let collateral: U256 = Self::backing_to_u128(collateral)?.into();
        let numerator: U256 = Self::issuing_to_u128(numerator)?.into();
        let denominator: U256 = Self::issuing_to_u128(denominator)?.into();

        let amount = collateral
            .checked_mul(numerator)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_div(denominator)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        Self::u128_to_backing(amount.try_into().map_err(|_| Error::<T>::TryIntoIntError)?)
    }

    /// RPC

    /// Get the total collateralization of the system.
    pub fn get_total_collateralization() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let issued_tokens = Self::get_total_issued_tokens(true)?;
        let total_collateral = Self::get_total_backing_collateral(true)?;

        // convert the issued_tokens to the raw amount
        let raw_issued_tokens = Self::issuing_to_u128(issued_tokens)?;
        ensure!(raw_issued_tokens != 0, Error::<T>::NoTokensIssued);

        // convert the collateral to issuing
        let collateral_in_issuing = ext::oracle::backing_to_issuing::<T>(total_collateral)?;
        let raw_collateral_in_issuing = Self::issuing_to_u128(collateral_in_issuing)?;

        Self::get_collateralization(raw_collateral_in_issuing, raw_issued_tokens)
    }

    /// Get the first available vault with sufficient collateral to fulfil an issue request
    /// with the specified amount of issued tokens.
    pub fn get_first_vault_with_sufficient_collateral(amount: Issuing<T>) -> Result<T::AccountId, DispatchError> {
        // find all vault accounts with sufficient collateral
        let suitable_vaults = Vaults::<T>::iter()
            .filter_map(|v| {
                // iterator returns tuple of (AccountId, Vault<T>), we check the vault and return the accountid
                let vault = Into::<RichVault<T>>::into(v.1);
                // make sure the vault accepts new issues
                if vault.data.status != VaultStatus::Active(true) {
                    return None;
                }
                let issuable_tokens = vault.issuable_tokens().ok()?;
                if issuable_tokens >= amount {
                    Some(v.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultWithSufficientCollateral.into())
        } else {
            let idx = Self::pseudo_rand_index(amount, suitable_vaults.len());
            Ok(suitable_vaults[idx].clone())
        }
    }

    /// Get the first available vault with sufficient locked tokens to fulfil a redeem request.
    pub fn get_first_vault_with_sufficient_tokens(amount: Issuing<T>) -> Result<T::AccountId, DispatchError> {
        // find all vault accounts with sufficient collateral
        let suitable_vaults = Vaults::<T>::iter()
            .filter_map(|v| {
                // iterator returns tuple of (AccountId, Vault<T>), we check the vault and return the accountid
                let vault = Into::<RichVault<T>>::into(v.1);
                let redeemable_tokens = vault.redeemable_tokens().ok()?;
                if redeemable_tokens >= amount {
                    Some(v.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultWithSufficientTokens.into())
        } else {
            let idx = Self::pseudo_rand_index(amount, suitable_vaults.len());
            Ok(suitable_vaults[idx].clone())
        }
    }

    /// Get all vaults that:
    /// - are below the premium redeem threshold, and
    /// - have a non-zero amount of redeemable tokens, and thus
    /// - are not banned
    ///
    /// Maybe returns a tuple of (VaultId, RedeemableTokens)
    /// The redeemable tokens are the currently vault.issued_tokens - the vault.to_be_redeemed_tokens
    pub fn get_premium_redeem_vaults() -> Result<Vec<(T::AccountId, Issuing<T>)>, DispatchError> {
        let mut suitable_vaults = Vaults::<T>::iter()
            .filter_map(|(account_id, vault)| {
                let rich_vault: RichVault<T> = vault.into();

                let redeemable_tokens = rich_vault.redeemable_tokens().ok()?;

                if !redeemable_tokens.is_zero() && Self::is_vault_below_premium_threshold(&account_id).unwrap_or(false)
                {
                    Some((account_id, redeemable_tokens))
                } else {
                    None
                }
            })
            .collect::<Vec<(_, _)>>();

        if suitable_vaults.is_empty() {
            Err(Error::<T>::NoVaultUnderThePremiumRedeemThreshold.into())
        } else {
            suitable_vaults.sort_by(|a, b| b.1.cmp(&a.1));
            Ok(suitable_vaults)
        }
    }

    /// Get all vaults with non-zero issuable tokens, ordered in descending order of this amount
    pub fn get_vaults_with_issuable_tokens() -> Result<Vec<(T::AccountId, Issuing<T>)>, DispatchError> {
        let mut vaults_with_issuable_tokens = Vaults::<T>::iter()
            .filter_map(|(account_id, _vault)| {
                // NOTE: we are not checking if the vault accepts new issues here - if not, then
                // get_issuable_tokens_from_vault will return 0, and we will filter them out below

                // iterator returns tuple of (AccountId, Vault<T>),
                match Self::get_issuable_tokens_from_vault(account_id.clone()).ok() {
                    Some(issuable_tokens) => {
                        if !issuable_tokens.is_zero() {
                            Some((account_id, issuable_tokens))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            })
            .collect::<Vec<(_, _)>>();

        vaults_with_issuable_tokens.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(vaults_with_issuable_tokens)
    }

    /// Get all vaults with non-zero issued (thus redeemable) tokens, ordered in descending order of this amount
    pub fn get_vaults_with_redeemable_tokens() -> Result<Vec<(T::AccountId, Issuing<T>)>, DispatchError> {
        // find all vault accounts with sufficient collateral
        let mut vaults_with_redeemable_tokens = Vaults::<T>::iter()
            .filter_map(|(account_id, vault)| {
                let vault = Into::<RichVault<T>>::into(vault);
                let redeemable_tokens = vault.redeemable_tokens().ok()?;
                if !redeemable_tokens.is_zero() {
                    Some((account_id, redeemable_tokens))
                } else {
                    None
                }
            })
            .collect::<Vec<(_, _)>>();

        vaults_with_redeemable_tokens.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(vaults_with_redeemable_tokens)
    }

    /// Get the amount of tokens a vault can issue
    pub fn get_issuable_tokens_from_vault(vault_id: T::AccountId) -> Result<Issuing<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        // make sure the vault accepts new issue requests.
        // NOTE: get_vaults_with_issuable_tokens depends on this check
        if vault.data.status != VaultStatus::Active(true) {
            return Ok(0u32.into());
        }
        vault.issuable_tokens()
    }

    /// Get the amount of tokens issued by a vault
    pub fn get_to_be_issued_tokens_from_vault(vault_id: T::AccountId) -> Result<Issuing<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        Ok(vault.to_be_issued_tokens())
    }

    /// Get the current collateralization of a vault
    pub fn get_collateralization_from_vault(
        vault_id: T::AccountId,
        only_issued: bool,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let collateral = vault.get_collateral();
        Self::get_collateralization_from_vault_and_collateral(vault_id, collateral, only_issued)
    }

    pub fn get_collateralization_from_vault_and_collateral(
        vault_id: T::AccountId,
        collateral: Backing<T>,
        only_issued: bool,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let issued_tokens = if only_issued {
            vault.data.issued_tokens
        } else {
            vault.data.issued_tokens + vault.data.to_be_issued_tokens
        };

        // convert the issued_tokens to the raw amount
        let raw_issued_tokens = Self::issuing_to_u128(issued_tokens)?;
        ensure!(raw_issued_tokens != 0, Error::<T>::NoTokensIssued);

        // convert the collateral to issuing
        let collateral_in_issuing = ext::oracle::backing_to_issuing::<T>(collateral)?;
        let raw_collateral_in_issuing = Self::issuing_to_u128(collateral_in_issuing)?;

        Self::get_collateralization(raw_collateral_in_issuing, raw_issued_tokens)
    }

    /// Gets the minimum amount of collateral required for the given amount of btc
    /// with the current threshold and exchange rate
    ///
    /// # Arguments
    /// * `amount_btc` - the amount of issuing
    pub fn get_required_collateral_for_issuing(amount_btc: Issuing<T>) -> Result<Backing<T>, DispatchError> {
        let threshold = SecureCollateralThreshold::<T>::get();
        let collateral = Self::get_required_collateral_for_issuing_with_threshold(amount_btc, threshold)?;
        Ok(collateral)
    }

    /// Get the amount of collateral required for the given vault to be at the
    /// current SecureCollateralThreshold with the current exchange rate
    pub fn get_required_collateral_for_vault(vault_id: T::AccountId) -> Result<Backing<T>, DispatchError> {
        let vault = Self::get_active_rich_vault_from_id(&vault_id)?;
        let issued_tokens = vault.data.issued_tokens + vault.data.to_be_issued_tokens;

        let required_collateral = Self::get_required_collateral_for_issuing(issued_tokens)?;

        Ok(required_collateral)
    }

    pub fn vault_exists(id: &T::AccountId) -> bool {
        Vaults::<T>::contains_key(id)
    }

    /// Private getters and setters

    fn get_rich_vault_from_id(vault_id: &T::AccountId) -> Result<RichVault<T>, DispatchError> {
        Ok(Self::get_vault_from_id(vault_id)?.into())
    }

    /// Like get_rich_vault_from_id, but only returns active vaults
    fn get_active_rich_vault_from_id(vault_id: &T::AccountId) -> Result<RichVault<T>, DispatchError> {
        Ok(Self::get_active_vault_from_id(vault_id)?.into())
    }

    fn get_rich_liquidation_vault() -> RichSystemVault<T> {
        Into::<RichSystemVault<T>>::into(Self::get_liquidation_vault())
    }

    fn get_minimum_collateral_vault() -> Backing<T> {
        MinimumCollateralVault::<T>::get()
    }

    // Other helpers

    /// get a psuedorandom value between 0 (inclusive) and `limit` (exclusive), based on
    /// the hashes of the last 81 blocks, and the given subject.
    ///
    /// # Arguments
    ///
    /// * `subject` - an extra value to feed into the pseudorandom number generator
    /// * `limit` - the limit of the returned value
    fn pseudo_rand_index(subject: Issuing<T>, limit: usize) -> usize {
        let raw_subject = Self::issuing_to_u128(subject).unwrap_or(0 as u128);

        // convert into a slice. Endianness of the conversion function is arbitrary chosen
        let bytes = &raw_subject.to_be_bytes();

        let (rand_hash, _) = T::RandomnessSource::random(bytes);

        let ret = rand_hash.to_low_u64_le() % (limit as u64);
        ret as usize
    }

    /// calculate the collateralization as a ratio of the issued tokens to the
    /// amount of provided collateral at the current exchange rate.
    fn get_collateralization(
        raw_collateral_in_issuing: u128,
        raw_issued_tokens: u128,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let collateralization =
            UnsignedFixedPoint::<T>::checked_from_rational(raw_collateral_in_issuing, raw_issued_tokens)
                .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(collateralization)
    }

    fn is_vault_below_threshold(
        vault_id: &T::AccountId,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<bool, DispatchError> {
        let vault = Self::get_rich_vault_from_id(&vault_id)?;

        // the currently issued tokens
        let issued_tokens = vault.data.issued_tokens;

        // the current locked backing collateral by the vault
        let collateral = Self::get_backing_collateral(vault_id)?;

        Self::is_collateral_below_threshold(collateral, issued_tokens, threshold)
    }

    fn is_collateral_below_threshold(
        collateral: Backing<T>,
        btc_amount: Issuing<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<bool, DispatchError> {
        let max_tokens = Self::calculate_max_issuing_from_collateral_for_threshold(collateral, threshold)?;
        // check if the max_tokens are below the issued tokens
        Ok(max_tokens < btc_amount)
    }

    /// Gets the minimum amount of collateral required for the given amount of btc
    /// with the current exchange rate and the given threshold. This function is the
    /// inverse of calculate_max_issuing_from_collateral_for_threshold
    ///
    /// # Arguments
    /// * `amount_btc` - the amount of issuing
    /// * `threshold` - the required secure collateral threshold
    fn get_required_collateral_for_issuing_with_threshold(
        issuing: Issuing<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<Backing<T>, DispatchError> {
        // Step 1: inverse of the scaling applied in calculate_max_issuing_from_collateral_for_threshold
        let amount_in_issuing = threshold
            .checked_mul_int_rounded_up(issuing)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        // Step 2: convert the amount to collateral
        let amount_in_backing = ext::oracle::issuing_to_backing::<T>(amount_in_issuing)?;
        Ok(amount_in_backing)
    }

    fn calculate_max_issuing_from_collateral_for_threshold(
        collateral: Backing<T>,
        threshold: UnsignedFixedPoint<T>,
    ) -> Result<Issuing<T>, DispatchError> {
        // convert the collateral to issuing
        let collateral_in_issuing = ext::oracle::backing_to_issuing::<T>(collateral)?;
        let collateral_in_issuing = Self::issuing_to_u128(collateral_in_issuing)?;

        // calculate how many tokens should be maximally issued given the threshold.
        let collateral_as_inner =
            TryInto::<Inner<T>>::try_into(collateral_in_issuing).map_err(|_| Error::<T>::TryIntoIntError)?;
        let max_btc_as_inner = UnsignedFixedPoint::<T>::checked_from_integer(collateral_as_inner)
            .ok_or(Error::<T>::TryIntoIntError)?
            .checked_div(&threshold)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .into_inner()
            .checked_div(&UnsignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        let max_btc_raw = UniqueSaturatedInto::<u128>::unique_saturated_into(max_btc_as_inner);

        Self::u128_to_issuing(max_btc_raw)
    }

    fn issuing_to_u128(x: Issuing<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn backing_to_u128(x: Backing<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_backing(x: u128) -> Result<Backing<T>, DispatchError> {
        TryInto::<Backing<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_issuing(x: u128) -> Result<Issuing<T>, DispatchError> {
        TryInto::<Issuing<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    pub fn insert_vault_deposit_address(vault_id: &T::AccountId, btc_address: BtcAddress) -> DispatchResult {
        ensure!(
            !ReservedAddresses::<T>::contains_key(&btc_address),
            Error::<T>::ReservedDepositAddress
        );
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        vault.insert_deposit_address(btc_address);
        ReservedAddresses::<T>::insert(btc_address, vault_id);
        Ok(())
    }

    pub fn new_vault_deposit_address(vault_id: &T::AccountId, secure_id: H256) -> Result<BtcAddress, DispatchError> {
        let mut vault = Self::get_active_rich_vault_from_id(vault_id)?;
        let btc_address = vault.new_deposit_address(secure_id)?;
        Ok(btc_address)
    }
}

trait CheckedMulIntRoundedUp {
    /// Like checked_mul_int, but this version rounds the result up instead of down.
    fn checked_mul_int_rounded_up<N: TryFrom<u128> + TryInto<u128>>(self, n: N) -> Option<N>;
}

impl<T: FixedPointNumber> CheckedMulIntRoundedUp for T {
    fn checked_mul_int_rounded_up<N: TryFrom<u128> + TryInto<u128>>(self, n: N) -> Option<N> {
        // convert n into fixed_point
        let n_inner = TryInto::<T::Inner>::try_into(n.try_into().ok()?).ok()?;
        let n_fixed_point = T::checked_from_integer(n_inner)?;

        // do the multiplication
        let product = self.checked_mul(&n_fixed_point)?;

        // convert to inner
        let product_inner = UniqueSaturatedInto::<u128>::unique_saturated_into(product.into_inner());

        // convert to u128 by dividing by a rounded up division by accuracy
        let accuracy = UniqueSaturatedInto::<u128>::unique_saturated_into(T::accuracy());
        product_inner
            .checked_add(accuracy)?
            .checked_sub(1)?
            .checked_div(accuracy)?
            .try_into()
            .ok()
    }
}
