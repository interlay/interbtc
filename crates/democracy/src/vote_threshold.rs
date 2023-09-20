//! Voting thresholds.

use crate::Tally;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{IntegerSquareRoot, Zero};
use sp_std::ops::{Add, Div, Mul, Rem};

/// A means of determining if a vote is past pass threshold.
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    sp_runtime::RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum VoteThreshold {
    /// A supermajority of approvals is needed to pass this vote.
    SuperMajorityApprove,
    /// A supermajority of rejects is needed to fail this vote.
    SuperMajorityAgainst,
    /// A simple majority of approvals is needed to pass this vote.
    SimpleMajority,
}

pub trait Approved<Balance> {
    /// Given a `tally` of votes and a total size of `electorate`, this returns `true` if the
    /// overall outcome is in favor of approval according to `self`'s threshold method.
    fn approved(&self, tally: Tally<Balance>, electorate: Balance) -> bool;
}

/// Return `true` iff `n1 / d1 < n2 / d2`. `d1` and `d2` may not be zero.
fn compare_rationals<T: Zero + Mul<T, Output = T> + Div<T, Output = T> + Rem<T, Output = T> + Ord + Copy>(
    mut n1: T,
    mut d1: T,
    mut n2: T,
    mut d2: T,
) -> bool {
    // Uses a continued fractional representation for a non-overflowing compare.
    // Detailed at https://janmr.com/blog/2014/05/comparing-rational-numbers-without-overflow/.
    loop {
        let q1 = n1 / d1;
        let q2 = n2 / d2;
        if q1 < q2 {
            return true;
        }
        if q2 < q1 {
            return false;
        }
        let r1 = n1 % d1;
        let r2 = n2 % d2;
        if r2.is_zero() {
            return false;
        }
        if r1.is_zero() {
            return true;
        }
        n1 = d2;
        n2 = d1;
        d1 = r2;
        d2 = r1;
    }
}

impl<
        Balance: IntegerSquareRoot
            + Zero
            + Ord
            + Add<Balance, Output = Balance>
            + Mul<Balance, Output = Balance>
            + Div<Balance, Output = Balance>
            + Rem<Balance, Output = Balance>
            + Copy,
    > Approved<Balance> for VoteThreshold
{
    fn approved(&self, tally: Tally<Balance>, electorate: Balance) -> bool {
        let sqrt_voters = tally.turnout.integer_sqrt();
        // Since voting power decays, in rare cases the electorate can be less (even zero) than the sum of votes.
        let sqrt_electorate = electorate.max(tally.turnout).integer_sqrt();
        if sqrt_voters.is_zero() || sqrt_electorate.is_zero() {
            return false;
        }
        match *self {
            VoteThreshold::SuperMajorityApprove => {
                compare_rationals(tally.nays, sqrt_voters, tally.ayes, sqrt_electorate)
            }
            VoteThreshold::SuperMajorityAgainst => {
                compare_rationals(tally.nays, sqrt_electorate, tally.ayes, sqrt_voters)
            }
            VoteThreshold::SimpleMajority => tally.ayes > tally.nays,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_work() {
        assert!(!VoteThreshold::SuperMajorityApprove.approved(
            Tally {
                ayes: 60,
                nays: 50,
                turnout: 110
            },
            210
        ));
        assert!(VoteThreshold::SuperMajorityApprove.approved(
            Tally {
                ayes: 100,
                nays: 50,
                turnout: 150
            },
            210
        ));
    }
}
