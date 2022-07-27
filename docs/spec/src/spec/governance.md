Governance
==========

Overview
--------

On-chain governance is useful for controlling system parameters,
authorizing trusted oracles and upgrading the core protocols. The
architecture adopted by interBTC is modelled on Polkadot with some
significant changes:

-   

    **Optimistic Governance**

    :   -   No **Council**, only public proposals from community
        -   Community can elect a **Technical Committee** to fast-track
            proposals
        -   Referenda are Super-Majority Against (Negative Turnout Bias)
            by default

-   

    **Stake-To-Vote**

    :   -   Adopted from Curve\'s governance model
        -   Users lock the native governance token
        -   Longer lockups give more voting power

![](../figures/spec/governance.jpeg){.align-center}

An important distinction is the `negative turnout bias` (Super-Majority
Against) voting threshold. This is best summarized by the
[Polkadot](https://wiki.polkadot.network/docs/learn-governance) docs:

A heavy super-majority of nay votes is required to reject at low
turnouts, but as turnout increases towards 100%, it becomes a simple
majority-carries as below.

$$\frac{\text{against}}{\sqrt{\text{electorate}}} < \frac{\text{approve}}{\sqrt{\text{turnout}}}$$

Terminology
-----------

-   **Proposals** are community-supported motions to perform
    system-level actions.
-   **Referenda** are accepted proposals undergoing voting.

Processes
---------

### Proposals

1.  Account submits public proposal with deposit (`> MinimumDeposit`)
2.  Account \"seconds\" proposal with additional deposit
3.  New referenda are started every `LaunchPeriod`
4.  Community can vote on referenda for the `VotingPeriod`
5.  Votes are tallied after `VotingPeriod` expires
6.  System update executed after `EnactmentPeriod`

### Technical Committee

1.  Community creates proposal as above
2.  TC may fast track before `LaunchPeriod`
3.  The new referendum is started immediately
4.  Community can vote on referenda for the `FastTrackVotingPeriod`

Parameters
----------

**EnactmentPeriod**

The period to wait before any approved change is enforced.

**LaunchPeriod**

The interval after which to start a new referenda from the queue.

**VotingPeriod**

The period to allow new votes for a referenda.

**MinimumDeposit**

The minimum deposit required for a proposal.

**FastTrackOrigin**

Used to fast-track a proposal before the `LaunchPeriod`.

**FastTrackVotingPeriod**

The period to allow new votes for a fast-tracked referendum.

**CancellationOrigin**

Used to cancel a proposal before it is launched.

**MaxProposals**

The maximum number of public proposals allowed in the queue.

**MaxMembers**

The maximum number of possible members in the TC.
