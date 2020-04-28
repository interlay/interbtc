use codec::{Decode, Encode, HasCompact};
use frame_support::{ensure, StorageMap};
use sp_core::H160;

#[cfg(test)]
use mocktopus::macros::mockable;

use x_core::Error;

use crate::types::{DOTBalance, PolkaBTCBalance, UnitResult};
use crate::{ext, Trait};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, PolkaBTCBalance: HasCompact> {
    // Account identifier of the Vault
    pub id: AccountId,
    // Number of PolkaBTC tokens pending issue
    pub to_be_issued_tokens: PolkaBTCBalance,
    // Number of issued PolkaBTC tokens
    pub issued_tokens: PolkaBTCBalance,
    // Number of PolkaBTC tokens pending redeem
    pub to_be_redeemed_tokens: PolkaBTCBalance,
    // DOT collateral locked by this Vault
    // collateral: DOTBalance,
    // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub btc_address: H160,
    // Block height until which this Vault is banned from being
    // used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
}

impl<AccountId, BlockNumber, PolkaBTCBalance: HasCompact + Default>
    Vault<AccountId, BlockNumber, PolkaBTCBalance>
{
    pub(crate) fn new(
        id: AccountId,
        btc_address: H160,
    ) -> Vault<AccountId, BlockNumber, PolkaBTCBalance> {
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

pub type DefaultVault<T> =
    Vault<<T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber, PolkaBTCBalance<T>>;

pub struct RichVault<T: Trait> {
    pub(crate) data: DefaultVault<T>,
}

#[cfg_attr(test, mockable)]
impl<T: Trait> RichVault<T> {
    pub(crate) fn new(id: T::AccountId, btc_address: H160) -> RichVault<T> {
        let vault = Vault::new(id, btc_address);
        RichVault { data: vault }
    }

    pub(crate) fn id(&self) -> T::AccountId {
        self.data.id.clone()
    }

    pub(crate) fn get_collateral(&self) -> DOTBalance<T> {
        ext::collateral::for_account::<T>(&self.data.id)
    }

    pub(crate) fn get_used_collateral(&self) -> Result<DOTBalance<T>, Error> {
        // FIXME: figure out how to multiply these two
        // and transform it to a DOTBalance<T>
        let issued_tokens = self.data.issued_tokens;
        let _used_collateral = ext::oracle::btc_to_dots::<T>(issued_tokens)?;
        Ok(Default::default())
    }

    pub(crate) fn issuable_tokens(&self) -> Result<PolkaBTCBalance<T>, Error> {
        // FIXME: figure out how to multiply these two
        // and transform it to a PolkaBTCBalance<T>
        let collateral = self.get_collateral();
        let _result = ext::oracle::dots_to_btc::<T>(collateral)?;
        Ok(1000u32.into())
    }

    pub(crate) fn increase_to_be_issued(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        let issuable_tokens = self.issuable_tokens()?;
        ensure!(issuable_tokens >= tokens, Error::ExceedingVaultLimit);
        Ok(self.update(|v| v.to_be_issued_tokens += tokens))
    }

    pub(crate) fn decrease_to_be_issued(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        ensure!(
            self.data.to_be_issued_tokens >= tokens,
            Error::InsufficientTokensCommitted
        );
        Ok(self.update(|v| v.to_be_issued_tokens -= tokens))
    }

    pub(crate) fn issue_tokens(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        self.decrease_to_be_issued(tokens)?;
        Ok(self.update(|v| v.issued_tokens += tokens))
    }

    pub(crate) fn increase_to_be_redeemed(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        let redeemable = self.redeemable_tokens();
        ensure!(redeemable >= tokens, Error::InsufficientTokensCommitted);
        Ok(self.update(|v| v.to_be_redeemed_tokens += tokens))
    }

    fn redeemable_tokens(&self) -> PolkaBTCBalance<T> {
        self.data.issued_tokens - self.data.to_be_redeemed_tokens
    }

    pub(crate) fn decrease_to_be_redeemed(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        let to_be_redeemed = self.data.to_be_redeemed_tokens;
        ensure!(to_be_redeemed >= tokens, Error::InsufficientTokensCommitted);
        Ok(self.update(|v| v.to_be_redeemed_tokens -= tokens))
    }

    pub(crate) fn decrease_issued(&mut self, tokens: PolkaBTCBalance<T>) -> UnitResult {
        let issued_tokens = self.data.issued_tokens;
        ensure!(issued_tokens >= tokens, Error::InsufficientTokensCommitted);
        Ok(self.update(|v| v.issued_tokens -= tokens))
    }

    pub(crate) fn get_free_collateral(&self) -> Result<DOTBalance<T>, Error> {
        let used_collateral = self.get_used_collateral()?;
        Ok(self.get_collateral() - used_collateral)
    }

    pub(crate) fn increase_collateral(&self, collateral: DOTBalance<T>) -> UnitResult {
        ext::collateral::lock::<T>(&self.data.id, collateral)
    }

    pub(crate) fn withdraw_collateral(&self, collateral: DOTBalance<T>) -> UnitResult {
        ext::collateral::release::<T>(&self.data.id, collateral)
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
