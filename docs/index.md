# FlexiBilling for Rust

FlexiBilling is a billing engine for Rust services. It manages named balances,
rates usage, applies priority rules, writes ledger transactions, grants
products idempotently, updates cache views, and processes a pending usage queue.

The crate keeps storage, transactions, web frameworks, caches, and payment
providers in the host application. Implement the public repository and cache
traits for the host's data model, or use the in-memory adapters for tests and
small programs.

## Install

```toml
[dependencies]
flexibilling = "0.1"
```

Amounts use `rust_decimal::Decimal`. The crate targets the Rust 2024 edition.

## Guides

- [Quickstart](quickstart.md) creates rules, funds a customer, and processes usage.
- [Concepts](concepts.md) explains balances, metrics, rules, waterfalls, and ledger entries.
- [Backend integration](backends.md) shows the repository, usage, and cache traits.
- [Framework integrations](integrations.md) covers operation boundaries and workers.
- [Operations](operations.md) covers transactions, retries, cache behavior, and production checks.
- [Development and releases](development.md) covers local checks, CI, docs, and crates.io publishing.

## Behavior

1. Billing decisions do not depend on a storage provider.
2. Asset and service names are application-defined strings.
3. Decimal values are used for balances, rates, and ledger amounts.
4. Cache and observability failures do not change the billing decision.
5. A host owns the transaction used for balance deductions and ledger writes.
