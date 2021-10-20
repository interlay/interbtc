use crate::{ext, Config, Error, Pallet};
use codec::{Decode, Encode, HasCompact};
use currency::Amount;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
};

pub use primitives::{VaultCurrencyPair, VaultId};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero};
use sp_std::collections::btree_set::BTreeSet;

#[cfg(test)]
use mocktopus::macros::mockable;

pub use bitcoin::{Address as BtcAddress, PublicKey as BtcPublicKey};

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// added replace_collateral to vault, changed vaultStatus enum
    V2,
}

#[derive(Debug, PartialEq)]
pub enum CurrencySource<T: frame_system::Config + staking::Config> {
    /// Used by vault to back issued tokens
    Collateral(DefaultVaultId<T>),
    /// Collateral put down by request_replace, but that has not been accepted yet
    AvailableReplaceCollateral(DefaultVaultId<T>),
    /// Collateral that is locked, but not used to back issued tokens (e.g. griefing collateral)
    ActiveReplaceCollateral(DefaultVaultId<T>),
    /// User's issue griefing collateral
    UserGriefing(<T as frame_system::Config>::AccountId),
    /// Unlocked balance
    FreeBalance(<T as frame_system::Config>::AccountId),
    /// Locked balance (like collateral but doesn't slash)
    LiquidatedCollateral(DefaultVaultId<T>),
    /// Funds within the liquidation vault
    LiquidationVault(DefaultVaultCurrencyPair<T>),
}

#[cfg_attr(test, mockable)]
impl<T: Config> CurrencySource<T> {
    pub fn account_id(&self) -> <T as frame_system::Config>::AccountId {
        match self {
            CurrencySource::Collateral(DefaultVaultId::<T> { account_id: x, .. })
            | CurrencySource::AvailableReplaceCollateral(DefaultVaultId::<T> { account_id: x, .. })
            | CurrencySource::ActiveReplaceCollateral(DefaultVaultId::<T> { account_id: x, .. })
            | CurrencySource::UserGriefing(x)
            | CurrencySource::FreeBalance(x)
            | CurrencySource::LiquidatedCollateral(DefaultVaultId::<T> { account_id: x, .. }) => x.clone(),
            CurrencySource::LiquidationVault(_) => Pallet::<T>::liquidation_vault_account_id(),
        }
    }

    pub fn current_balance(&self, currency_id: CurrencyId<T>) -> Result<crate::Amount<T>, DispatchError> {
        let amount = match self {
            CurrencySource::Collateral(vault_id) => Pallet::<T>::get_backing_collateral(vault_id)?,
            CurrencySource::AvailableReplaceCollateral(vault_id) => {
                let vault = Pallet::<T>::get_vault_from_id(vault_id)?;
                Amount::new(vault.replace_collateral, T::GetGriefingCollateralCurrencyId::get())
            }
            CurrencySource::ActiveReplaceCollateral(vault_id) => {
                let vault = Pallet::<T>::get_vault_from_id(vault_id)?;
                Amount::new(
                    vault.active_replace_collateral,
                    T::GetGriefingCollateralCurrencyId::get(),
                )
            }
            CurrencySource::UserGriefing(x) => ext::currency::get_reserved_balance::<T>(currency_id, x),
            CurrencySource::FreeBalance(x) => ext::currency::get_free_balance::<T>(currency_id, x),
            CurrencySource::LiquidatedCollateral(vault_id) => {
                let vault = Pallet::<T>::get_vault_from_id(vault_id)?;
                Amount::new(vault.liquidated_collateral, vault_id.collateral_currency())
            }
            CurrencySource::LiquidationVault(currency_pair) => {
                let liquidation_vault = Pallet::<T>::get_liquidation_vault(&currency_pair);
                Amount::new(liquidation_vault.collateral, currency_pair.collateral)
            }
        };
        Ok(amount)
    }
}

pub(crate) type BalanceOf<T> = <T as Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type SignedFixedPoint<T> = <T as currency::Config>::SignedFixedPoint;

pub(crate) type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

pub(crate) type SignedInner<T> = <T as currency::Config>::SignedInner;

pub type CurrencyId<T> = <T as staking::Config>::CurrencyId;

pub type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub type DefaultVaultCurrencyPair<T> = VaultCurrencyPair<CurrencyId<T>>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default, TypeInfo)]
pub struct Wallet {
    // store all addresses for `report_vault_theft` checks
    pub addresses: BTreeSet<BtcAddress>,
    // we use this public key to generate new addresses
    pub public_key: BtcPublicKey,
}

impl Wallet {
    pub fn new(public_key: BtcPublicKey) -> Self {
        Self {
            addresses: BTreeSet::new(),
            public_key,
        }
    }

    pub fn has_btc_address(&self, address: &BtcAddress) -> bool {
        self.addresses.contains(address)
    }

    pub fn add_btc_address(&mut self, address: BtcAddress) {
        // TODO: add maximum or griefing collateral
        self.addresses.insert(address);
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo)]
pub enum VaultStatus {
    /// Vault is active - bool=true indicates that the vault accepts new issue requests
    Active(bool),

    /// Vault has been liquidated
    Liquidated,

    /// Vault theft has been reported
    CommittedTheft,
}

impl Default for VaultStatus {
    fn default() -> Self {
        VaultStatus::Active(true)
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
    /// Account identifier of the Vault
    pub id: VaultId<AccountId, CurrencyId>,
    /// Bitcoin address of this Vault (P2PKH, P2SH, P2WPKH, P2WSH)
    pub wallet: Wallet,
    /// Current status of the vault
    pub status: VaultStatus,
    /// Block height until which this Vault is banned from being used for
    /// Issue, Redeem (except during automatic liquidation) and Replace.
    pub banned_until: Option<BlockNumber>,
    /// Number of tokens pending issue
    pub to_be_issued_tokens: Balance,
    /// Number of issued tokens
    pub issued_tokens: Balance,
    /// Number of tokens pending redeem
    pub to_be_redeemed_tokens: Balance,
    /// Number of tokens that have been requested for a replace through
    /// `request_replace`, but that have not been accepted yet by a new_vault.
    pub to_be_replaced_tokens: Balance,
    /// Amount of collateral that is available as griefing collateral to vaults accepting
    /// a replace request. It is to be payed out if the old_vault fails to call execute_replace.
    pub replace_collateral: Balance,
    /// Amount of collateral locked for accepted replace requests.
    pub active_replace_collateral: Balance,
    /// Amount of collateral that is locked for remaining to_be_redeemed
    /// tokens upon liquidation.
    pub liquidated_collateral: Balance,
}

#[derive(Encode, Decode, Clone, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct SystemVault<Balance, CurrencyId: Copy> {
    // Number of tokens pending issue
    pub to_be_issued_tokens: Balance,
    // Number of issued tokens
    pub issued_tokens: Balance,
    // Number of tokens pending redeem
    pub to_be_redeemed_tokens: Balance,
    // amount of collateral stored
    pub collateral: Balance,
    /// the currency used for collateral
    pub currency_pair: VaultCurrencyPair<CurrencyId>,
}

impl<AccountId: Default + Ord, BlockNumber: Default, Balance: HasCompact + Default, CurrencyId: Copy>
    Vault<AccountId, BlockNumber, Balance, CurrencyId>
{
    // note: public only for testing purposes
    pub fn new(
        id: VaultId<AccountId, CurrencyId>,
        public_key: BtcPublicKey,
    ) -> Vault<AccountId, BlockNumber, Balance, CurrencyId> {
        let wallet = Wallet::new(public_key);
        Vault {
            id,
            wallet,
            banned_until: None,
            status: VaultStatus::Active(true),
            issued_tokens: Default::default(),
            liquidated_collateral: Default::default(),
            replace_collateral: Default::default(),
            to_be_issued_tokens: Default::default(),
            to_be_redeemed_tokens: Default::default(),
            to_be_replaced_tokens: Default::default(),
            active_replace_collateral: Default::default(),
        }
    }

    pub fn is_liquidated(&self) -> bool {
        matches!(self.status, VaultStatus::Liquidated)
    }
}

pub type DefaultVault<T> = Vault<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    BalanceOf<T>,
    CurrencyId<T>,
>;

pub type DefaultSystemVault<T> = SystemVault<BalanceOf<T>, CurrencyId<T>>;

#[cfg_attr(feature = "integration-tests", visibility::make(pub))]
pub(crate) trait UpdatableVault<T: Config> {
    fn id(&self) -> DefaultVaultId<T>;

    fn issued_tokens(&self) -> Amount<T>;

    fn to_be_issued_tokens(&self) -> Amount<T>;

    fn increase_issued(&mut self, tokens: &Amount<T>) -> DispatchResult;

    fn increase_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult;

    fn increase_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult;

    fn decrease_issued(&mut self, tokens: &Amount<T>) -> DispatchResult;

    fn decrease_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult;

    fn decrease_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult;
}

pub struct RichVault<T: Config> {
    pub(crate) data: DefaultVault<T>,
}

impl<T: Config> UpdatableVault<T> for RichVault<T> {
    fn id(&self) -> DefaultVaultId<T> {
        self.data.id.clone()
    }

    fn issued_tokens(&self) -> Amount<T> {
        Amount::new(self.data.issued_tokens, self.id().wrapped_currency())
    }

    fn to_be_issued_tokens(&self) -> Amount<T> {
        Amount::new(self.data.to_be_issued_tokens, self.id().wrapped_currency())
    }

    fn increase_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault(&self.data.id.currencies).increase_issued(tokens)
        } else {
            ext::reward::deposit_stake::<T>(&self.id(), tokens)?;
            let new_value = self.issued_tokens().checked_add(&tokens)?.amount();
            self.update(|v| {
                v.issued_tokens = new_value;
                Ok(())
            })
        }
    }

    fn increase_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        // this function should never be called on liquidated vaults
        ensure!(!self.data.is_liquidated(), Error::<T>::VaultNotFound);

        let new_value = self.to_be_issued_tokens().checked_add(&tokens)?.amount();
        self.update(|v| {
            v.to_be_issued_tokens = new_value;
            Ok(())
        })
    }

    fn increase_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult {
        // this function should never be called on liquidated vaults
        ensure!(!self.data.is_liquidated(), Error::<T>::VaultNotFound);

        let new_value = self.to_be_redeemed_tokens().checked_add(&tokens)?.amount();
        self.update(|v| {
            v.to_be_redeemed_tokens = new_value;
            Ok(())
        })
    }

    fn decrease_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault(&self.data.id.currencies).decrease_issued(tokens)
        } else {
            ext::reward::withdraw_stake::<T>(&self.id(), tokens)?;
            let new_value = self.issued_tokens().checked_sub(&tokens)?.amount();
            self.update(|v| {
                v.issued_tokens = new_value;
                Ok(())
            })
        }
    }

    fn decrease_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault(&self.data.id.currencies).decrease_to_be_issued(tokens)
        } else {
            let new_value = self.to_be_issued_tokens().checked_sub(&tokens)?.amount();
            self.update(|v| {
                v.to_be_issued_tokens = new_value;
                Ok(())
            })
        }
    }

    fn decrease_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult {
        // in addition to the change to this vault, _also_ change the liquidation vault
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault(&self.data.id.currencies).decrease_to_be_redeemed(tokens)?;
        }
        let new_value = self.to_be_redeemed_tokens().checked_sub(&tokens)?.amount();
        self.update(|v| {
            v.to_be_redeemed_tokens = new_value;
            Ok(())
        })
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichVault<T> {
    pub(crate) fn wrapped_currency(&self) -> CurrencyId<T> {
        self.data.id.wrapped_currency()
    }

    pub(crate) fn backed_tokens(&self) -> Result<Amount<T>, DispatchError> {
        let amount = self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        Ok(Amount::new(amount, self.wrapped_currency()))
    }

    pub(crate) fn to_be_replaced_tokens(&self) -> Amount<T> {
        Amount::new(self.data.to_be_replaced_tokens, self.wrapped_currency())
    }

    pub(crate) fn to_be_redeemed_tokens(&self) -> Amount<T> {
        Amount::new(self.data.to_be_redeemed_tokens, self.wrapped_currency())
    }

    pub(crate) fn liquidated_collateral(&self) -> Amount<T> {
        Amount::new(self.data.liquidated_collateral, self.data.id.currencies.collateral)
    }

    pub(crate) fn replace_collateral(&self) -> Amount<T> {
        Amount::new(self.data.replace_collateral, T::GetGriefingCollateralCurrencyId::get())
    }

    pub fn get_vault_collateral(&self) -> Result<Amount<T>, DispatchError> {
        Pallet::<T>::compute_collateral(&self.id())
    }

    pub fn get_total_collateral(&self) -> Result<Amount<T>, DispatchError> {
        Pallet::<T>::get_backing_collateral(&self.id())
    }

    pub fn get_free_collateral(&self) -> Result<Amount<T>, DispatchError> {
        let used_collateral = self.get_used_collateral()?;
        self.get_total_collateral()?.checked_sub(&used_collateral)
    }

    pub fn get_used_collateral(&self) -> Result<Amount<T>, DispatchError> {
        let issued_tokens = self.backed_tokens()?;
        let issued_tokens_in_collateral = issued_tokens.convert_to(self.data.id.currencies.collateral)?;

        let secure_threshold =
            Pallet::<T>::secure_collateral_threshold(&self.data.id.currencies).ok_or(Error::<T>::ThresholdNotSet)?;

        let used_collateral = issued_tokens_in_collateral.checked_fixed_point_mul(&secure_threshold)?;

        self.get_total_collateral()?.min(&used_collateral)
    }

    pub fn issuable_tokens(&self) -> Result<Amount<T>, DispatchError> {
        // unable to issue additional tokens when banned
        if self.is_banned() {
            return Ok(Amount::new(0u32.into(), self.wrapped_currency()));
        }

        // used_collateral = (exchange_rate * (issued_tokens + to_be_issued_tokens)) * secure_collateral_threshold
        // free_collateral = collateral - used_collateral
        let free_collateral = self.get_free_collateral()?;

        let secure_threshold =
            Pallet::<T>::secure_collateral_threshold(&self.data.id.currencies).ok_or(Error::<T>::ThresholdNotSet)?;

        // issuable_tokens = (free_collateral / exchange_rate) / secure_collateral_threshold
        let issuable = Pallet::<T>::calculate_max_wrapped_from_collateral_for_threshold(
            &free_collateral,
            self.wrapped_currency(),
            secure_threshold,
        )?;

        Ok(issuable)
    }

    pub fn redeemable_tokens(&self) -> Result<Amount<T>, DispatchError> {
        // unable to redeem additional tokens when banned
        if self.is_banned() {
            return Ok(Amount::new(0u32.into(), self.wrapped_currency()));
        }

        self.issued_tokens().checked_sub(&self.to_be_redeemed_tokens())
    }

    pub(crate) fn set_to_be_replaced_amount(&mut self, tokens: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_replaced_tokens = tokens.amount();
            Ok(())
        })
    }

    pub(crate) fn increase_available_replace_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.replace_collateral = v
                .replace_collateral
                .checked_add(&amount.amount())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    pub(crate) fn decrease_available_replace_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.replace_collateral = v
                .replace_collateral
                .checked_sub(&amount.amount())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub(crate) fn increase_active_replace_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.active_replace_collateral = v
                .active_replace_collateral
                .checked_add(&amount.amount())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    pub(crate) fn decrease_active_replace_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.active_replace_collateral = v
                .active_replace_collateral
                .checked_sub(&amount.amount())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub(crate) fn set_accept_new_issues(&mut self, accept_new_issues: bool) -> DispatchResult {
        self.update(|v| {
            v.status = VaultStatus::Active(accept_new_issues);
            Ok(())
        })
    }

    pub(crate) fn issue_tokens(&mut self, tokens: &Amount<T>) -> DispatchResult {
        self.decrease_to_be_issued(tokens)?;
        self.increase_issued(tokens)
    }

    pub(crate) fn decrease_tokens(&mut self, tokens: &Amount<T>) -> DispatchResult {
        self.decrease_to_be_redeemed(tokens)?;
        self.decrease_issued(tokens)
        // Note: slashing of collateral must be called where this function is called (e.g. in Redeem)
    }

    pub(crate) fn increase_liquidated_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.liquidated_collateral = v
                .liquidated_collateral
                .checked_add(&amount.amount())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    pub(crate) fn decrease_liquidated_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        self.update(|v| {
            v.liquidated_collateral = v
                .liquidated_collateral
                .checked_sub(&amount.amount())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub(crate) fn slash_for_to_be_redeemed(&mut self, amount: &Amount<T>) -> DispatchResult {
        let vault_id = self.id();
        let collateral = self.get_vault_collateral()?.min(amount)?;
        ext::staking::withdraw_stake::<T>(&vault_id, &vault_id.account_id, &collateral)?;
        self.increase_liquidated_collateral(&collateral)?;
        Ok(())
    }

    pub(crate) fn slash_to_liquidation_vault(&mut self, amount: &Amount<T>) -> DispatchResult {
        let vault_id = self.id();

        // get the collateral supplied by the vault (i.e. excluding nomination)
        let collateral = self.get_vault_collateral()?;
        let (to_withdraw, to_slash) = amount
            .checked_sub(&collateral)
            .and_then(|leftover| Ok((collateral, Some(leftover))))
            .unwrap_or((amount.clone(), None));

        // "slash" vault first
        ext::staking::withdraw_stake::<T>(&vault_id, &vault_id.account_id, &to_withdraw)?;
        // take remainder from nominators
        if let Some(to_slash) = to_slash {
            ext::staking::slash_stake::<T>(self.wrapped_currency(), &vault_id, &to_slash)?;
        }

        Pallet::<T>::transfer_funds(
            CurrencySource::LiquidatedCollateral(self.id()),
            CurrencySource::LiquidationVault(vault_id.currencies),
            amount,
        )?;
        Ok(())
    }

    pub(crate) fn get_theft_fee_max(&self) -> Result<Amount<T>, DispatchError> {
        let collateral = Pallet::<T>::compute_collateral(&self.id())?;
        let theft_reward = ext::fee::get_theft_fee::<T>(&collateral)?;

        let theft_fee_max = ext::fee::get_theft_fee_max::<T>();
        let max_theft_reward = theft_fee_max.convert_to(self.data.id.currencies.collateral)?;
        if theft_reward.le(&max_theft_reward)? {
            Ok(theft_reward)
        } else {
            Ok(max_theft_reward)
        }
    }

    pub(crate) fn liquidate(
        &mut self,
        status: VaultStatus,
        reporter: Option<T::AccountId>,
    ) -> Result<Amount<T>, DispatchError> {
        let vault_id = self.id();

        // pay the theft report fee first
        if let Some(ref reporter_id) = reporter {
            let reward = self.get_theft_fee_max()?;
            Pallet::<T>::force_withdraw_collateral(&vault_id, &reward)?;
            reward.transfer(&vault_id.account_id, reporter_id)?;
        }

        // we liquidate at most SECURE_THRESHOLD * collateral
        // this value is the amount of collateral held for the issued + to_be_issued
        let liquidated_collateral = self.get_used_collateral()?;

        // amount of tokens being backed
        let collateral_tokens = self.backed_tokens()?;

        // (liquidated_collateral * (collateral_tokens - to_be_redeemed_tokens)) / collateral_tokens
        let liquidated_collateral_excluding_to_be_redeemed = Pallet::<T>::calculate_collateral(
            &liquidated_collateral,
            &collateral_tokens.checked_sub(&self.to_be_redeemed_tokens())?,
            &collateral_tokens,
        )?;

        let collateral_for_to_be_redeemed =
            liquidated_collateral.saturating_sub(&liquidated_collateral_excluding_to_be_redeemed)?;

        // slash collateral for the to_be_redeemed tokens
        // this is re-deposited once the tokens are burned
        self.slash_for_to_be_redeemed(&collateral_for_to_be_redeemed)?;

        // slash collateral used for issued + to_be_issued to the liquidation vault
        self.slash_to_liquidation_vault(&liquidated_collateral_excluding_to_be_redeemed)?;

        // Copy all tokens to the liquidation vault
        let mut liquidation_vault = Pallet::<T>::get_rich_liquidation_vault(&self.data.id.currencies);
        liquidation_vault.increase_issued(&self.issued_tokens())?;
        liquidation_vault.increase_to_be_issued(&self.to_be_issued_tokens())?;
        liquidation_vault.increase_to_be_redeemed(&self.to_be_redeemed_tokens())?;
        // todo: clear replace collateral?
        // withdraw stake from the reward pool
        ext::reward::withdraw_stake::<T>(&vault_id, &self.issued_tokens())?;

        // Update vault: clear to_be_issued & issued_tokens, but don't touch to_be_redeemed
        let _ = self.update(|v| {
            v.to_be_issued_tokens = Zero::zero();
            v.issued_tokens = Zero::zero();
            v.status = status;
            Ok(())
        });

        Ok(liquidated_collateral_excluding_to_be_redeemed)
    }

    pub fn ensure_not_banned(&self) -> DispatchResult {
        if self.is_banned() {
            Err(Error::<T>::VaultBanned.into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn is_banned(&self) -> bool {
        match self.data.banned_until {
            None => false,
            Some(until) => ext::security::active_block_number::<T>() <= until,
        }
    }

    pub fn ban_until(&mut self, height: T::BlockNumber) {
        let _ = self.update(|v| {
            v.banned_until = Some(height);
            Ok(())
        });
    }

    fn new_deposit_public_key(&self, secure_id: H256) -> Result<BtcPublicKey, DispatchError> {
        let vault_public_key = self.data.wallet.public_key.clone();
        let vault_public_key = vault_public_key
            .new_deposit_public_key(secure_id)
            .map_err(|_| Error::<T>::InvalidPublicKey)?;

        Ok(vault_public_key)
    }

    pub(crate) fn insert_deposit_address(&mut self, btc_address: BtcAddress) {
        let _ = self.update(|v| {
            v.wallet.add_btc_address(btc_address);
            Ok(())
        });
    }

    pub(crate) fn new_deposit_address(&mut self, secure_id: H256) -> Result<BtcAddress, DispatchError> {
        let public_key = self.new_deposit_public_key(secure_id)?;
        let btc_address = BtcAddress::P2WPKHv0(public_key.to_hash());
        self.insert_deposit_address(btc_address);
        Ok(btc_address)
    }

    pub(crate) fn update_public_key(&mut self, public_key: BtcPublicKey) {
        let _ = self.update(|v| {
            v.wallet.public_key = public_key.clone();
            Ok(())
        });
    }

    fn update<F>(&mut self, func: F) -> DispatchResult
    where
        F: Fn(&mut DefaultVault<T>) -> DispatchResult,
    {
        func(&mut self.data)?;
        <crate::Vaults<T>>::insert(&self.id(), &self.data);
        Ok(())
    }
}

impl<T: Config> From<&RichVault<T>> for DefaultVault<T> {
    fn from(rv: &RichVault<T>) -> DefaultVault<T> {
        rv.data.clone()
    }
}

impl<T: Config> From<DefaultVault<T>> for RichVault<T> {
    fn from(vault: DefaultVault<T>) -> RichVault<T> {
        RichVault { data: vault }
    }
}

#[cfg_attr(feature = "integration-tests", visibility::make(pub))]
pub(crate) struct RichSystemVault<T: Config> {
    pub(crate) data: DefaultSystemVault<T>,
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichSystemVault<T> {
    pub(crate) fn wrapped_currency(&self) -> CurrencyId<T> {
        self.data.currency_pair.wrapped
    }

    pub(crate) fn redeemable_tokens(&self) -> Result<Amount<T>, DispatchError> {
        self.issued_tokens().checked_sub(&self.to_be_redeemed_tokens())
    }

    pub(crate) fn backed_tokens(&self) -> Result<Amount<T>, DispatchError> {
        self.issued_tokens().checked_add(&self.to_be_issued_tokens())
    }

    pub(crate) fn to_be_redeemed_tokens(&self) -> Amount<T> {
        Amount::new(self.data.to_be_redeemed_tokens, self.wrapped_currency())
    }

    pub(crate) fn collateral(&self) -> Amount<T> {
        Amount::new(self.data.collateral, self.data.currency_pair.collateral)
    }

    pub(crate) fn increase_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        let new_value = self.collateral().checked_add(&amount)?.amount();
        self.update(|v| {
            v.collateral = new_value;
            Ok(())
        })
    }

    pub(crate) fn decrease_collateral(&mut self, amount: &Amount<T>) -> DispatchResult {
        let new_value = self.collateral().checked_sub(&amount)?.amount();
        self.update(|v| {
            v.collateral = new_value;
            Ok(())
        })
    }
}

impl<T: Config> UpdatableVault<T> for RichSystemVault<T> {
    fn id(&self) -> DefaultVaultId<T> {
        let account_id = Pallet::<T>::liquidation_vault_account_id();
        VaultId::new(
            account_id,
            self.data.currency_pair.collateral,
            self.data.currency_pair.wrapped,
        )
    }

    fn issued_tokens(&self) -> Amount<T> {
        Amount::new(self.data.issued_tokens, self.wrapped_currency())
    }

    fn to_be_issued_tokens(&self) -> Amount<T> {
        Amount::new(self.data.to_be_issued_tokens, self.wrapped_currency())
    }

    fn increase_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.issued_tokens().checked_add(&tokens)?.amount();
        self.update(|v| {
            v.issued_tokens = new_value;
            Ok(())
        })
    }

    fn increase_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.to_be_issued_tokens().checked_add(&tokens)?.amount();
        self.update(|v| {
            v.to_be_issued_tokens = new_value;
            Ok(())
        })
    }

    fn increase_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.to_be_redeemed_tokens().checked_add(&tokens)?.amount();
        self.update(|v| {
            v.to_be_redeemed_tokens = new_value;
            Ok(())
        })
    }

    fn decrease_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.issued_tokens().checked_sub(&tokens)?.amount();
        self.update(|v| {
            v.issued_tokens = new_value;
            Ok(())
        })
    }

    fn decrease_to_be_issued(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.to_be_issued_tokens().checked_sub(&tokens)?.amount();
        self.update(|v| {
            v.to_be_issued_tokens = new_value;
            Ok(())
        })
    }

    fn decrease_to_be_redeemed(&mut self, tokens: &Amount<T>) -> DispatchResult {
        let new_value = self.to_be_redeemed_tokens().checked_sub(&tokens)?.amount();
        self.update(|v| {
            v.to_be_redeemed_tokens = new_value;
            Ok(())
        })
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichSystemVault<T> {
    fn update<F>(&mut self, func: F) -> Result<(), DispatchError>
    where
        F: Fn(&mut DefaultSystemVault<T>) -> Result<(), DispatchError>,
    {
        func(&mut self.data)?;
        <crate::LiquidationVault<T>>::insert(&self.data.currency_pair, self.data.clone());
        Ok(())
    }
}

impl<T: Config> From<&RichSystemVault<T>> for DefaultSystemVault<T> {
    fn from(rv: &RichSystemVault<T>) -> DefaultSystemVault<T> {
        rv.data.clone()
    }
}

impl<T: Config> From<DefaultSystemVault<T>> for RichSystemVault<T> {
    fn from(vault: DefaultSystemVault<T>) -> RichSystemVault<T> {
        RichSystemVault { data: vault }
    }
}
