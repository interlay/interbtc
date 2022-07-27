Oracle
======

::: {.note}
::: {.title}
Note
:::

This oracle model relies on trusted oracle sources. Decentralized
oracles are a difficult and open research problem that is outside of the
scope of this specification. However, the general interface to get the
exchange rate can remain the same even with different constructions.
:::

The Oracle receives a continuous data feed from off-chain oracles, with
information in exchange rates or bitcoin inclusion estimates. Multiple
oracles can be authorized, in which case the \'median\' of all unexpired
values is used as the actual value. It is not technically the median -
when an even number of oracles have submitted values, it does not
average the middle two values. Instead, it arbitrarily picks one of
them. This is done because this can be done in O(n) rather than in O(n
log n).

In the implementation, the
`oracle_function_feed_values`{.interpreted-text role="ref"} function
does not directly update the aggregate - this is done in the
`oracle_hook_on_initialize`{.interpreted-text role="ref"} hook, in order
to keep the `oracle_function_feed_values`{.interpreted-text role="ref"}
function weight independent of the number of oracles. Furthermore, for
oracle offline detection and for updating the aggregate when a value
becomes outdated, the `oracle_hook_on_initialize`{.interpreted-text
role="ref"} hook was necessary anyway.

The implementation of the oracle client **is not part of this
specification**. InterBTC assumes the oracle operates correctly and that
the received data is reliable.

Data Model
----------

### Enums

#### OracleKey

Key to indicate a specific value.

::: {.tabularcolumns}
L\|
:::

  Discriminant                 Description
  ---------------------------- --------------------------------------------------------------
  `ExchangeRate(CurrencyId)`   Exchange rate against Bitcoin, in e.g. planck per satoshi.
  `FeeEstimation`              Estimate of the Bitcoin inclusion fee, in satoshis per byte.

### Scalars

#### MaxDelay {#MaxDelay}

The time after which a reported value will no longer be considered
valid.

### Maps

#### Aggregate

Maps `oracle_key` to the median of all unexpired values reported by
oracles for that key.

#### AuthorizedOracles

The account(s) of the oracle. Returns true if registered as an oracle.

#### ValidUntil

Maps oracle\_keys to a timestamp that indicates when one of the values
expires, at which time a new aggregate needs to be calculated.

#### RawValues

Maps oracle\_keys and account ids to raw timestamped values.

#### RawValuesUpdated

Maps oracle\_key to a boolean value that indicates that a new value has
been received that has not yet been included in the aggregate.

#### AuthorizedOracles

Maps oracle `accountId` to the oracle\'s name. The presence of an
account id in this map indicates that the account is authorized to feed
values.

Functions
---------

### feed\_values {#oracle_function_feed_values}

The dispatchable function that oracles call to feed new price data into
the system.

#### Specification

*Function Signature*

`feed_values(oracle_id, Vec<oracle_key, value>)`

*Parameters*

-   `oracle_id`: the oracle account calling this function.
-   `oracle_key`: indicated which value is being set
-   `value`: the value being set

*Events*

-   `oracle_event_feed_values`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `oracle_id`.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be `SHUTDOWN:2`.
-   The oracle MUST be authorized.

*Postconditions*

For each `(oracle_key, value)` pair,

-   `RawValuesUpdated[oracle_key]` MUST be set to true
-   `RawValues[oracle_key]` MUST be set to a `TimeStamped` values,
    where,
    -   `TimeStamped.timestamp` MUST be the current time,
    -   `TimeStamped.value` MUST be `value`.

### get\_price {#oracle_function_get_price}

Returns the latest medianized value for the given key, as calculated
from the received external data sources.

#### Specification

*Function Signature*

`get_price(oracle_key)`

*Parameters*

-   `oracle_key`: the key for which the value should be returned

*Preconditions*

-   `ExchangeRate[oracle_key]` MUST NOT be `None`. That is, sufficient
    oracles must have submitted unexpired values.

*Postconditions*

-   MUST return the fixed point value for the given key.

### convert

Converts the given amount to the given currency.

#### Specification

*Function Signature*

`convert(amount, currencyId)`

*Parameters*

-   `amount`: the amount to convert
-   `currencyId`: the currency to convert to

*Preconditions*

-   Exactly one of `amount.currencyId` and the `currencyId` argument
    MUST be the wrapped currency.
-   Exactly one of `amount.currencyId` and the `currencyId` argument
    MUST be a collateral currency.

*Postconditions*

-   MUST return `amount` converted to `currencyId`.

### on\_initialize {#oracle_hook_on_initialize}

This function is called at the start of every block. When new values
have been submitted, or when old values expire, this function update the
aggregate value.

#### Specification

*Function Signature*

`on_initialize()`

*Postconditions*

-   If `RawValuesUpdated` is empty, i.e.,
    `oracle_function_feed_values`{.interpreted-text role="ref"} was not
    yet called since the initialization of the parachain, then the
    `OracleOffline` MUST be set in the `security`{.interpreted-text
    role="ref"} pallet.
-   For each `(oracle_key, updated)` in `RawValuesUpdated`, if `updated`
    is true, or the current time is greater than `ValidUntil[oracle]`,
    -   `RawValuesUpdated[oracle_key]` MUST be set to false
    -   `ExchangeRate[oracle_key]` MUST be set to the middle value of
        the sorted list of unexpired values from
        `RawValues[oracle_key]`. If there are an even number, one MAY be
        arbitrarily picked.
    -   `ValidUntil[oracle_key]` MUST be set to `MaxDelay` plus the
        minimum timestamp from the unexpired values in
        `RawValues[oracle_key]`.

Events
------

### FeedValues {#oracle_event_feed_values}

### SetExchangeRate

Emits the new exchange rate when it is updated by the oracle.

*Event Signature*

`FeedValues(oracle_id, Vec<(oracle_key, value)>),`

*Parameters*

-   `oracle_id`: the oracle account calling this function.
-   `oracle_key`: the key indicating which value is being set
-   `value`: the new value

*Function*

-   `oracle_function_feed_values`{.interpreted-text role="ref"}
