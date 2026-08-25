# FlexiBilling for Rust

[![CI](https://github.com/arterialist/flexibilling-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/arterialist/flexibilling-rs/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/flexibilling.svg)](https://crates.io/crates/flexibilling)
[![Docs](https://img.shields.io/badge/docs-online-blue.svg)](https://arterialist.github.io/flexibilling-rs/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

FlexiBilling is a provider-agnostic billing engine for Rust backends. It tracks
named balances, rates usage, applies priority rules, writes ledger entries, and
processes pending usage records.

The crate does not require a database, web framework, cache, or payment
provider. Implement `BillingRepository` and `BillingCache` for the host's
storage and transaction boundaries, or use the included in-memory adapters.

## Install

```toml
[dependencies]
flexibilling = "0.1"
```

Amounts use `rust_decimal::Decimal`, so balance and ledger calculations do not
use binary floating-point values.

## Quickstart

```rust
use flexibilling::{
    BillingRule, BillingService, InMemoryBillingCache, InMemoryBillingRepository,
    MetricType, UsageRecord,
};
use rust_decimal::Decimal;

let mut repository = InMemoryBillingRepository::new();
repository.rules.push(BillingRule {
    service: "api_request".into(),
    target_asset: "units".into(),
    metric_type: MetricType::Units,
    conversion_rate: Decimal::ONE,
    priority: 10,
    filter_condition: None,
    refund_service_type: None,
    is_active: true,
    id: None,
});
repository.upsert_balance("customer-1", "units", Decimal::from(100))?;

let mut service = BillingService::new(repository, InMemoryBillingCache::default());
let mut record = UsageRecord::new("customer-1", "api_request");
record.id = Some("usage-1".into());
record.units = Some(12);
service.repo.records.push(record.clone());
service.process_record(&mut record)?;
# Ok::<(), flexibilling::BillingError>(())
```

Asset and service names are open strings. The crate does not impose a catalog.

## Usage sessions

Use `UsageContext` and `write_usage_session` when an operation discovers usage:

```rust
use flexibilling::{write_usage_session, UsageContext};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

let mut context = UsageContext {
    customer_id: "customer-1".into(),
    service: "api_request".into(),
    variant: "standard".into(),
    reference_id: Some("request-123".into()),
    metadata: BTreeMap::new(),
    metrics: Default::default(),
};
context.report(Decimal::ZERO, Decimal::new(45, 2), 12, 0, 0, 0, 0);
let _record = write_usage_session(context, false, true, &mut usage_repository)?;
# Ok::<(), flexibilling::BillingError>(())
```

`duration_seconds` is stored on the usage record and mirrored into metadata
under the same key when a caller has not already set it. Set
`write_on_exception` to `false` to skip failed operations.

## What is included

- `BillingService` funds accounts, rates usage, charges, refunds, and updates cache views.
- `BillingRepository`, `UsageRepository`, and `BillingCache` define backend ports.
- `RatingEngine` and `WaterfallEngine` calculate costs and select fundable rules.
- `UsageContext` and `write_usage_session` record operation-boundary usage.
- `BillingWorker` processes pending records with retry-safe state transitions.
- The in-memory adapters support tests and small local programs.

## Documentation

Read the [Rust documentation](https://arterialist.github.io/flexibilling-rs/)
for the quickstart, concepts, backend ports, integrations, operations, and
release process.

## Development

```bash
cargo fmt --check
cargo test --all-targets
uvx --with mkdocs-material mkdocs build --strict
```

The crate uses the 2024 Rust edition. Releases publish through the repository's
GitHub Actions workflow after a version tag is released.

## License

Apache-2.0. See [LICENSE](LICENSE).
