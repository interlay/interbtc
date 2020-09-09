use frame_support::traits::Currency;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, StorageMap,
};
use sp_core::H160;

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::{ext, Error, Trait};

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, PolkaBTC> {
    // Account identifier of the Vault
    pub id: AccountId,
    // Number of PolkaBTC tokens pending issue
    pub to_be_issued_tokens: PolkaBTC,
    // Number of issued PolkaBTC tokens
    pub issued_tokens: PolkaBTC,
    // Number of PolkaBTC tokens pending redeem
    pub to_be_redeemed_tokens: PolkaBTC,
    // DOT collateral locked by this Vault
    // collateral: DOT,
    // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub btc_address: H160,
    // Block height until which this Vault is banned from being
    // used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
}

impl<AccountId, BlockNumber, PolkaBTC: HasCompact + Default>
    Vault<AccountId, BlockNumber, PolkaBTC>
{
    pub(crate) fn new(id: AccountId, btc_address: H160) -> Vault<AccountId, BlockNumber, PolkaBTC> {
        Vault {
            id,
            btc_address,
            to_be_issued_tokens: Default::default(),
            issued_tokens: Default::default(),
            to_be_redeemed_tokens: Default::default(),
            banned_until: None,
        }
    }
}

pub type DefaultVault<T> = Vault<
    <T as frame_system::Trait>::AccountId,
    <T as frame_system::Trait>::BlockNumber,
    PolkaBTC<T>,
>;

pub(crate) struct RichVault<T: Trait> {
    pub(crate) data: DefaultVault<T>,
}

#[cfg_attr(test, mockable)]
impl<T: Trait> RichVault<T> {
    pub fn new(id: T::AccountId, btc_address: H160) -> RichVault<T> {
        let vault = Vault::new(id, btc_address);
        RichVault { data: vault }
    }

    pub fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    pub fn increase_collateral(&self, collateral: DOT<T>) -> DispatchResult {
        ext::collateral::lock::<T>(&self.data.id, collateral)
    }

    pub fn withdraw_collateral(&self, collateral: DOT<T>) -> DispatchResult {
        let current_collateral = ext::collateral::for_account::<T>(&self.data.id);

        let raw_current_collateral = crate::Module::<T>::dot_to_u128(current_collateral)?;
        let raw_collateral = crate::Module::<T>::dot_to_u128(collateral)?;
        let raw_new_collateral = raw_current_collateral
            .checked_sub(raw_collateral)
            .unwrap_or(0);

        let new_collateral = crate::Module::<T>::u128_to_dot(raw_new_collateral)?;

        ensure!(
            !crate::Module::<T>::_is_collateral_below_secure_threshold(
                new_collateral,
                self.data.issued_tokens
            )?,
            Error::<T>::InsufficientCollateral
        );

        ext::collateral::release::<T>(&self.data.id, collateral)
    }

    pub fn get_collateral(&self) -> DOT<T> {
        ext::collateral::for_account::<T>(&self.data.id)
    }

    pub fn get_free_collateral(&self) -> Result<DOT<T>, DispatchError> {
        let used_collateral = self.get_used_collateral()?;
        Ok(self.get_collateral() - used_collateral)
    }

    pub fn get_used_collateral(&self) -> Result<DOT<T>, DispatchError> {
        let issued_tokens = self.data.issued_tokens + self.data.to_be_issued_tokens;
        let issued_tokens_in_dot = ext::oracle::btc_to_dots::<T>(issued_tokens)?;

        let raw_issued_tokens_in_dot = crate::Module::<T>::dot_to_u128(issued_tokens_in_dot)?;

        let secure_threshold = crate::Module::<T>::_get_secure_collateral_threshold();

        let raw_used_collateral = raw_issued_tokens_in_dot
            .checked_mul(secure_threshold)
            .ok_or(Error::<T>::RuntimeError)?;

        let used_collateral = crate::Module::<T>::u128_to_dot(raw_used_collateral)?;

        Ok(used_collateral)
    }

    pub fn issuable_tokens(&self) -> Result<PolkaBTC<T>, DispatchError> {
        let free_collateral = self.get_free_collateral()?;

        let secure_threshold = crate::Module::<T>::_get_secure_collateral_threshold();

        let issuable = crate::Module::<T>::calculate_max_polkabtc_from_collateral_for_threshold(
            free_collateral,
            secure_threshold,
        )?;

        Ok(issuable)
    }

    pub fn increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let issuable_tokens = self.issuable_tokens()?;
        ensure!(issuable_tokens >= tokens, Error::<T>::ExceedingVaultLimit);
        Ok(self.force_increase_to_be_issued(tokens))
    }

    fn force_increase_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> () {
        self.update(|v| v.to_be_issued_tokens += tokens);
    }

    pub fn decrease_to_be_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        ensure!(
            self.data.to_be_issued_tokens >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        Ok(self.update(|v| v.to_be_issued_tokens -= tokens))
    }

    pub fn issue_tokens(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        self.decrease_to_be_issued(tokens)?;
        Ok(self.force_issue_tokens(tokens))
    }

    fn force_issue_tokens(&mut self, tokens: PolkaBTC<T>) -> () {
        self.update(|v| v.issued_tokens += tokens)
    }

    pub fn decrease_issued(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let issued_tokens = self.data.issued_tokens;
        ensure!(
            issued_tokens >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        Ok(self.update(|v| v.issued_tokens -= tokens))
    }

    pub fn increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let redeemable = self.data.issued_tokens - self.data.to_be_redeemed_tokens;
        ensure!(
            redeemable >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        Ok(self.force_increase_to_be_redeemed(tokens))
    }

    fn force_increase_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> () {
        self.update(|v| v.to_be_redeemed_tokens += tokens);
    }

    pub fn decrease_to_be_redeemed(&mut self, tokens: PolkaBTC<T>) -> DispatchResult {
        let to_be_redeemed = self.data.to_be_redeemed_tokens;
        ensure!(
            to_be_redeemed >= tokens,
            Error::<T>::InsufficientTokensCommitted
        );
        Ok(self.update(|v| v.to_be_redeemed_tokens -= tokens))
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
        Ok(other.force_issue_tokens(tokens))
    }

    pub fn liquidate(&self, liquidation_vault: &mut RichVault<T>) -> DispatchResult {
        ext::collateral::slash::<T>(&self.id(), &liquidation_vault.id(), self.get_collateral())?;
        liquidation_vault.force_issue_tokens(self.data.issued_tokens);
        liquidation_vault.force_increase_to_be_issued(self.data.to_be_issued_tokens);
        liquidation_vault.force_increase_to_be_redeemed(self.data.to_be_redeemed_tokens);
        <crate::Vaults<T>>::remove(&self.id());
        Ok(())
    }

    pub fn ensure_not_banned(&self, height: T::BlockNumber) -> DispatchResult {
        let is_banned = match self.data.banned_until {
            None => false,
            Some(until) => height <= until,
        };

        if is_banned {
            Err(Error::<T>::VaultBanned.into())
        } else {
            Ok(())
        }
    }

    pub fn ban_until(&mut self, height: T::BlockNumber) {
        self.update(|v| v.banned_until = Some(height));
    }

    fn update<F>(&mut self, func: F) -> ()
    where
        F: Fn(&mut DefaultVault<T>) -> (),
    {
        func(&mut self.data);
        <crate::Vaults<T>>::mutate(&self.data.id, func);
    }
}

impl<T: Trait> From<&RichVault<T>> for DefaultVault<T> {
    fn from(rv: &RichVault<T>) -> DefaultVault<T> {
        rv.data.clone()
    }
}

impl<T: Trait> From<DefaultVault<T>> for RichVault<T> {
    fn from(vault: DefaultVault<T>) -> RichVault<T> {
        RichVault { data: vault }
    }
}
