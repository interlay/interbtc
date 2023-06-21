Escrow {#escrow-protocol}
======

Overview
--------

The Escrow module allows users to lockup tokens in exchange for a
non-fungible voting asset. The total \"power\" of this asset decays
linearly as the lock approaches expiry - calculated based on the block
height. Historic points for the linear function are recorded each time a
user\'s balance is adjusted which allows us to re-construct voting power
at a particular point in time.

This architecture was adopted from Curve, see: [Vote-Escrowed CRV
(veCRV)](https://curve.readthedocs.io/dao-vecrv.html).

::: {.note}
::: {.title}
Note
:::

This specification is still a Work-in-Progress (WIP), some information
may be outdated or incomplete.
:::

### Step-by-step

1.  A user may lock any amount of defined governance currency (KINT on
    Kintsugi, INTR on Interlay) up to a maximum lock period.
2.  Both the amount and the unlock time may be increased to improve
    voting power.
3.  The user may unlock their fungible asset after the lock has expired.

Data Model
----------

### Constants

#### Span {#escrow_constant_span}

The locktime is rounded to weeks to limit checkpoint iteration.

#### MaxPeriod {#escrow_constant_max_period}

The maximum period for lockup.

### Scalars

#### Epoch {#escrow_scalar_epoch}

The current global epoch for `PointHistory`.

### Maps

#### Locked {#escrow_map_locked}

Stores the `amount` and `end` block for an account\'s lock.

#### PointHistory {#escrow_map_point_history}

Stores the global `bias`, `slope` and `height` at a particular point in
history.

#### UserPointHistory {#escrow_map_user_point_history}

Stores the `bias`, `slope` and `height` for an account at a particular
point in history.

#### UserPointEpoch {#escrow_map_user_point_epoch}

Stores the current epoch for an account.

#### SlopeChanges {#escrow_map_slope_changes}

Stores scheduled changes of slopes for ending locks.

### Structs

#### LockedBalance

The `amount` and `end` height for a locked balance.

::: {.tabularcolumns}
l
:::

  Parameter   Type          Description
  ----------- ------------- ---------------------------------------------------------
  `amount`    Balance       The amount deposited to receive vote-escrowed tokens.
  `end`       BlockNumber   The end height after which the balance can be unlocked.

#### Point

The `bias`, `slope` and `height` for our linear function.

::: {.tabularcolumns}
l
:::

  Parameter   Type          Description
  ----------- ------------- ------------------------------------------------------
  `bias`      Balance       The bias for the linear function.
  `slope`     Balance       The slope for the linear function.
  `height`    BlockNumber   The current block height when this point was stored.

External Functions
------------------

### create\_lock {#escrow_function_create_lock}

Create a lock on the account\'s balance to expire in the future.

#### Specification

*Function Signature*

`create_lock(who, amount, unlock_height)`

*Parameters*

-   `who`: The user\'s address.
-   `amount`: The amount to be locked.
-   `unlock_height`: The height to lock until.

*Events*

-   `escrow_event_deposit`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `who`.
-   The `amount` MUST be non-zero.
-   The account\'s `old_locked.amount` MUST be non-zero.
-   The `unlock_height` MUST be greater than `now`.
-   The `unlock_height` MUST NOT be greater than `now + MaxPeriod`.

*Postconditions*

-   The account\'s `LockedBalance` MUST be set as follows:

    > -   `new_locked.amount`: MUST be the `amount`.
    > -   `new_locked.end`: MUST be the `unlock_height`.

-   The `UserPointEpoch` MUST increase by one.

-   A new `Point` MUST be recorded at this epoch:

    > -   `slope = amount / max_period`
    > -   `bias = slope * (unlock_height - now)`
    > -   `height = now`

-   Function `reward_withdrawStake`{.interpreted-text role="ref"} MUST
    complete successfully using the account\'s total stake.

-   Function `reward_depositStake`{.interpreted-text role="ref"} MUST
    complete successfully using the current balance
    (`escrow_function_balance_at`{.interpreted-text role="ref"}).

### increase\_amount {#escrow_function_increase_amount}

Deposit additional tokens for a pre-existing lock to improve voting
power.

#### Specification

*Function Signature*

`increase_amount(who, amount)`

*Parameters*

-   `who`: The user\'s address.
-   `amount`: The amount to be locked.

*Events*

-   `escrow_event_deposit`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `who`.
-   The `amount` MUST be non-zero.
-   The account\'s `old_locked.amount` MUST be non-zero.
-   The account\'s `old_locked.end` MUST be greater than `now`.

*Postconditions*

-   The account\'s `LockedBalance` MUST be set as follows:

    > -   `new_locked.amount`: MUST be `old_locked.amount + amount`.
    > -   `new_locked.end`: MUST be the `old_locked.end`.

-   The `UserPointEpoch` MUST increase by one.

-   A new `Point` MUST be recorded at this epoch:

    > -   `slope = new_locked.amount / max_period`
    > -   `bias = slope * (new_locked.end - now)`
    > -   `height = now`

### extend\_unlock\_height {#escrow-function-extend-unlock-height}

Push back the expiry on a pre-existing lock to retain voting power.

#### Specification

*Function Signature*

`extend_unlock_height(who, unlock_height)`

*Parameters*

-   `who`: The user\'s address.
-   `unlock_height`: The new expiry deadline.

*Events*

-   `escrow_event_deposit`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `who`.
-   The `amount` MUST be non-zero.
-   The account\'s `old_locked.amount` MUST be non-zero.
-   The account\'s `old_locked.end` MUST be greater than `now`.
-   The `unlock_height` MUST be greater than `old_locked.end`.
-   The `unlock_height` MUST NOT be greater than `now + MaxPeriod`.

*Postconditions*

-   The account\'s `LockedBalance` MUST be set as follows:

    > -   `new_locked.amount`: MUST be `old_locked.amount`.
    > -   `new_locked.end`: MUST be the `unlock_height`.

-   The `UserPointEpoch` MUST increase by one.

-   A new `Point` MUST be recorded at this epoch:

    > -   `slope = new_locked.amount / max_period`
    > -   `bias = slope * (new_locked.end - now)`
    > -   `height = now`

### withdraw {#escrow_function_withdraw}

Remove the lock on an account to allow access to the account\'s funds.

#### Specification

*Function Signature*

`withdraw(who)`

*Parameters*

-   `who`: The user\'s address.

*Events*

-   `escrow_event_withdraw`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `who`.
-   The account\'s `old_locked.amount` MUST be non-zero.
-   The current height (`now`) MUST be greater than or equal to
    `old_locked.end`.

*Postconditions*

-   The account\'s `LockedBalance` MUST be removed.
-   Function `reward_withdrawStake`{.interpreted-text role="ref"} MUST
    complete successfully using the account\'s total stake.

Internal Functions
------------------

### balance\_at {#escrow_function_balance_at}

Using the `Point`, we can calculate the current voting power (`balance`)
as follows:

> `balance = point.bias - (point.slope * (height - point.height))`

#### Specification

*Function Signature*

`balance_at(who, height)`

*Parameters*

-   `who`: The user\'s address.
-   `height`: The future height.

*Preconditions*

-   The `height` MUST be `>= point.height`.

Events
------

### Deposit {#escrow_event_deposit}

Emit an event if a user successfully deposited tokens or increased the
lock time.

*Event Signature*

`Deposit(who, amount, unlock_height)`

*Parameters*

-   `who`: The user\'s account identifier.
-   `amount`: The amount locked.
-   `unlock_height`: The height to unlock after.

*Functions*

-   `escrow_function_create_lock`{.interpreted-text role="ref"}

### Withdraw {#escrow_event_withdraw}

Emit an event if a user withdrew previously locked tokens.

*Event Signature*

`Withdraw(who, amount)`

*Parameters*

-   `who`: The user\'s account identifier.
-   `amount`: The amount unlocked.

*Functions*

-   `escrow_function_withdraw`{.interpreted-text role="ref"}
