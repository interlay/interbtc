# Democracy Pallet

- [`democracy::Config`](https://docs.rs/pallet-democracy/latest/pallet_democracy/trait.Config.html)
- [`Call`](https://docs.rs/pallet-democracy/latest/pallet_democracy/enum.Call.html)

## Overview

The democracy pallet handles the administration of general stakeholder voting.

Proposals made by the community are added to a queue before they become a referendum.

Every launch period - a length defined in the runtime - this pallet will launch a
referendum from the proposal queue. Any token holder in the system can vote on this.

### Terminology

- **Enactment Period:** The minimum period of locking and the period between a proposal being
approved and enacted.
- **Lock Period:** A period of time after proposal enactment that the tokens of _winning_ voters
will be locked.
- **Vote:** A value that can either be in approval ("Aye") or rejection ("Nay")
  of a particular referendum.
- **Proposal:** A submission to the chain that represents an action that a proposer (either an
account or an external origin) suggests that the system adopt.
- **Referendum:** A proposal that is in the process of being voted on for
  either acceptance or rejection as a change to the system.

### Adaptive Quorum Biasing

A _referendum_ can be either simple majority-carries in which 50%+1 of the
votes decide the outcome or _adaptive quorum biased_. Adaptive quorum biasing
makes the threshold for passing or rejecting a referendum higher or lower
depending on how the referendum was originally proposed. There are two types of
adaptive quorum biasing: 1) _positive turnout bias_ makes a referendum
require a super-majority to pass that decreases as turnout increases and
2) _negative turnout bias_ makes a referendum require a super-majority to
reject that decreases as turnout increases. Another way to think about the
quorum biasing is that _positive bias_ referendums will be rejected by
default and _negative bias_ referendums get passed by default.

## Interface

### Dispatchable Functions

#### Public

These calls can be made from any externally held account capable of creating
a signed extrinsic.

Basic actions:
- `propose` - Submits a sensitive action, represented as a hash. Requires a deposit.
- `second` - Signals agreement with a proposal, moves it higher on the proposal queue, and
  requires a matching deposit to the original.
- `vote` - Votes in a referendum, either the vote is "Aye" to enact the proposal or "Nay" to
  keep the status quo.
- `unvote` - Cancel a previous vote, this must be done by the voter before the vote ends.

Administration actions that can be done to any account:
- `reap_vote` - Remove some account's expired votes.
- `unlock` - Redetermine the account's balance lock, potentially making tokens available.

Preimage actions:
- `note_preimage` - Registers the preimage for an upcoming proposal, requires
  a deposit that is returned once the proposal is enacted.
- `note_imminent_preimage` - Registers the preimage for an upcoming proposal.
  Does not require a deposit, but the proposal must be in the dispatch queue.
- `reap_preimage` - Removes the preimage for an expired proposal. Will only
  work under the condition that it's the same account that noted it and
  after the voting period, OR it's a different account after the enactment period.

#### Fast Track Origin

These calls can only be made by the `FastTrackOrigin`.

- `fast_track_proposal` - Schedules the current externally proposed proposal that
  is "majority-carries" to become a referendum immediately.
- `fast_track_referendum` - Schedules an active referendum to end in `FastTrackVotingPeriod` 
  blocks.

#### Root

- `cancel_referendum` - Removes a referendum.
- `cancel_queued` - Cancels a proposal that is queued for enactment.
- `clear_public_proposal` - Removes all public proposals.

License: Apache-2.0
