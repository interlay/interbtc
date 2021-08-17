use codec::{Decode, Encode};
use sp_std::fmt::Debug;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Wrapped<T> = BalanceOf<T>;

/// Bonded participant which can suggest and vote on proposals.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct StakedRelayer<Balance, BlockNumber> {
    // total stake for this participant
    pub stake: Balance,
    // the height at which the participant bonded
    pub height: BlockNumber,
}
