use crate::Config;
use codec::{Decode, Encode, MaxEncodedLen};
use currency::CurrencyId;
use primitives::VaultId;
use scale_info::TypeInfo;
use sp_arithmetic::FixedPointNumber;

pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type UnsignedInner<T> = <<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
    /// Use sovereign account ID.
    V1,
}
