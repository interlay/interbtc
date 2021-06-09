use crate::{ext, Config, Error, Pallet, Slashable};
use codec::{Decode, Encode, HasCompact};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Saturating, Zero};
use sp_std::collections::btree_set::BTreeSet;

#[cfg(test)]
use mocktopus::macros::mockable;

pub use bitcoin::{Address as BtcAddress, PublicKey as BtcPublicKey};

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// added replace_collateral to vault, changed vaultStatus enum
    V2,
}

#[derive(Debug, PartialEq)]
pub enum CurrencySource<T: frame_system::Config> {
    /// Used by vault to back issued tokens
    Collateral(<T as frame_system::Config>::AccountId),
    /// Collateral that is locked, but not used to back issued tokens (e.g. griefing collateral)
    Griefing(<T as frame_system::Config>::AccountId),
    /// Unlocked balance
    FreeBalance(<T as frame_system::Config>::AccountId),
    /// Locked balance (like collateral but doesn't slash)
    ReservedBalance(<T as frame_system::Config>::AccountId),
    /// Funds within the liquidation vault
    LiquidationVault,
}

impl<T: Config> CurrencySource<T> {
    pub fn account_id(&self) -> <T as frame_system::Config>::AccountId {
        match self {
            CurrencySource::Collateral(x)
            | CurrencySource::Griefing(x)
            | CurrencySource::FreeBalance(x)
            | CurrencySource::ReservedBalance(x) => x.clone(),
            CurrencySource::LiquidationVault => Pallet::<T>::liquidation_vault_account_id(),
        }
    }

    pub fn current_balance(&self) -> Result<Collateral<T>, DispatchError> {
        match self {
            CurrencySource::Collateral(x) => {
                let vault = Pallet::<T>::get_rich_vault_from_id(&x)?;
                Ok(vault.data.backing_collateral)
            }
            CurrencySource::Griefing(x) => {
                let vault = Pallet::<T>::get_rich_vault_from_id(&x)?;
                let backing_collateral = if vault.data.is_liquidated() {
                    vault
                        .data
                        .liquidated_collateral
                        .checked_add(&vault.data.backing_collateral)
                        .ok_or(Error::<T>::ArithmeticOverflow)?
                } else {
                    vault.data.backing_collateral
                };

                let current = ext::collateral::get_reserved_balance::<T>(&x);
                Ok(current
                    .checked_sub(&backing_collateral)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?)
            }
            CurrencySource::FreeBalance(x) => Ok(ext::collateral::get_free_balance::<T>(x)),
            CurrencySource::ReservedBalance(x) => Ok(ext::collateral::get_reserved_balance::<T>(x)),
            CurrencySource::LiquidationVault => Ok(ext::collateral::get_reserved_balance::<T>(&self.account_id())),
        }
    }
}

pub(crate) type BalanceOf<T> = <T as Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;

#[derive(Encode, Decode, Clone, PartialEq, Debug, Default)]
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

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, Wrapped, Collateral, SignedFixedPoint> {
    /// Account identifier of the Vault
    pub id: AccountId,
    /// Number of tokens pending issue
    pub to_be_issued_tokens: Wrapped,
    /// Number of issued tokens
    pub issued_tokens: Wrapped,
    /// Number of tokens pending redeem
    pub to_be_redeemed_tokens: Wrapped,
    /// Bitcoin address of this Vault (P2PKH, P2SH, P2WPKH, P2WSH)
    pub wallet: Wallet,
    /// Number of tokens that have been requested for a replace through
    /// `request_replace`, but that have not been accepted yet by a new_vault.
    pub to_be_replaced_tokens: Wrapped,
    /// Amount of collateral that is locked as griefing collateral to be payed out if
    /// the old_vault fails to call execute_replace
    pub replace_collateral: Collateral,
    /// Amount of collateral that is locked for remaining to_be_redeemed
    /// tokens upon liquidation.
    pub liquidated_collateral: Collateral,
    /// Block height until which this Vault is banned from being used for
    /// Issue, Redeem (except during automatic liquidation) and Replace.
    pub banned_until: Option<BlockNumber>,
    /// Current status of the vault
    pub status: VaultStatus,
    /// Used to calculate the amount we need to slash.
    pub slash_per_token: SignedFixedPoint,
    /// Updated upon deposit or withdrawal.
    pub slash_tally: SignedFixedPoint,
    /// Amount of collateral that is locked to back tokens. This will be
    /// reduced if the vault has been slashed. If nomination is enabled
    /// this will be greater than the `collateral` supplied by the vault.
    /// Note that this excludes griefing collateral.
    pub backing_collateral: Collateral,
    /// Total amount of collateral after deposit / withdraw, excluding
    /// any amount that has been slashed. If nomination is enabled this
    /// will be greater than the `collateral` supplied by the vault.
    pub total_collateral: Collateral,
    /// Collateral supplied by the vault itself. If the vault has been
    /// slashed a proportional amount will be deducted from this.
    pub collateral: Collateral,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct SystemVault<Wrapped> {
    // Number of tokens pending issue
    pub to_be_issued_tokens: Wrapped,
    // Number of issued tokens
    pub issued_tokens: Wrapped,
    // Number of tokens pending redeem
    pub to_be_redeemed_tokens: Wrapped,
}

impl<
        AccountId: Default + Ord,
        BlockNumber: Default,
        Wrapped: HasCompact + Default,
        Collateral: HasCompact + Default,
        SignedFixedPoint: Default,
    > Vault<AccountId, BlockNumber, Wrapped, Collateral, SignedFixedPoint>
{
    pub(crate) fn new(
        id: AccountId,
        public_key: BtcPublicKey,
    ) -> Vault<AccountId, BlockNumber, Wrapped, Collateral, SignedFixedPoint> {
        let wallet = Wallet::new(public_key);
        Vault {
            id,
            wallet,
            banned_until: None,
            status: VaultStatus::Active(true),
            ..Default::default()
        }
    }

    pub fn is_liquidated(&self) -> bool {
        matches!(self.status, VaultStatus::Liquidated)
    }
}

pub type DefaultVault<T> = Vault<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    Wrapped<T>,
    Collateral<T>,
    SignedFixedPoint<T>,
>;

pub type DefaultSystemVault<T> = SystemVault<Wrapped<T>>;

pub(crate) trait UpdatableVault<T: Config> {
    fn id(&self) -> T::AccountId;

    fn issued_tokens(&self) -> Wrapped<T>;

    fn to_be_issued_tokens(&self) -> Wrapped<T>;

    fn increase_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult;

    fn increase_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult;

    fn increase_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult;

    fn decrease_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult;

    fn decrease_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult;

    fn decrease_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult;
}

pub struct RichVault<T: Config> {
    pub(crate) data: DefaultVault<T>,
}

impl<T: Config> UpdatableVault<T> for RichVault<T> {
    fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    fn issued_tokens(&self) -> Wrapped<T> {
        self.data.issued_tokens
    }

    fn to_be_issued_tokens(&self) -> Wrapped<T> {
        self.data.to_be_issued_tokens
    }

    fn increase_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault().increase_issued(tokens)
        } else {
            self.update(|v| {
                v.issued_tokens = v
                    .issued_tokens
                    .checked_add(&tokens)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                Ok(())
            })
        }
    }

    fn increase_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        // this function should never be called on liquidated vaults
        ensure!(!self.data.is_liquidated(), Error::<T>::VaultNotFound);

        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn increase_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        // this function should never be called on liquidated vaults
        ensure!(!self.data.is_liquidated(), Error::<T>::VaultNotFound);

        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn decrease_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault().decrease_issued(tokens)
        } else {
            self.update(|v| {
                v.issued_tokens = v
                    .issued_tokens
                    .checked_sub(&tokens)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                Ok(())
            })
        }
    }

    fn decrease_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault().decrease_to_be_issued(tokens)
        } else {
            self.update(|v| {
                v.to_be_issued_tokens = v
                    .to_be_issued_tokens
                    .checked_sub(&tokens)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                Ok(())
            })
        }
    }

    fn decrease_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        // in addition to the change to this vault, _also_ change the liquidation vault
        if self.data.is_liquidated() {
            Pallet::<T>::get_rich_liquidation_vault().decrease_to_be_redeemed(tokens)?;
        }

        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichVault<T> {
    pub(crate) fn backed_tokens(&self) -> Result<Wrapped<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?)
    }

    pub fn get_collateral(&self) -> Collateral<T> {
        self.data.backing_collateral
    }

    pub fn get_free_collateral(&self) -> Result<Collateral<T>, DispatchError> {
        let used_collateral = self.get_used_collateral()?;
        Ok(self
            .get_collateral()
            .checked_sub(&used_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_used_collateral(&self) -> Result<Collateral<T>, DispatchError> {
        let issued_tokens = self.backed_tokens()?;
        let issued_tokens_in_collateral = ext::oracle::wrapped_to_collateral::<T>(issued_tokens)?;

        let secure_threshold = Pallet::<T>::secure_collateral_threshold();

        let used_collateral = secure_threshold
            .checked_mul_int(issued_tokens_in_collateral)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        Ok(self.data.backing_collateral.min(used_collateral))
    }

    pub fn issuable_tokens(&self) -> Result<Wrapped<T>, DispatchError> {
        // unable to issue additional tokens when banned
        if self.is_banned() {
            return Ok(0u32.into());
        }

        let free_collateral = self.get_free_collateral()?;

        let secure_threshold = Pallet::<T>::secure_collateral_threshold();

        let issuable =
            Pallet::<T>::calculate_max_wrapped_from_collateral_for_threshold(free_collateral, secure_threshold)?;

        Ok(issuable)
    }

    pub fn redeemable_tokens(&self) -> Result<Wrapped<T>, DispatchError> {
        // unable to redeem additional tokens when banned
        if self.is_banned() {
            return Ok(0u32.into());
        }

        let redeemable_tokens = self
            .data
            .issued_tokens
            .checked_sub(&self.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        Ok(redeemable_tokens)
    }

    pub(crate) fn set_to_be_replaced_amount(
        &mut self,
        tokens: Wrapped<T>,
        griefing_collateral: Collateral<T>,
    ) -> DispatchResult {
        self.update(|v| {
            v.to_be_replaced_tokens = tokens;
            v.replace_collateral = griefing_collateral;
            Ok(())
        })
    }

    pub(crate) fn set_accept_new_issues(&mut self, accept_new_issues: bool) -> DispatchResult {
        self.update(|v| {
            v.status = VaultStatus::Active(accept_new_issues);
            Ok(())
        })
    }

    pub(crate) fn issue_tokens(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.decrease_to_be_issued(tokens)?;
        self.increase_issued(tokens)
    }

    pub(crate) fn decrease_tokens(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.decrease_to_be_redeemed(tokens)?;
        self.decrease_issued(tokens)
        // Note: slashing of collateral must be called where this function is called (e.g. in Redeem)
    }

    pub(crate) fn increase_liquidated_collateral(&mut self, amount: Collateral<T>) -> DispatchResult {
        self.update(|v| {
            v.liquidated_collateral = v
                .liquidated_collateral
                .checked_add(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    pub(crate) fn decrease_liquidated_collateral(&mut self, amount: Collateral<T>) -> DispatchResult {
        self.update(|v| {
            v.liquidated_collateral = v
                .liquidated_collateral
                .checked_sub(&amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub(crate) fn slash_to_liquidation_vault(&mut self, amount: Collateral<T>) -> DispatchResult {
        self.slash_collateral(amount)?;
        Pallet::<T>::transfer_funds(
            CurrencySource::ReservedBalance(self.id()),
            CurrencySource::LiquidationVault,
            amount,
        )?;
        Ok(())
    }

    pub(crate) fn liquidate<V: UpdatableVault<T>>(
        &mut self,
        liquidation_vault: &mut V,
        status: VaultStatus,
    ) -> Result<Collateral<T>, DispatchError> {
        // we liquidate at most SECURE_THRESHOLD * collateral
        // this value is the amount of collateral held for the issued + to_be_issued
        let liquidated_collateral = self.get_used_collateral()?;

        // amount of tokens being backed
        let collateral_tokens = self.backed_tokens()?;

        // (liquidated_collateral * (collateral_tokens - to_be_redeemed_tokens)) / collateral_tokens
        let liquidated_collateral_excluding_to_be_redeemed = Pallet::<T>::calculate_collateral(
            liquidated_collateral,
            collateral_tokens
                .checked_sub(&self.data.to_be_redeemed_tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?,
            collateral_tokens,
        )?;

        let collateral_for_to_be_redeemed =
            liquidated_collateral.saturating_sub(liquidated_collateral_excluding_to_be_redeemed);

        // slash backing collateral used for issued + to_be_issued to the liquidation vault
        self.slash_to_liquidation_vault(liquidated_collateral_excluding_to_be_redeemed)?;
        // temporarily slash additional collateral for the to_be_redeemed tokens
        // this is re-distributed once the tokens are burned
        self.slash_collateral(collateral_for_to_be_redeemed)?;
        self.increase_liquidated_collateral(collateral_for_to_be_redeemed)?;

        // Copy all tokens to the liquidation vault
        liquidation_vault.increase_issued(self.data.issued_tokens)?;
        liquidation_vault.increase_to_be_issued(self.data.to_be_issued_tokens)?;
        liquidation_vault.increase_to_be_redeemed(self.data.to_be_redeemed_tokens)?;

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
        <crate::Vaults<T>>::mutate(&self.data.id, func)?;
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

pub(crate) struct RichSystemVault<T: Config> {
    pub(crate) data: DefaultSystemVault<T>,
}

#[cfg_attr(test, mockable)]
impl<T: Config> RichSystemVault<T> {
    pub(crate) fn redeemable_tokens(&self) -> Result<Wrapped<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_sub(&self.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub(crate) fn backed_tokens(&self) -> Result<Wrapped<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?)
    }
}

impl<T: Config> UpdatableVault<T> for RichSystemVault<T> {
    fn id(&self) -> T::AccountId {
        Pallet::<T>::liquidation_vault_account_id()
    }

    fn issued_tokens(&self) -> Wrapped<T> {
        self.data.issued_tokens
    }

    fn to_be_issued_tokens(&self) -> Wrapped<T> {
        self.data.to_be_issued_tokens
    }

    fn increase_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn increase_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn increase_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn decrease_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn decrease_to_be_issued(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn decrease_to_be_redeemed(&mut self, tokens: Wrapped<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
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
        <crate::LiquidationVault<T>>::set(self.data.clone());
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
