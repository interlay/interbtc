use codec::{Decode, Encode, HasCompact};
use sp_core::H160;

#[cfg(test)]
use mocktopus::macros::mockable;

use x_core::Error;

use crate::types::{DOTBalance, PolkaBTCBalance};
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

    pub(crate) fn get_collateral(&self) -> DOTBalance<T> {
        ext::collateral::for_account::<T>(&self.data.id)
    }

    pub(crate) fn get_used_collateral(&self) -> Result<DOTBalance<T>, Error> {
        // TODO: figure out how to multiply these two
        // and transform it to a DOTBalance<T>
        let _issued_tokens = self.data.issued_tokens;
        let _exchange_rate = ext::oracle::get_exchange_rate::<T>()?;
        Ok(Default::default())
    }

    pub(crate) fn issuable_tokens(&self) -> Result<PolkaBTCBalance<T>, Error> {
        // TODO: figure out how to multiply these two
        // and transform it to a PolkaBTCBalance<T>
        let _collateral = self.get_collateral();
        let _exchange_rate = ext::oracle::get_exchange_rate::<T>()?;
        Ok(1000u32.into())
    }

    pub(crate) fn get_free_collateral(&self) -> Result<DOTBalance<T>, Error> {
        let used_collateral = self.get_used_collateral()?;
        Ok(self.get_collateral() - used_collateral)
    }

    pub(crate) fn increase_collateral(&self, collateral: DOTBalance<T>) -> Result<(), Error> {
        ext::collateral::lock::<T>(&self.data.id, collateral)
    }

    pub(crate) fn withdraw_collateral(&self, collateral: DOTBalance<T>) -> Result<(), Error> {
        ext::collateral::release::<T>(&self.data.id, collateral)
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
