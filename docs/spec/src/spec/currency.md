Currency
========

Overview
--------

This currency pallet provides an interface for the other pallets to
manage balances of different currencies. Accounts have three balances
per currency: they have a `free`, `reserved`, and `frozen` amount. Users
are able to freely transfer `free - frozen` balances, but only the
parachain pallets are able to operate on `reserved` amounts. `Frozen` is
used to implement temptorary locks of free balances like vesting
schedules.

The external API for dispatchable and RPC functions use \'thin\' amount
types, meaning that the used currency depends on the context. For
example, the currency used in
`vault_registry_function_deposit_collateral`{.interpreted-text
role="ref"} depends on the vault\'s `currencyId`. Sometimes, as is for
example the case for
`vault_registry_function_register_vault`{.interpreted-text role="ref"},
the function takes an additional `currencyId` argument to specify the
currency to use. In contrast, internally in the parachain amounts are
often represented by the `Amount` type defined in this pallet, which in
addition to the amount, also contains the used currency. The benefit of
this type is two-fold. First, we can guarantee that operations only work
on compatible amounts. For example, it prevents adding DOT amounts to
KSM amounts. Second, it allows for a more convenient api.

Data Model
----------

### Structs

#### Amount

Stores an amount and the used currency.

::: {.tabularcolumns}
l
:::

  Parameter    Type         Description
  ------------ ------------ --------------------
  `balance`    Balance      The amount.
  `currency`   CurrencyId   The used currency.

Functions
---------

### from\_signed\_fixed\_point {#currency_function_from_signed_fixed_point}

Constructs an `Amount` from a signed fixed point number and a
`currencyId`. The fixed point number is truncated. E.g., a value of 2.5
would return 2.

#### Specification

*Function Signature*

`from_signed_fixed_point(amount, currencyId)`

*Parameters*

-   `amount`: The amount as fixed point.
-   `currencyId`: The currency.

*Preconditions*

-   `amount` MUST be representable as a 128 bit unsigned number.

*Postconditions*

-   An `Amount` MUST be returned where `Amount.amount` is the truncated
    `amount` argument, and `Amount.currencyId` is the `currencyId`
    argument.

### to\_signed\_fixed\_point {#currency_function_to_signed_fixed_point}

Converts an `Amount` struct into a fixed-point number.

#### Specification

*Function Signature*

`to_signed_fixed_point(amount)`

*Parameters*

-   `amount`: The amount struct.

*Preconditions*

-   `amount` MUST be representable by the signed fixed point type.

*Postconditions*

-   `amount.amount` MUST be returned as a fixed point number.

### convert\_to {#currency_function_convert_to}

Converts the given `amount` into the given currency.

#### Specification

*Function Signature*

`convert_to(amount, currencyId)`

*Parameters*

-   `amount`: The amount struct.
-   `currencyId`: The currency to convert to.

*Preconditions*

-   `convert`{.interpreted-text role="ref"} when called with `amount`
    and `currencyId` MUST return successfully.

*Postconditions*

-   `convert`{.interpreted-text role="ref"} MUST be called with `amount`
    and `currencyId` as arguments.

### checked\_add {#currency_function_checked_add}

Adds two amounts.

#### Specification

*Function Signature*

`checked_add(amount1, amount2)`

*Parameters*

-   `amount1`: the first amount.
-   `amount2`: the second amount.

*Preconditions*

-   `amount1.currencyId` MUST be equal to `amount2.currencyId`

*Postconditions*

-   MUST return the sum of both amounts.

### checked\_sub {#currency_function_checked_sub}

Subtracts two amounts.

#### Specification

*Function Signature*

`checked_sub(amount1, amount2)`

*Parameters*

-   `amount1`: the first amount.
-   `amount2`: the second amount.

*Preconditions*

-   `amount1.currencyId` MUST be equal to `amount2.currencyId`

*Postconditions*

-   MUST return `amount1 - amount2`.

### saturating\_sub {#currency_function_saturating_sub}

Subtracts two amounts, or zero if the result would be negative.

#### Specification

*Function Signature*

`saturating_sub(amount1, amount2)`

*Parameters*

-   `amount1`: the first amount.
-   `amount2`: the second amount.

*Preconditions*

-   `amount1.currencyId` MUST be equal to `amount2.currencyId`

*Postconditions*

-   if `amount2 <= amount1`, then this function MUST return
    `amount1 - amount2`.
-   if `amount2 > amount1`, then this function MUST return zero.

### checked\_fixed\_point\_mul {#currency_function_checked_fixed_point_mul}

Multiplies an amount by a fixed point scalar. The result is rounded
down.

#### Specification

*Function Signature*

`checked_fixed_point_mul(amount, scalar)`

*Parameters*

-   `amount`: the Amount struct.
-   `scalar`: the fixed point scalar.

*Preconditions*

-   The multiplied amount MUST be representable by a 128 bit unsigned
    integer.

*Postconditions*

-   MUST return a copy of `amount` that is multiplied by the scalar. The
    result MUST be rounded down.

### checked\_fixed\_point\_mul\_rounded\_up {#currency_function_checked_fixed_point_mul_rounded_up}

Like `currency_function_checked_fixed_point_mul`{.interpreted-text
role="ref"}, but with a rounded-up result.

#### Specification

*Function Signature*

`checked_fixed_point_mul_rounded_up(amount, scalar)`

*Parameters*

-   `amount`: the Amount struct.
-   `scalar`: the fixed point scalar.

*Preconditions*

-   The multiplied amount MUST be representable by a 128 bit unsigned
    integer.

*Postconditions*

-   MUST return a copy of `amount` that is multiplied by the scalar. The
    result MUST be rounded up.

### rounded\_mul {#currency_function_rounded_mul}

Like `currency_function_checked_fixed_point_mul`{.interpreted-text
role="ref"}, but with a rounded result.

#### Specification

*Function Signature*

`rounded_mul(amount, scalar)`

*Parameters*

-   `amount`: the Amount struct.
-   `scalar`: the fixed point scalar.

*Preconditions*

-   The multiplied amount MUST be representable by a 128 bit unsigned
    integer.

*Postconditions*

-   MUST return a copy of `amount` that is multiplied by the scalar. The
    result MUST be rounded to the nearest integer.

### checked\_div {#currency_function_checked_div}

Divides an amount by a fixed point scalar. The result is rounded down.

#### Specification

*Function Signature*

`checked_div(amount, scalar)`

*Parameters*

-   `amount`: the Amount struct.
-   `scalar`: the fixed point scalar.

*Preconditions*

-   The multiplied amount MUST be representable by a 128 bit unsigned
    integer.

*Postconditions*

-   MUST return a copy of `amount` that is divided by the scalar.

### ratio {#currency_function_ratio}

Returns the fixed point ratio between two amounts.

#### Specification

*Function Signature*

`ratio(amount1, amount2)`

*Parameters*

-   `amount1`: the first Amount struct.
-   `amount2`: the second Amount struct.

*Preconditions*

-   `amount1.currencyId` MUST be equal to `amount2.currencyId`
-   The ratio MUST be representable by the fixed point type.

*Postconditions*

-   MUST return the ratio between the two amounts.

### Comparisons: lt, le, eq, ge, gt {#currency_function_cmp}

Compares two amounts

#### Specification

*Function Signature*

`[lt|le|eq|ge|gt](amount1, amount2)`

*Parameters*

-   `amount1`: the first Amount struct.
-   `amount2`: the second Amount struct.

*Preconditions*

-   `amount1.currencyId` MUST be equal to `amount2.currencyId`

*Postconditions*

-   MUST return true when the comparison holds.

### transfer {#currency_function_transfer}

Transfers the amount between the given accounts.

#### Specification

*Function Signature*

`transfer(amount, source, destination)`

*Parameters*

-   `amount`: the Amount struct.
-   `source`: the account to transfer from.
-   `destination`: the account to transfer to.

*Preconditions*

-   `source` MUST have sufficient unlocked funds in the given currency

*Postconditions*

-   The free balance of `source` MUST decrease by `amount.amount` (in
    the currency determined by `amount.currencyId)`
-   The free balance of `destination` MUST increase by `amount.amount`
    (in the currency determined by `amount.currencyId)`

### lock\_on {#currency_function_lock_on}

Locks the amount on the given account.

#### Specification

*Function Signature*

`lock_on(amount, accountId)`

*Parameters*

-   `amount`: the Amount struct.
-   `accountId`: the account to lock the amount on.

*Preconditions*

-   The given account MUST have sufficient unlocked funds in the given
    currency.

*Postconditions*

-   The free balance of `accountId` MUST decrease by `amount.amount` (in
    the currency determined by `amount.currencyId)`
-   The locked balance of `accountId` MUST increase by `amount.amount`
    (in the currency determined by `amount.currencyId)`

### unlock\_on {#currency_function_unlock_on}

Unlocks the amount on the given account.

#### Specification

*Function Signature*

`unlock_on(amount, accountId)`

*Parameters*

-   `amount`: the Amount struct.
-   `accountId`: the account to unlock the amount on.

*Preconditions*

-   The given account MUST have sufficient locked funds in the given
    currency.

*Postconditions*

-   The locked balance of `accountId` MUST decrease by `amount.amount`
    (in the currency determined by `amount.currencyId)`
-   The free balance of `accountId` MUST increase by `amount.amount` (in
    the currency determined by `amount.currencyId)`

### burn\_from {#currency_function_burn_from}

Burns the amount on the given account.

#### Specification

*Function Signature*

`burn_from(amount, accountId)`

*Parameters*

-   `amount`: the Amount struct.
-   `accountId`: the account to lock the amount on.

*Preconditions*

-   The given account MUST have sufficient locked funds in the given
    currency.

*Postconditions*

-   The locked balance of `accountId` MUST decrease by `amount.amount`
    (in the currency determined by `amount.currencyId)`

### mint\_to {#currency_function_mint_to}

Mints the amount on the given account.

#### Specification

*Function Signature*

`mint_to(amount, accountId)`

*Parameters*

-   `amount`: the Amount struct.
-   `accountId`: the account to mint the amount on.

*Postconditions*

-   The `free` balance of `accountId` MUST increase by `amount.amount`
    (in the currency determined by `amount.currencyId)`
