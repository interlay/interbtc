use primitives::VaultId;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type SignedFixedPoint<T> = <T as currency::Config>::SignedFixedPoint;

pub(crate) type SignedInner<T> = <T as currency::Config>::SignedInner;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, <T as staking::Config>::CurrencyId>;
