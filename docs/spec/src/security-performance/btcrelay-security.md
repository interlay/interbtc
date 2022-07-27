BTC-Relay Security Analysis {#btcrelay_security}
===========================

This section provides an overview of security considerations related to
BTC-Relay. We refer the reader to [this paper (Section
7)](https://eprint.iacr.org/2018/643.pdf) for more details.

Security Parameter *k* {#security_parameter_k}
----------------------

Blockchains using Nakamoto consensus as underlying agreement protocol
(i.e., leveraging PoW for random leader election in a dynamically
changing set of consensus participants) exhibit so called *stabilizing
consensus*. Specifically, finality of transactions included in the
blockchain converges with a *security parameter k*, measured in
confirmations (i.e., blocks mined on top of a block containing the
observed transaction). That is, the probability of a transaction being
reverted in a blockchain reorganization decreases exponentially in *k*.
We refer the reader to [this
paper](https://eprint.iacr.org/2018/400.pdf) for more details on
Nakamoto consensus.

In Bitcoin, this security parameter is often set to *k = 6*, i.e.,
transactions are considered \"final\" after 6 blocks have been mined on
top. However, there is *no mathematical reasoning* behind this, nor is
there a proof that 6 confirmations are sufficient.

In fact,
[research](https://www.cs.huji.ac.il/~yoni_sompo/pubs/16/security_model.pdf)
has shown that when estimating the necessary confirmations before
accepting a transaction, the *transaction value* itself must also be
considered: the higher the value, the more confirmations are necessary
to maintain the same level of security. However, [recent
analysis](https://medium.com/@dionyziz/summa-proofs-are-not-composable-57b87825f428)
suggests that it is insufficient to consider the value of a single
transaction - instead, to estimate the necessary *k* one must study the
value of the entire block. The existence of bribing attacks, which can
even be executed cross-chain, makes the situation worse: in theory, it
is [impossible to estimate k
reliably](https://www.alexeizamyatin.me/files/Pay-to-Win_slides.pdf), as
there can always be a large transaction that is being attacked by a
reorg in an older block.

**What does this mean for BTC-Relay?**

BTC-Relay does not specify a recommended value for *k*. This task lies
with the applications which interact with the relay. BTC-Relay itself
only *mirrors* the state of Bitcoin to Polkadot, including all forks and
failures which may occur.

Liveness Failures
-----------------

The correct operation of BTC-Relay relies on receiving a steady stream
of Bitcoin block headers as input. A high delay between block generation
in Bitcoin and submission to BTC-Relay yields the system susceptible to
attacks: an adversary can attempt to *poison* the relay by submitting a
fork, even if the fork was not submitted to Bitcoin itself (see
`relay-poisoning`{.interpreted-text role="ref"} below).

While by design, any user can submit Bitcoin block headers to BTC-Relay,
it is recommended to introduce an explicit set of participants for this
task. These can be *Staked Relayers*, which already run Bitcoin full
nodes for validation purposes, or *Vaults* which are used for the
creation of Bitcoin-backed assets in the interBTC component.

Safety Failures
---------------

### 51% Attack on Bitcoin

One of the major questions that arises in cross-chain communication is:
what to do if one of the interlinked chains fails?

In the case of BTC-Relay, a major chain reorganization in Bitcoin would
be accepted, if the new chain exceeds the tracked `Chains` in BTC-Relay.
If the length of the fork exceeds the security parameter *k* relied upon
by applications using BTC-Relay, this can have sever impacts, beyond
that of users loosing BTC.

However, as BTC-Relay acts only as mirror of the Bitcoin blockchain, the
only possible mitigation of a 51% attack on Bitcoin **halting
BTC-Relay** via manual intervention of *Staked Relayers* or the
*Governance Mechanism*. See **Failure Handling** for more details on
BTC-Relay failure modes and recovery procedures.

::: {.todo}
Add reference to Failure Handling spec, once deployed.
:::

A major challenge thereby is to ensure the potential financial loss of
*Staked Relayers* and/or participants of the *Governance Mechanism*
exceeds the potential gains from colluding with an adversary on Bitcoin.

### Relay Poisoning

BTC-Relay poisoning is a more subtle way of interfering with correct
operation of the system: an adversary submits a Bitcoin fork to
BTC-Relay, but does not broadcast it to the actual Bitcoin network. If
Liveness of BTC-Relay is breached, e.g. *Staked Relayers* are
unavailable, BTC-Relay can be tricked into accepting an alternate
`Chains` than actually maintained in Bitcoin.

However, as long as a single honest participant is online and capable of
submitting Bitcoin block headers from the Bitcoin main chain to
BTC-Relay within *k* blocks, poisoning attacks can be mitigated.

### Replay Attacks {#replace-attacks}

Since BTC-Relay does not store Bitcoin transactions, nor can it be aware
of all possible applications using `verifyTransactionInclusion`,
duplicate submission of transaction inclusion proofs **cannot be easily
detected** by BTC-Relay.

As such, it lies in the responsibility of each application interacting
with BTC-Relay to introduce necessary replay protection mechanisms (e.g.
nonces stored in OP\_RETURN outputs of verified transactions) and to
check the latter using the `parser`{.interpreted-text role="ref"}
component of BTC-Relay.

Hard and Soft forks
-------------------

Permanent chain splits or *hard forks* occur where consensus rules are
\"loosened\" or new conflicting rules are introduced. As a result,
multiple instances of the same blockchain are created, e.g. as in the
case of Bitcoin and Bitcoin Cash.

BTC-Relay by default will follow the old consensus rules, and must be
updated accordingly if it is to follow the new version of the system.

Thereby, is it for the *Governance Mechanism* to determine (i) whether
an update will be executed and (ii) if two parallel blockchains result
from the hard fork, whether an additional new instance of BTC-Relay is
to be deployed (and how).

Note: to differentiate between the two resulting chains after a hard
fork, replay protection is necessary for secure operation. While
typically accounted for by the developers of the verified blockchain,
the absence of replay protection can lead to undesirable behavior.
Specifically, payments made on one fork may be accepted as valid on the
other as well - and propagated to BTC-Relay. To this end, *if a fork
lacks replay protection*, **halting of the relay** may be necessary
until the matter is resolved.
