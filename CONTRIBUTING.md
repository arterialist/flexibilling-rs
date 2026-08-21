# Contributing

## Setup

Install the stable Rust toolchain and run:

```bash
cargo fmt --check
cargo test
```

The unit tests cover exact-decimal rating, waterfall fallback, distinct domain
errors, ledger writes, payment idempotency, usage sessions, charge/refund, and
period snapshots.

## Backend adapters

Implement `BillingRepository` for balances, rules, products, ledger entries,
and usage queue state. Implement `BillingCache` for materialized balance and
period views. The crate's trait calls are synchronous and caller-owned: the
host should invoke the service inside its own transaction or unit of work.

Balance mutation and its ledger entry must share that transaction. Cache writes
are derived views and must not change the billing decision.

## Releases

Create a GitHub release with a semver tag. After a crates.io token and trusted
publishing configuration are available, the release workflow can publish the
crate with `cargo publish`.
