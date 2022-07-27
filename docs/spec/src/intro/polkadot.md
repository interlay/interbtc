Polkadot
========

Polkadot is a [sharded
blockchain](https://wiki.polkadot.network/docs/en/learn-introduction)
that aims to connect multiple different blockchains together. The idea
is that each shard has its custom state transition function. In
Polkadot, a shard is called a
[Parachain](https://wiki.polkadot.network/docs/en/learn-parachains).
Having different shards with varying state transition functions offers
to build blockchains with various cases in mind.

Each blockchain has to make trade-offs in terms of features it wishes to
include. Great examples are Bitcoin which focusses on the core aspect of
asset transfers with limited scripting capabilities. On the other end of
the spectrum is Ethereum that features a (resource-limited) Turing
complete execution environment. With Polkadot, the idea is to allow
transfers between these different blockchains using a concept called
[Bridges](https://wiki.polkadot.network/docs/en/learn-bridges).

Substrate
---------

Polkadot is built using the [Substrate
framework](https://substrate.dev/). Substrate is a blockchain framework
that allows to create custom blockchains. We refer the reader to the
detailed introduction on the [Substrate
website](https://substrate.dev/docs/en/).

Substrate Specifics
-------------------

While this specification does not intend to give a general introduction
to either Polkadot or Substrate, we want to highlight several features
that are relevant to the implementation.

-   **Account-based model**: Substrate uses an account-based model to
    store user\'s and their balances through the
    [Balances](https://substrate.dev/rustdocs/master/pallet_balances/index.html)
    or [Generic
    Asset](https://substrate.dev/rustdocs/master/pallet_generic_asset/index.html)
    modules.
-   **DOT to Parachain**: Currently, there exists no pre-defined module
    to maintain DOT, Polkadot\'s native currency, on Substrate. This
    will be added in the future. For now, we assume such a module exists
    and model its functionality via the Generic Assets module.
-   **Restricting function calls**: Functions declared in Substrate can
    be called by any external party. To restrict calls to specific
    modules, each module can have an account (`AccountId` in Substrate)
    assigned. Restricting a function call can then be enforced by
    limiting calls from pre-defined accounts (i.e. caller `Origin` must
    be equal to the modules `AccountId`).
-   **Failure handling**: Substrate has no implicit failure handling.
    Errors within a function or errors raised in other function calls
    must be handled explicitly in the function implementation. Best
    practice is to (1) verify that the function conditions are met, (2)
    update the state, and (3) emit events and return. *Note*: It is now
    possible to add a `transactional` attribute to function that ensure
    that state is only updated if the function or any of its called
    functions are not resulting in an error. This prevents partial state
    update and must be used on all external functions.
-   **Concurrency**: Substrate does not support concurrent state
    transitions at the moment.
-   **Generic Rust crates**: Substrate does not include the Rust
    standard library due to non-deterministic behavior. However, crates
    can still be used and custom made if they do not depend on the Rust
    standard library.
