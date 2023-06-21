Collateral {#collateral-module}
==========

Overview
--------

There are two different kinds of collateral in use in the bridge. The
first is the backing collateral that vaults use as insurance to issued
wrapped tokens. Multiple backing collaterals are supported, see
`vault_registry_overview_multi_collateral`{.interpreted-text
role="ref"}, but similarly to MakerDAO, each vault uses a single
currency. If vault operators want to use multiple currencies, they have
to register multiple vaults. It is possible to use [key
derivation](https://substrate.dev/docs/en/knowledgebase/integrate/subkey#hd-key-derivation)
to run multiple vaults using a single mnemonic. When a vault is
registered, they have to explicitly choose the used currency. In
contrast, when interacting with vaults, the used collateral is implicit.
For example, when a vault fails to execute a redeem request, the user
will receive some amount of the vault\'s backing collateral. As such,
the user might want to select a vault that uses their preferred
currency.

The second type of collateral is griefing collateral. The currency used
for this type of collateral is fixed and depends on the used network.
This is the currency that is also used to pay transaction fees. For
example, in Kusama transaction fees are by default paid in KINT and on
Polkadot transaction fees are paid in INTR.

While collateral management is logically distinct from treasury
management, they are both implemented using the same
`currency`{.interpreted-text role="ref"} pallet. This pallet is used to
(i) lock, (ii) release, and (iii) slash collateral of either users or
vaults. It can only be accessed by other modules and not directly
through external transactions.

### Step-by-Step

The protocol has three different \"sub-protocols\".

-   **Lock**: Store a certain amount of collateral from a single entity
    (user or vault).
-   **Unlock**: Transfer a certain amount of collateral back to the
    entity that paid it.
-   **Slash**: Transfer a certain amount of locked collateral to a party
    that was damaged by the actions of another party.
