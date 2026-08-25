# Contributing

## Setup

Install the current stable Rust toolchain (the crate uses Rust edition 2024)
and run:

```bash
cargo fmt --check
cargo test --all-targets
```

The unit tests cover decimal rating, waterfall fallback, domain errors, ledger
writes, payment idempotency, usage sessions, charges, refunds, and snapshots.

`tests/sqlite_integration.rs` is the real-backend conformance test. It uses
`rusqlite` only as a dev dependency and exercises the exported repository and
cache traits without coupling the crate to SQLite in production.

## Backend adapters

Implement `BillingRepository` for balances, rules, products, ledger entries,
and usage queue state. Implement `BillingCache` for balance and period views.
Trait calls are synchronous and caller-owned. Invoke the service inside the
host transaction or unit of work.

Balance mutation and its ledger entry must share that transaction. Cache writes
are derived views and must not change the billing decision.

## Releases

Create a GitHub release with a semver tag. After a crates.io token and trusted
publishing configuration are available, the release workflow can publish the
crate with `cargo publish`.
