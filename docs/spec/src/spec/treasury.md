Treasury {#treasury-module}
========

Overview
--------

Conceptually, the treasury serves as both the central storage for all
interBTC and the interface though which to manage interBTC amount. It is
implemented through the `currency`{.interpreted-text role="ref"} pallet.

There are three main operations on interBTC to interact with the user or
the `issue-protocol`{.interpreted-text role="ref"} and
`redeem-protocol`{.interpreted-text role="ref"} components.

### Step-by-step

-   **Transfer**: A user sends an amount of interBTC to another user by
    calling the `currency_function_transfer`{.interpreted-text
    role="ref"} function.
-   **Issue**: The issue module calls into the treasury when an issue
    request is completed (via `executeIssue`{.interpreted-text
    role="ref"}) and the user has provided a valid proof that the
    required amount of BTC was sent to the correct vault. The issue
    module calls the `currency_function_mint_to`{.interpreted-text
    role="ref"} function to create interBTC.
-   **Redeem**: The redeem protocol requires two calls to the treasury
    module. First, a user requests a redeem via the
    `requestRedeem`{.interpreted-text role="ref"} function. This invokes
    a call to the `currency_function_lock_on`{.interpreted-text
    role="ref"} function that locks the requested amount of tokens for
    this user. Second, when a redeem request is completed (via
    `executeRedeem`{.interpreted-text role="ref"}) and the vault has
    provided a valid proof that it transferred the required amount of
    BTC to the correct user, the redeem module calls the
    `currency_function_burn_from`{.interpreted-text role="ref"} function
    to destroy the previously locked interBTC.
