interBTC at a Glance
====================

The *interBTC bridge* connects the Polkadot ecosystem with Bitcoin. It
allows the creation of *interBTC*, a fungible token that represents
Bitcoin in the Polkadot ecosystem. interBTC is backed by Bitcoin 1:1 and
allows redeeming of the equivalent amount of Bitcoins by relying on a
collateralized third-party.

![The interBTC bridge allows the creation of collateralized 1:1
Bitcoin-backed tokens in Polkadot. These tokens can be transferred and
traded within the Polkadot ecosystem.](../figures/intro/overview.png)

Functionality
-------------

On a high-level, the BTC Parachain enables the issuing and redeeming of
interBTC. The *issue process* allows a user to lock Bitcoin on the
Bitcoin chain and, in return, issue interBTC on the BTC Parachain.
Consequently, the *redeem process* allows a user to burn interBTC on the
BTC Parachain and redeem previously locked Bitcoins on Bitcoin. Users
can trade interBTC on the BTC Parachain and, through the Relay Chain, in
other Parachains as well. The issue and redeem process can be executed
by different users. Typically, this process is augmented by a
collateralized realized third-party, a so-called *vault*.

![The BTC Parachain includes a protocol to issue interBTC by locking
Bitcoin and a protocol to redeem Bitcoin by burning interBTC
tokens.](../figures/intro/Overview-Func.png)

Components
----------

The BTC Parachain makes use of two main components to achieve issuing
and redeeming of interBTC:

-   **XCLAIM(BTC,DOT)**: The XCLAIM(BTC,DOT) component implements four
    protocols including issue, transfer, redeem, and replace. It
    maintains the interBTC tokens, i.e. who owns how many tokens and
    manages the vaults as well as the collateral in the system.
-   **BTC-Relay**: The BTC-Relay component is used to verify that
    certain transactions have happened on the Bitcoin blockchain. For
    example, when a user issues a new interBTC an equivalent amount of
    Bitcoins needs to be locked on the Bitcoin chain. The user can prove
    this to the interBTC component by verifying his transaction in the
    BTC-Relay component.

The figure below describes the relationships between the components in a
high level. Please note that we use a simplified model here, where users
are the ones augmenting the issue and redeem process. In practice, this
is executed by the collateralized vaults.

![The BTC Parachain consists of two logically different components. The
XCLAIM(BTC,DOT) component (in green) maintains the accounts that own
interBTC tokens. The BTC-Relay (blue) is repsonible for verifying the
Bitcoin state to verify transactions. Users (in purple) are able to
create new interBTC by locking BTC on the Bitcoin chain and redeeming
BTC by burning interBTC. Also, users can trade interBTC on the BTC
Parachain and in the wider Polkadot
ecosystem.](../figures/intro/Overview-Components.png)
