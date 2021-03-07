use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageValue;
use crate::{ext, Config, Error, Module};
use codec::{Decode, Encode, HasCompact};
use frame_support::traits::Currency;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, StorageMap,
};
use sp_arithmetic::FixedPointNumber;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Zero};
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
}

#[derive(Debug, PartialEq)]
pub enum CurrencySource<T: frame_system::Config> {
    /// used by vault to back PolkaBTC
    Backing(<T as frame_system::Config>::AccountId),
    /// Collateral that is locked, but not used to back PolkaBTC (e.g. griefing collateral)
    Griefing(<T as frame_system::Config>::AccountId),
    /// Unlocked balance
    FreeBalance(<T as frame_system::Config>::AccountId),
    /// funds within the liquidation vault
    LiquidationVault,
}

impl<T: Config> CurrencySource<T> {
    pub fn account_id(&self) -> <T as frame_system::Config>::AccountId {
        match self {
            CurrencySource::Backing(x)
            | CurrencySource::Griefing(x)
            | CurrencySource::FreeBalance(x) => x.clone(),
            CurrencySource::LiquidationVault => Module::<T>::get_rich_liquidation_vault().data.id,
        }
    }
    pub fn current_balance(&self) -> Result<DOT<T>, DispatchError> {
        match self {
            CurrencySource::Backing(x) => Ok(Module::<T>::get_rich_vault_from_id(&x)?
                .data
                .backing_collateral),
            CurrencySource::Griefing(x) => {
                let backing_collateral = match Module::<T>::get_rich_vault_from_id(&x) {
                    Ok(vault) => vault.data.backing_collateral,
                    Err(_) => 0u32.into(),
                };
                let current = ext::collateral::for_account::<T>(&x);
                Ok(current
                    .checked_sub(&backing_collateral)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?)
            }
            CurrencySource::FreeBalance(x) => Ok(ext::collateral::get_free_balance::<T>(x)),
            CurrencySource::LiquidationVault => {
                Ok(ext::collateral::for_account::<T>(&self.account_id()))
            }
        }
    }
}

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> = <<T as treasury::Config>::PolkaBTC as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;

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
    /// Vault is active
    Active = 0,

    /// Vault has been liquidated
    Liquidated = 1,

    /// Vault theft has been reported
    CommittedTheft = 2,
}

impl Default for VaultStatus {
    fn default() -> Self {
        VaultStatus::Active
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, PolkaBTC, DOT> {
    // Account identifier of the Vault
    pub id: AccountId,
    // number of PolkaBTC tokens that have been requested for a replace through
    // `request_replace`, but that have not been accepted yet by a new_vault.
    pub to_be_replaced_tokens: PolkaBTC,
    // Number of PolkaBTC tokens pending issue
    pub to_be_issued_tokens: PolkaBTC,
    // Number of issued PolkaBTC tokens
    pub issued_tokens: PolkaBTC,
    // Number of PolkaBTC tokens pending redeem
    pub to_be_redeemed_tokens: PolkaBTC,
    // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub wallet: Wallet,
    // amount of DOT collateral that is locked to back PolkaBTC tokens. Note that
    // this excludes griefing collateral.
    pub backing_collateral: DOT,
    // Block height until which this Vault is banned from being
    // used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
    /// Current status of the vault
    pub status: VaultStatus,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct SystemVault<AccountId, PolkaBTC> {
    // Account identifier of the Vault
    pub id: AccountId,
    // Number of PolkaBTC tokens pending issue
    pub to_be_issued_tokens: PolkaBTC,
    // Number of issued PolkaBTC tokens
    pub issued_tokens: PolkaBTC,
    // Number of PolkaBTC tokens pending redeem
    pub to_be_redeemed_tokens: PolkaBTC,
}

impl<AccountId, BlockNumber, PolkaBTC: HasCompact + Default, DOT: HasCompact + Default>
    Vault<AccountId, BlockNumber, PolkaBTC, DOT>
{
    pub(crate) fn new(
        id: AccountId,
        public_key: BtcPublicKey,
    ) -> Vault<AccountId, BlockNumber, PolkaBTC, DOT> {
        let wallet = Wallet::new(public_key);
        Vault {
            id,
            wallet,
            to_be_replaced_tokens: Default::default(),
            to_be_issued_tokens: Default::default(),
            issued_tokens: Default::default(),
            to_be_redeemed_tokens: Default::default(),
            backing_collateral: Default::default(),
            banned_until: None,
            status: VaultStatus::Active,
        }
    }

    pub fn is_liquidated(&self) -> bool {
        matches!(self.status, VaultStatus::Liquidated)
    }
}

pub type DefaultVault<T> = Vault<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    PolkaBTC<T>,
    DOT<T>,
>;

pub type DefaultSystemVault<T> = SystemVault<<T as frame_system::Config>::AccountId, PolkaBTC<T>>;

pub trait UpdatableVault<T: Config> {
    fn id(&self) -> T::AccountId;

    fn issued_tokens(&self) -> PolkaBTC<T>;

    fn force_issue_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn force_increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn force_increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn force_decrease_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn force_decrease_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn force_decrease_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult;

    fn decrease_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let issued_tokens = self.issued_tokens();
        ensure!(
            issued_tokens >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        self.force_decrease_issued(tokens)
    }
}

pub struct RichVault<T: Config> {
    pub(crate) data: DefaultVault<T>,
}

impl<T: Config> UpdatableVault<T> for RichVault<T> {
    fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    fn issued_tokens(&self) -> PolkaBTC<T> {
        self.data.issued_tokens
    }

    fn force_issue_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_decrease_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn force_decrease_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn force_decrease_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
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
    pub(crate) fn backed_tokens(&self) -> Result<PolkaBTC<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?)
    }

    pub fn increase_collateral(&mut self, collateral: DOT<T>) -> DispatchResult {
        self.update(|v| {
            v.backing_collateral = collateral
                .checked_add(&v.backing_collateral)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })?;
        ext::collateral::lock::<T>(&self.data.id, collateral)
    }

    pub(crate) fn force_withdraw_collateral(&mut self, collateral: DOT<T>) -> DispatchResult {
        self.update(|v| {
            v.backing_collateral = v
                .backing_collateral
                .checked_sub(&collateral)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })?;
        ext::collateral::release_collateral::<T>(&self.data.id, collateral)
    }

    pub fn withdraw_collateral(&mut self, collateral: DOT<T>) -> DispatchResult {
        let new_collateral = self
            .data
            .backing_collateral
            .checked_sub(&collateral)
            .ok_or(Error::<T>::InsufficientCollateral)?;

        let tokens = self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ensure!(
            !Module::<T>::is_collateral_below_secure_threshold(new_collateral, tokens)?,
            Error::<T>::InsufficientCollateral
        );
        self.force_withdraw_collateral(collateral)
    }

    pub fn get_collateral(&self) -> DOT<T> {
        self.data.backing_collateral
    }

    pub fn get_free_collateral(&self) -> Result<DOT<T>, DispatchError> {
        let used_collateral = self.get_used_collateral()?;
        Ok(self
            .get_collateral()
            .checked_sub(&used_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_used_collateral(&self) -> Result<DOT<T>, DispatchError> {
        let issued_tokens = self.data.issued_tokens + self.data.to_be_issued_tokens;
        let issued_tokens_in_dot = ext::oracle::btc_to_dots::<T>(issued_tokens)?;

        let raw_issued_tokens_in_dot = Module::<T>::dot_to_u128(issued_tokens_in_dot)?;

        let secure_threshold = Module::<T>::secure_collateral_threshold();

        let raw_used_collateral = secure_threshold
            .checked_mul_int(raw_issued_tokens_in_dot)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let used_collateral = Module::<T>::u128_to_dot(raw_used_collateral)?;

        Ok(used_collateral)
    }

    pub fn issuable_tokens(&self) -> Result<PolkaBTC<T>, DispatchError> {
        // if banned, there are no issuable tokens
        if self.is_banned(<frame_system::Module<T>>::block_number()) {
            return Ok(0u32.into());
        }

        let free_collateral = self.get_free_collateral()?;

        let secure_threshold = Module::<T>::secure_collateral_threshold();

        let issuable = Module::<T>::calculate_max_polkabtc_from_collateral_for_threshold(
            free_collateral,
            secure_threshold,
        )?;

        Ok(issuable)
    }

    pub fn increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let issuable_tokens = self.issuable_tokens()?;
        ensure!(issuable_tokens >= tokens, Error::<T>::ExceedingVaultLimit);
        self.force_increase_to_be_issued(tokens)
    }

    pub fn increase_to_be_replaced(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let new_to_be_replaced = self
            .data
            .to_be_replaced_tokens
            .checked_add(&tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let required_tokens = new_to_be_replaced
            .checked_add(&self.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        ensure!(
            self.data.issued_tokens >= required_tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        self.update(|v| {
            v.to_be_replaced_tokens = new_to_be_replaced;
            Ok(())
        })
    }

    pub fn decrease_to_be_replaced(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_replaced_tokens = v
                .to_be_replaced_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub fn decrease_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        ensure!(
            self.data.to_be_issued_tokens >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub fn issue_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.decrease_to_be_issued(tokens)?;
        self.force_issue_tokens(tokens)
    }

    pub fn increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let redeemable = self
            .data
            .issued_tokens
            .checked_sub(&self.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        ensure!(
            redeemable >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        self.force_increase_to_be_redeemed(tokens)
    }

    pub fn decrease_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let to_be_redeemed = self.data.to_be_redeemed_tokens;
        ensure!(
            to_be_redeemed >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    pub fn decrease_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.decrease_to_be_redeemed(tokens)?;
        self.decrease_issued(tokens)
        // Note: slashing of collateral must be called where this function is called (e.g. in Redeem)
    }

    pub fn redeem_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.decrease_tokens(tokens)
    }

    pub fn transfer(&mut self, other: &mut RichVault<T>, tokens: PolkaBTC<T>) -> DispatchResult {
        self.decrease_tokens(tokens)?;
        other.force_issue_tokens(tokens)
    }

    pub fn liquidate<V: UpdatableVault<T>>(
        &mut self,
        liquidation_vault: &mut V,
        status: VaultStatus,
    ) -> DispatchResult {
        let backing_collateral = self.data.backing_collateral;

        // we liquidate at most SECURE_THRESHOLD * backing.
        let liquidated_collateral = backing_collateral.min(self.get_used_collateral()?);

        // amount of tokens being backed
        let backing_tokens = self.backed_tokens()?;

        let to_slash = Module::<T>::calculate_collateral(
            liquidated_collateral,
            backing_tokens
                .checked_sub(&self.data.to_be_redeemed_tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?,
            backing_tokens,
        )?;

        Module::<T>::slash_collateral(
            CurrencySource::Backing(self.id()),
            CurrencySource::LiquidationVault,
            to_slash,
        )?;

        // everything above the secure threshold we release
        let to_release = backing_collateral
            .checked_sub(&liquidated_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        if !to_release.is_zero() {
            self.force_withdraw_collateral(to_release)?;
        }

        // Copy all tokens to the liquidation vault
        liquidation_vault.force_issue_tokens(self.data.issued_tokens)?;
        liquidation_vault.force_increase_to_be_issued(self.data.to_be_issued_tokens)?;
        liquidation_vault.force_increase_to_be_redeemed(self.data.to_be_redeemed_tokens)?;

        // Update vault: clear to_be_issued & issued_tokens, but don't touch to_be_redeemed
        let _ = self.update(|v| {
            v.to_be_issued_tokens = 0u32.into();
            v.issued_tokens = 0u32.into();
            v.status = status;
            Ok(())
        });

        Ok(())
    }

    pub fn ensure_not_banned(&self, height: T::BlockNumber) -> DispatchResult {
        if self.is_banned(height) {
            Err(Error::<T>::VaultBanned.into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn is_banned(&self, height: T::BlockNumber) -> bool {
        match self.data.banned_until {
            None => false,
            Some(until) => height <= until,
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

    pub fn insert_deposit_address(&mut self, btc_address: BtcAddress) {
        let _ = self.update(|v| {
            v.wallet.add_btc_address(btc_address);
            Ok(())
        });
    }

    pub fn new_deposit_address(&mut self, secure_id: H256) -> Result<BtcAddress, DispatchError> {
        let public_key = self.new_deposit_public_key(secure_id)?;
        let btc_address = BtcAddress::P2WPKHv0(public_key.to_hash());
        self.insert_deposit_address(btc_address);
        Ok(btc_address)
    }

    pub fn update_public_key(&mut self, public_key: BtcPublicKey) {
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
    pub(crate) fn redeemable_tokens(&self) -> Result<PolkaBTC<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_sub(&self.data.to_be_redeemed_tokens)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }
    pub(crate) fn backed_tokens(&self) -> Result<PolkaBTC<T>, DispatchError> {
        Ok(self
            .data
            .issued_tokens
            .checked_add(&self.data.to_be_issued_tokens)
            .ok_or(Error::<T>::ArithmeticOverflow)?)
    }
}

impl<T: Config> UpdatableVault<T> for RichSystemVault<T> {
    fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    fn issued_tokens(&self) -> PolkaBTC<T> {
        self.data.issued_tokens
    }

    fn force_issue_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_redeemed_tokens = v
                .to_be_redeemed_tokens
                .checked_add(&tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok(())
        })
    }

    fn force_decrease_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.issued_tokens = v
                .issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn force_decrease_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.update(|v| {
            v.to_be_issued_tokens = v
                .to_be_issued_tokens
                .checked_sub(&tokens)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(())
        })
    }

    fn force_decrease_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
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
