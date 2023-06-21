Cryptocurrency-backed Assets
============================

Building trustless cross-blockchain trading protocols is challenging.
Centralized exchanges thus remain the preferred route to executing
transfers across blockchains. However, these services require trust and
therefore undermine the very nature of the blockchains on which they
operate. To overcome this, several decentralized exchanges have recently
emerged which offer support for *commit-reveal* atomic cross-chain swaps
(ACCS).

Commit-reveal ACCS, most notably based on
[HTCLs](https://en.bitcoin.it/wiki/Hashed_Timelock_Contracts), enable
the trustless exchange of cryptocurrencies across blockchains. To this
date, this is the only mechanism to have been deployed in production.
However, commit-reveal ACCS face numerous challenges:

-   **Long waiting times:** Each commit-reveal ACCS requires multiple
    transactions to occur on all involved blockchains (commitments and
    revealing of secrets).
-   **High costs:** Publishing multiple transaction per swap results in
    high fees to maintain such a system.
-   **Strict online requirements:** Both parties must be online during
    the ACCS. Otherwise, the trade fails or, in the worst case, *loss of
    funds is possible*.
-   **Out-of-band channels:** Secure operation requires users to
    exchange additional data *off-chain* (revocation commitments).
-   **Race conditions:** Commit-reveal ACCS use time-locks to ensure
    security. Synchronizing time across blockchains, however, is
    challenging and opens up risks to race conditions.
-   **Inefficiency:** Finally, commit-reveal ACCS are *one-time*. That
    is, all of the above challenges are faced with each and every trade.

Commit-reveal ACCS have been around since 2012. The practical challenges
explain their limited use in practice.

Cryptocurrency-back Assets (CbA)
--------------------------------

The idea of CbAs is that an asset is locked on a *backing blockchain*
and issued 1:1 on an *issuing blockchain*. CbA that minimize trust in a
third-party are based on the [XCLAIM protocol](https://www.xclaim.io/).
The third parties in XCLAIM are called *vaults* and are required to lock
collateral as an insurance against misbehaviour.

XCLAIM introduces three protocols to achieve decentralized, transparent,
consistent, atomic, and censorship resistant cross-blockchain swaps:

-   **Issue**: Create Bitcoin-backed tokens, so-called *interBTC* on the
    BTC Parachain.
-   **Transfer**: Transfer interBTC to others within the Polkadot
    ecosystem.
-   **Redeem**: Burn Bitcoin-backed tokens on the BTC Parachain and
    receive 1:1 of the amount of Bitcoin in return.

The basic intuition of the protocol is as below:

![The issue, transfer/swap, and redeem protocols in
XCLAIM.](../figures/intro/xclaim-process.png)

Design Principles
-----------------

XCLAIM guarantees that Bitcoin-backed tokens can be redeemed for the
corresponding amount of Bitcoin, or the equivalent economic value in
DOT. Thereby, XCLAIM overcomes the limitations of centralized approaches
through three primary techniques:

-   **Secure audit logs**: Logs are constructed to record actions of all
    users both on Bitcoin and the BTC Parachain.
-   **Transaction inclusion proofs**: Chain relays are used to prove
    correct behavior on Bitcoin to the BTC Parachain.
-   **Proof-or-Punishment**: Instead of relying on timely fraud proofs
    (reactive), XCLAIM requires correct behavior to be proven
    proactively.
-   **Over-collateralization**: Non-trusted intermediaries, i.e. vaults,
    are bound by collateral, with mechanisms in place to mitigate
    exchange rate fluctuations.

Recommended Background Reading
------------------------------

-   **XCLAIM: Trustless, Interoperable, Cryptocurrency-backed Assets**.
    *IEEE Security and Privacy (S&P).* Zamyatin, A., Harz, D., Lind, J.,
    Panayiotou, P., Gervais, A., & Knottenbelt, W. (2019).
-   **Enabling Blockchain Innovations with Pegged Sidechains**. *Back,
    A., Corallo, M., Dashjr, L., Friedenbach, M., Maxwell, G., Miller,
    A., Poelstra A., Timon J., & Wuille, P*. (2014)
-   **SoK: Communication Across Distributed Ledgers**. *Cryptology
    ePrint Archiv, Report 2019/1128*. Zamyatin A, Al-Bassam M, Zindros
    D, Kokoris-Kogias E, Moreno-Sanchez P, Kiayias A, Knottenbelt
    WJ. (2019)
-   **Proof-of-Work Sidechains**. *Workshop on Trusted Smart Contracts,
    Financial Cryptography* Kiayias, A., & Zindros, D. (2018)
