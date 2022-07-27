Security
========

The Security module is responsible for (1) tracking the status of the
BTC Parachain, (2) the \"active\" blocks of the BTC Parachain, and (3)
generating secure identifiers.

1.  **Status**: The BTC Parachain has three distinct states: `Running`,
    `Error`, and `Shutdown` which determine which functions can be used.
2.  **Active Blocks**: When the BTC Parachain is not in the `Running`
    state, certain operations are restricted. In order to prevent impact
    on the users and Vaults for the core issue, redeem, and replace
    operations, the BTC Parachain only considers Active Blocks for the
    Issue, Redeem, and Replace periods.
3.  **Secure Identifiers**: As part of the `op-return`{.interpreted-text
    role="ref"} scheme to prevent replay attacks, the security module
    generates unique identifiers that are used to identify transactions.

Overview
--------

### Failure Modes

The BTC Parachain can enter into an ERROR and SHUTDOWN state, depending
on the occurred error. An overview is provided in the figure below.

![(Informal) State machine showing the operational and failure modes and
how to recover from or flag failures.](../figures/spec/failureModes.png)

Failure handling methods calls are **restricted**, i.e., can only be
called by pre-determined roles.

### Oracle Offline {#oracle-offline-err}

The `oracle`{.interpreted-text role="ref"} experienced a liveness
failure (no up-to-date exchange rate available). The frequency of the
oracle updates is defined in the Oracle module.

**Error code:** `ORACLE_OFFLINE`

Data Model
----------

### Enums

#### StatusCode {#statusCode}

Indicates ths status of the BTC Parachain.

-   `RUNNING: 0` - BTC Parachain fully operational
-   `ERROR: 1`- an error was detected in the BTC Parachain. See `Errors`
    for more details, i.e., the specific error codes (these determine
    how to react).
-   `SHUTDOWN: 2` - BTC Parachain operation fully suspended. This can
    only be achieved via manual intervention by the Governance
    Mechanism.

#### ErrorCode {#errorCode}

Enum specifying error codes tracked in `Errors`.

-   `NONE: 0`
-   `ORACLE_OFFLINE: 1`

Data Storage
------------

### Scalars

#### ParachainStatus

Stores the status code (`statusCode`{.interpreted-text role="ref"})
which defines the current state of the BTC Parachain.

#### Errors

Stores the set of error codes (`errorCode`{.interpreted-text
role="ref"}), indicating the reason for the error.

#### Nonce

Integer increment-only counter, used to prevent collisions when
generating identifiers for e.g., redeem or replace requests (for
OP\_RETURN field in Bitcoin).

#### ActiveBlockCount {#activeBlockCount}

A counter variable that increments every block when the parachain status
is `RUNNING:0`. This variable is used to keep track of durations, such
as issue/redeem/replace expiry. This is used instead of the block number
because if the parachain status is not `RUNNING:0`, no payment proofs
can be submitted, so it would not be fair towards users and Vaults to
continue counting down the (expiry) periods. This field MUST be set to
the current block height on initialization.

Functions
---------

### generateSecureId {#generateSecureId}

Generates a unique ID using an account identifier, the `Nonce` and a
random seed.

#### Specification

*Function Signature*

`generateSecureId(account)`

*Parameters*

-   `account`: account identifier (links this identifier to the
    AccountId associated with the process where this secure id is to be
    used, e.g., the user calling `requestIssue`{.interpreted-text
    role="ref"}).

*Preconditions*

-   A parent block MUST exist (cannot be called on the parachain genesis
    block).

*Postconditions*

-   Nonce MUST be incremented by one.
-   MUST return the 256-bit hash of the
    `` account`, ``nonce`, and`parent\_hash\`\` (the hash of the
    previous block of this transaction).

### hasExpired {#hasExpired}

Checks if the `period` has expired since the `opentime`. This
calculation is based on the `activeBlockCount`{.interpreted-text
role="ref"}.

#### Specification

*Function Signature*

`hasExpired(opentime, period)`

*Parameters*

-   `opentime`: the `activeBlockCount`{.interpreted-text role="ref"} at
    the time the issue/redeem/replace was opened.
-   `period`: the number of blocks the user or Vault has to complete the
    action.

*Preconditions*

-   The [ActiveBlockCount]{.title-ref} MUST be greater than 0.

*Postconditions*

-   MUST return `True` if `opentime + period < ActiveBlockCount`,
    `False` otherwise.

### setParachainStatus {#setParachainStatus}

Governance sets a status code for the BTC Parachain manually.

#### Specification

*Function Signature*

`setParachainStatus(StatusCode)`

*Parameters*

-   `StatusCode`: the new StatusCode of the BTC-Parachain.

### insertParachainError {#insertParachainError}

Governance inserts an error for the BTC Parachain manually.

#### Specification

*Function Signature*

`insertParachainError(ErrorCode)`

*Parameters*

-   `ErrorCode`: the ErrorCode to be added to the set of errors of the
    BTC-Parachain.

### removeParachainError {#removeParachainError}

Governance removes an error for the BTC Parachain manually.

#### Specification

*Function Signature*

`removeParachainError(ErrorCode)`

*Parameters*

-   `ErrorCode`: the ErrorCode to be removed from the set of errors of
    the BTC-Parachain.

Events
------

### RecoverFromErrors

*Event Signature*

`RecoverFromErrors(StatusCode, ErrorCode[])`

*Parameters*

-   `StatusCode`: the new StatusCode of the BTC Parachain
-   `ErrorCode[]`: the list of current errors
