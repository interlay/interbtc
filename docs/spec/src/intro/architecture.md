Architecture
============

interBTC consists of four different actors and eight modules. The
component further uses two additional modules, the BTC-Relay component
and the Parachain Governance mechanism.

Actors
------

There are four main participant roles in the system. A high-level
overview of all modules and actors, as well as interactions between
them, is provided in `high-level`{.interpreted-text role="numref"}
below.

-   **Vaults**: Vaults are collateralized intermediaries that are active
    on both the backing blockchain (Bitcoin) and the issuing blockchain
    to provide collateral in DOT. They receive and hold BTC from users
    who wish to create interBTC tokens. When a user destroys interBTC
    tokens, a vault releases the corresponding amount of BTC to the
    user\'s BTC address. Vaults interact with the following modules
    directly: `vault-registry`{.interpreted-text role="ref"},
    `redeem-protocol`{.interpreted-text role="ref"}, and
    `replace-protocol`{.interpreted-text role="ref"}.
    -   **Reporting**: Monitors that other Vaults do not move locked BTC
        on Bitcoin without prior authorization by the BTC Parachain
        (i.e., through one of the `redeem-protocol`{.interpreted-text
        role="ref"}, `replace-protocol`{.interpreted-text role="ref"} or
        `refund-protocol`{.interpreted-text role="ref"} protocols).
    -   **Relaying**: Submits block headers published on Bitcoin to the
        `btc_relay`{.interpreted-text role="ref"}.
-   **Users**: Users interact with the BTC Parachain to create, use
    (trade/transfer/\...), and redeem Bitcoin-backed interBTC tokens.
    Since the different protocol phases can be executed by different
    users, we introduce the following *sub-roles*:
    -   **Requester**: A user that locks BTC with a vault on Bitcoin and
        issues interBTC on the BTC Parachain. Interacts with the
        `issue-protocol`{.interpreted-text role="ref"} module.
    -   **Sender** and **Receiver**: A user (Sender) that sends interBTC
        to another user (Receiver) on the BTC Parachain. Interacts with
        the `currency`{.interpreted-text role="ref"} module.
    -   **Redeemer**: A user that destroys interBTC on the BTC Parachain
        to receive the corresponding amount of BTC on the Bitcoin
        blockchain from a Vault. Interacts with the
        `redeem-protocol`{.interpreted-text role="ref"} module.
-   **Governance Mechanism**: The Parachain Governance Mechanism
    monitors the correct operation of the BTC Parachain. Interacts with
    the `security`{.interpreted-text role="ref"} module and can manually
    update the parameterization of all components in the BTC Parachain.

Modules
-------

The eight modules in interBTC plus the BTC-Relay and Governance
Mechanism interact with each other, but all have distinct logical
functionalities. The figure below shows them.

The specification clearly separates these modules to ensure that each
module can be implemented, tested, and verified in isolation. The
specification follows the principle of abstracting the internal
implementation away and providing a clear interface. This should allow
optimization and improvements of a module with minimal impact on other
modules.

::: {#high-level}
![High level overview of the BTC Parachain. interBTC consists of seven
modules. The Oracle module stores the exchange rates based on the input
of centralized and decentralized exchanges. The Treasury module
maintains the ownership of interBTC, the VaultRegistry module stores
information about the current Vaults in the system, and the Issue,
Redeem and Replace modules expose functions and maintain data related to
the respective sub protocols. The StabilizedCollateral modules handles
vault collateralization, stabilization against exchange rate
fluctuations and automatic liquidation. BTC-Relay tracks the Bitcoin
main chain and verifies transaction inclusion. The Parachain Governance
maintains correct operation of the BTC Parachain and intervenes / halts
operation if necessary.](../figures/intro/architecture.png)
:::

### BTC-Relay

BTC-Relay is a key component of the BTC Parachain on Polkadot. Its main
task is to allow the Parachain to verify the state of Bitcoin and react
to transactions and events. Specifically, BTC-Relay acts as a [Bitcoin
SPV/light
client](https://en.bitcoin.it/wiki/Scalability#Simplified_payment_verification)
on Polkadot, storing only Bitcoin block headers and allowing users to
verify transaction inclusion proofs. Further, it is able to handle forks
and follows the chain with the most accumulated Proof-of-Work.

The correct operation of BTC-Relay is crucial: should BTC-Relay cease to
operate, the bridge between Polkadot and Bitcoin is interrupted.

![BTC-Relay (highlighted in blue) is a key component of the BTC
Parachain: it is necessary to verify and keep track of the state of
Bitcoin.](../figures/intro/interBTC-btcrelay.png)

Below, we provide an overview of its components, as well as relevant
actors - offering references to the full specification contained in the
rest of this document.

![Overview of the BTC-Relay architecture. Bitcoin block headers are
submitted to the Verification Component, which interacts with the Utils,
Parser and Failure Handling components, as well as the Parachain
Storage.](../figures/intro/btcrelay-architecture.png)

### Oracle

The Oracle module maintains the exchange rate value between the asset
that is used to collateralize Vaults (e.g. DOT) and the wrapped asset
(interBTC). Governance authorizes trusted third parties to feed the
current exchange rates into the system for a nominal fee.

### Treasury

The Treasury module maintains the ownership and balance of interBTC
token holders. It allows respective owners of interBTC to send their
tokens to other entities and to query their balance. Further, it tracks
the total supply of tokens.

### Vault Registry

The VaultRegistry module manages the Vaults in the system.It allows
Managing the list of active Vaults in the system and the necessary data
(e.g. BTC addresses) to execute the Issue, Redeem, and Replace
protocols.

This module also handles the collateralization rates of Vaults and
reacts to exchange rate fluctuations. Specifically, it:

-   Stores how much collateral each vault provided and how much of that
    collateral is allocated to interBTC.
-   Triggers, as a last resort, automatic liquidation if a vault falls
    below the minimum collateralization rate.

### Collateral

The Collateral module is the central storage for any collateral that is
collected in any other module. It is allows for three simple operations:
locking collateral by a party, releasing collateral back to the original
party that locked this collateral, and last, slashing collateral where
the collateral is relocated to a party other than the one that locked
the collateral.

### Issue

The Issue module handles the issuing process for interBTC tokens. It
tracks issue requests by users, handles the collateral provided by users
as griefing protection and exposes functionality for users to prove
correct locking on BTC with Vaults (interacting with the endpoints in
BTC-Relay).

### Redeem

The Redeem module handles the redeem process for interBTC tokens. It
tracks redeem requests by users, exposes functionality for Vaults to
prove correct release of BTC to users (interacting with the endpoints in
BTC-Relay), and handles the Vault\'s collateral in case of success
(free) and failure (slash).

### Replace

The Replace module handles the replace process for Vaults. It tracks
replace requests by existing Vaults, exposes functionality for
to-be-replaced Vaults to prove correct transfer of locked BTC to new
vault candidates (interacting with the endpoints in BTC-Relay), and
handles the collateral provided by participating Vaults as griefing
protection.

### Security

The Security module is the kernel of the BTC Parachain. It is imported
by most modules to ensure that the chain is running.

### Governance Mechanism

The Governance Mechanism handles correct operation of the BTC Parachain.

Interactions
------------

### Dependency Graph

We provide a dependency graph of the different pallets in
`fig-dependency-graph`{.interpreted-text role="numref"}. Note that for
clarity, dependencies that are already implied by transitivity are not
displayed. That is, if `a -> b`, `b -> c` and `a -> b`, we do not show a
dependency `a -> c` even when it is an explicit dependency in the
implementation.

::: {#fig-dependency-graph}
![Pallet dependency graph](../figures/intro/pallet-dependencies.png)
:::

### External Interactions

We provide an overview in `fig-dispatchable-functions`{.interpreted-text
role="numref"} of the main ways that different actors interact with the
parachain. Note that we only include the function calls that have side
effects, i.e., that write to storage. Also, some calls that are not
central to the main protocol are omitted to keep the overview clear. The
pallets are displayed in the center column, while the various actors
surround it in yellow.

::: {#fig-dispatchable-functions}
![Overview of interactions of different actors with the
parachain.](../figures/intro/dispatchable-functions.png)
:::
