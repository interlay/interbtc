Economic Incentives {#incentives}
===================

Incentives are the core of decentralized systems. Fundamentally, actors
in decentralized systems participate in a game where each actor attempts
to maximize its utility. Designs of such decentralized systems need to
encode a mechanism that provides clear incentives for actors to adhere
to protocol rules while discouraging undesired behavior. Specifically,
actors make risk-based decisions: payoffs associated with the execution
of certain actions are compared against the risk incurred by the action.
The BTC Parachain, being an open system with multiple distinct
stakeholders, must hence offer a mechanism to assure honest
participation outweighs subversive strategies.

The overall objective of the incentive mechanism is an optimization
problem with private information in a dynamic setting. Users need to pay
fees to Vaults in return for their service. On the one hand, user fees
should be low enough to allow them to profit from having interBTC (e.g.,
if a user stands to gain from earning interest in a stablecoin system
using interBTC, then the fee for issuing interBTC should not outweigh
the interest gain).

On the other hand, fees need to be high enough to encourage Vaults to
lock their DOT in the system and operate Vault clients. This problem is
amplified as the BTC Parachain does not exist in isolation and Vaults
can choose to participate in other protocols (e.g., staking, stablecoin
issuance) as well. In the following, we outline the constraints we see,
a viable incentive model, and pointers to further research questions we
plan to solve by getting feedback from potential Vaults as well as
quantitative modeling.

Currencies
----------

The BTC-Parachain features four asset types:

-   [BTC]{.title-ref} - the backing-asset (locked on Bitcoin)
-   [interBTC]{.title-ref} - the issued cryptocurrency-backed asset (on
    Polkadot)
-   [DOT]{.title-ref} - the currency used to pay for transaction fees
-   [COL]{.title-ref} - the currencies used as collateral (e.g.,
    [DOT]{.title-ref}, [KSM]{.title-ref}, \...)

Actors: Roles, Risks, and Economics
-----------------------------------

The main question when designing the fee model for interBTC is: When are
fees paid, by whom, and how much?

![High-level overview of fee accrual in the BTC-Parachain (external
sources only).](../figures/economics/taxable-actions.png)

We can classify four groups of users, or actors, in the interBTC bridge.

Below, we provide an overview of the protocol role, the risks, and the
economics of each of the four actors. Specifically, we list the
following:

-   **Protocol role** The intended interactions of the actor with the
    bridge.
-   **Risks** An informal overview of the risks of using the bridge.
-   **Economics** An informal overview of the following economic
    factors:
    -   **Income**: revenue achieved by using the bridge. We
        differentiate between *primary* income that is achieved when the
        bridge works as intended and *secondary* income that is
        available in failure cases (e.g., misbehavior of Vaults or
        Users).
    -   **Internal costs**: costs associated directly with the
        BTC-Parachain (i.e., inflow or internal flow of funds)
    -   **External costs**: costs associated with external factors, such
        as node operation, engineering costs etc. (i.e., outflow of
        funds)
    -   **Opportunity costs**: lost revenue, if e.g. locked up
        collateral was to be used in other applications (e.g. to stake
        on the Relay chain)

### Users

-   **Protocol role** Users lock BTC with Vaults to create interBTC.
    They hold and/or use interBTC for payments, lending, or investment
    in financial products. At some point, users redeem interBTC for BTC
    by destroying the backed assets.
-   **Risks** A user gives up custody over their BTC to a Vault. The
    Vault is over-collateralized in [COL]{.title-ref}, (i.e., compared
    to the USD they will lose when taking away the user's BTC). However,
    in a market crisis with significant price drops and liquidity
    shortages, Vaults might choose to steal the BTC. Users will be
    reimbursed with [COL]{.title-ref} in that case - not the currency
    they initially started out with.
-   **Economics** A user holds BTC and has exposure to an exchange rate
    from BTC to other assets. A user's incentives are based on the
    services (and their rewards) available when issuing interBTC.
    -   **Income**
        -   Primary: Use of interBTC in external applications (outside
            the bridge)
        -   Secondary: Slashed collateral of Vaults on failed redeems
            paid in [COL]{.title-ref}, see
            `cancelRedeem`{.interpreted-text role="ref"}
        -   Secondary: Slashed collateral of Vaults on premium redeems
            paid in [COL]{.title-ref}, see
            `requestRedeem`{.interpreted-text role="ref"}
        -   Secondary: Arbitrage interBTC for [COL]{.title-ref}, see
            `liquidationRedeem`{.interpreted-text role="ref"}
    -   **Internal Cost**
        -   Issue and redeem fees paid in [interBTC]{.title-ref}, see
            `requestIssue`{.interpreted-text role="ref"} and
            `requestRedeem`{.interpreted-text role="ref"}
        -   Parachain transaction fees on every transaction with the
            system paid in [DOT]{.title-ref}
        -   Optional: Additional BTC fees on refund paid in
            [BTC]{.title-ref}, see `executeRefund`{.interpreted-text
            role="ref"}
    -   **External Costs**
        -   *None*
    -   **Opportunity Cost**
        -   Locking BTC with a Vault that could be used in another
            protocol

### Vaults

-   **Protocol role** Vaults lock up collateral in the BTC Parachain and
    hold users' BTC (i.e., receive custody). When users wish to redeem
    interBTC for BTC, Vaults release BTC to users according to the
    events received from the BTC Parachain.
-   **Risks** A Vault backs a set of interBTC with collateral. If the
    exchange rate of the [COL/BTC]{.title-ref} pair drops the Vault
    stands at risk to not be able to keep the required level of
    over-collateralization. This risk can be elevated by a shortage of
    liquidity.
-   **Economics** Vaults hold [COL]{.title-ref} and thus have exposure
    to the [COL]{.title-ref} price against [BTC]{.title-ref}. Vaults
    inherently make a bet that [COL]{.title-ref} will either stay
    constant or increase in value against BTC -- otherwise they would
    simply exchange [COL]{.title-ref} against their preferred asset(s).
    This is a simplified view of the underlying problem. We assume
    Vaults to be economically driven, i.e., following a strategy to
    maximize profits over time. While there may be altruistic actors,
    who follow protocol rules independent of the economic impact, we do
    not consider these here.
    -   **Income**
        -   Primary: Issue and redeem fees paid in
            [interBTC]{.title-ref}, see `requestIssue`{.interpreted-text
            role="ref"} and `requestRedeem`{.interpreted-text
            role="ref"}
        -   Secondary: Slashed collateral of Users on failed issues paid
            in [DOT]{.title-ref}, see `cancelIssue`{.interpreted-text
            role="ref"}
        -   Secondary: Slashed collateral of Vaults on failed replace
            paid in [COL]{.title-ref}, see
            `cancelReplace`{.interpreted-text role="ref"}
        -   Secondary: Additional BTC of Users on refund paid in
            [BTC]{.title-ref}, see `executeRefund`{.interpreted-text
            role="ref"}
    -   **Internal Cost**
        -   Parachain transaction fees on every transaction with the
            system paid in [DOT]{.title-ref}
        -   Optional: Slashed collateral on failed redeems paid in
            [COL]{.title-ref}, see `cancelRedeem`{.interpreted-text
            role="ref"}
        -   Optional: Slashed collateral on theft paid in
            [COL]{.title-ref}, see
            `relay_function_report_vault_theft`{.interpreted-text
            role="ref"}
        -   Optional: Slashed collateral on liquidation paid in
            [COL]{.title-ref}, see `liquidateVault`{.interpreted-text
            role="ref"}
    -   **External Costs**
        -   Vault client operation/maintenance costs
        -   Bitcoin full node operation/maintenance costs
    -   **Opportunity Cost**
        -   Locking [COL]{.title-ref} that could be used in another
            protocol

### Relayers

-   **Protocol role** Relayers run Bitcoin full nodes and submit block
    headers to BTC-Relay, ensuring it remains up to date with Bitcoin's
    state. They also report misbehaving Vaults who have allegedly stolen
    BTC (move BTC outside of BTC Parachain constraints).
-   **Risks** Relayers have no financial stake in the system. Their
    highest risk is that they do not get sufficient rewards for
    submitting transactions (i.e., reporting Vault theft or submitting
    BTC block headers).
-   **Economics** Relayers are exposed to similar mechanics as Vaults,
    since they also hold DOT. However, they have no direct exposure to
    the BTC/DOT exchange rate, since they (typically, at least as part
    of the BTC Parachain) do not hold BTC. As such, Staked Relayers can
    purely be motivated to earn interest on DOT, but can also have the
    option to earn interest in interBTC and optimize their holdings
    depending on the best possible return at any given time.
    -   **Income**
        -   Primary: *None*
        -   Secondary: Slashed collateral on theft paid in
            [COL]{.title-ref}, see
            `relay_function_report_vault_theft`{.interpreted-text
            role="ref"}
    -   **Internal Cost**
        -   Parachain transaction fees on every transaction with the
            system paid in [DOT]{.title-ref}
    -   **External Costs**
        -   Bitcoin full node operation/maintenance costs
        -   Parachain node operation/maintenance costs
    -   **Opportunity Cost**
        -   *None*

::: {.note}
::: {.title}
Note
:::

Operating a Vault requires access to a Bitcoin wallet. Currently, the
best solution to access a Bitcoin wallet programmatically is by using
the inbuilt wallet of the Bitcoin core full node. Hence, the Vault
client is already running a Bitcoin full node. Therefore, the Relayer
and the Vault roles are bundled together in the implementation of the
Vault/Relayer clients.
:::

### Collators

-   **Protocol role** Collators are full nodes on both a parachain and
    the Relay Chain. They collect parachain transactions and produce
    state transition proofs for the validators on the Relay Chain. They
    can also send and receive messages from other parachains using XCMP.
    More on collators can be found in the Polkadot wiki:
    <https://wiki.polkadot.network/docs/en/learn-collator#docsNav>
-   **Risks** Collators have no financial stake in the system. Hence
    running a collator has no inherent risk.
-   **Economics** Collators have to run a full node for the parachain
    incurring external costs. In return, they can receive fees.
    -   **Income**
        -   Primary: Parachain transaction fees on every transaction
            with the system paid in [DOT]{.title-ref}
    -   **Internal Cost**
        -   *None*
    -   **External Costs**
        -   Parachain full node operation/maintenance costs
    -   **Opportunity Cost**
        -   *None*

Challenges Around Economic Efficiency
-------------------------------------

To ensure security of interBTC, i.e., that users never face financial
damage, XCLAIM relies on collateral. However, in the current design,
this leads to the following economic challenges:

-   **Over-collateralization**. Vaults must lock up significantly (e.g.,
    150%) more collateral than minted interBTC to ensure security
    against exchange rate fluctuations (see
    `secureCollateralThreshold`{.interpreted-text role="ref"}).
    Dynamically modifying the secure collateral threshold could only
    marginally reduce this requirement, at a high computational
    overhead. As such, to issue 1 interBTC, one must lock up 1 BTC, as
    well as the 1.5 BTC worth of collateral (e.g. in DOT), resulting in
    a 250% collateralization.
-   **Non-deterministic Collateral Lockup**. When a Vault locks
    collateral to secure interBTC, it does not know for how long this
    collateral will remain locked. As such, it is nearly impossible to
    determine a fair price for the premium charged to the user, without
    putting either the user or the Vault at a disadvantage.
-   **Limited Chargeable Events**. The Vault only has two events during
    which it can charge fees: (1) fulfillment of and issue request
    and (2) fulfillment of a redeem request. Thereby, the fees charged
    for the redeem request must be **upper-bounded** for security
    reasons (to prevent extortion by the Vault via sky-rocketing redeem
    fees).

External Economic Risks {#externalEconomicRisks}
-----------------------

A range of external factors also have to be considered in the incentives
for the actors.

-   **Exchange rate fluctuations**. Vaults have a risk of having their
    [COL]{.title-ref} liquidated if the [COL/BTC]{.title-ref} exchange
    rate drops below the `liquidationThreshold`{.interpreted-text
    role="ref"}. In this case, the collateral is liquidated as described
    in `liquidations`{.interpreted-text role="ref"}. Liquidations
    describe that users can restore the [interBTC]{.title-ref} to
    [BTC]{.title-ref} peg by burning [interBTC]{.title-ref} for
    [COL]{.title-ref}. However, in a continuous drop of the exchange
    rate the value of [COL]{.title-ref} will fall below the value of the
    burned [interBTC]{.title-ref}. As such, the system relies on actors
    that execute fast arbitrage trades of [interBTC]{.title-ref} for
    [COL]{.title-ref}.
-   **Counterparty risk for BTC in custody**. When a user locks BTC with
    the Vault, they implicitly sell a BTC call option to the Vault. The
    Vault can, at any point in time, decide to exercise this option by
    \"stealing\" the user\'s BTC. The price for this option is
    determined by *spot\_price + punishment\_fee* (*punishment\_fee* is
    essentially the option premium). The main issue here is that we do
    not know how to price this option, because it has no expiry date -
    so this deal between the User and the Vault essentially becomes a
    BTC perpetual that can be physically exercised at any point in time
    (American-style).
-   **interBTC Liquidity Shortage**. Related to the exchange rate
    fluctuations, arbitrageurs rely on their own [interBTC]{.title-ref}
    or a place to buy [interBTC]{.title-ref} for [COL]{.title-ref} to
    execute an arbitrage trade. In a [interBTC]{.title-ref} liquidity
    shortage, simply not enough [interBTC]{.title-ref} might be
    available. In combination with a severe exchange rate drop (more
    than `liquidationThreshold`{.interpreted-text role="ref"} - 100%),
    there will be no financial incentive to restore the
    [interBTC]{.title-ref} to [BTC]{.title-ref} peg.
-   **BTC and COL Liquidity Shortage**. [interBTC]{.title-ref} is a
    \"stablecoin\" in relation to [BTC]{.title-ref}. Since owning
    [interBTC]{.title-ref} gives a claim to redeem [BTC]{.title-ref},
    the price of [interBTC]{.title-ref} to [BTC]{.title-ref} should
    remain roughly the same. However, in case [interBTC]{.title-ref}
    demand is much larger than either the [COL]{.title-ref} and/or
    [BTC]{.title-ref} supply, the price for [interBTC]{.title-ref} might
    increase much faster than [BTC]{.title-ref}. In practice, this
    should not be an issue since the collateral thresholds are computed
    based on the [BTC]{.title-ref} to [COL]{.title-ref} exchange rate
    rather than the [interBTC]{.title-ref} rates.
-   **Opportunity costs**: Each actor might decide to take an
    alternative path to receive the desired incentives. For example,
    users might pick a different platform or bridge to utilize their
    BTC. Also Vaults and Keepers might pick other protocols to earn
    interest on their DOT holdings.
