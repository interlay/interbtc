use codec::{Decode, Encode, MaxEncodedLen};
use currency::CurrencyId;
use primitives::{VaultCurrencyPair, VaultId};
use scale_info::TypeInfo;

pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub(crate) type DefaultVaultCurrencyPair<T> = VaultCurrencyPair<CurrencyId<T>>;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
}
