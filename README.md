# FlexiBilling for Rust

Billing and usage metering for Rust backends. FlexiBilling tracks named
balances, rates usage, and applies priority rules.

The crate follows the [FlexiBilling contract](https://github.com/arterialist/flexibilling).
Traits define persistence and cache operations, so a host application can keep
its existing database and transaction model.

## Install

```toml
[dependencies]
flexibilling = "0.1"
```

Amounts use `rust_decimal::Decimal`. Balance and ledger arithmetic does not use
binary floating-point values.

## Quickstart

```rust
use flexibilling::{
    AssetName, BillingRepository, BillingRule, BillingService, InMemoryBillingCache,
    InMemoryBillingRepository, MetricType, UsageRecord,
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
```

`AssetName` is an open string alias; applications can define their own assets
and services without changing the crate.

## Main components

- `RatingEngine` calculates fixed, quantity, duration, and units costs.
- `WaterfallEngine` selects the first fundable rule by priority.
- `BillingService` processes records, funds products, charges, refunds, and
  updates cache views.
- `BillingRepository`, `UsageRepository`, and `BillingCache` define host ports.
- `InMemoryBillingRepository` and `InMemoryBillingCache` support tests and
  examples.
- `UsageContext` and `write_usage_session` record operation-boundary usage.
- `BillingWorker` processes pending usage records.
- `get_usage_snapshot` returns used and remaining totals for a period.

## Development

```bash
cargo fmt --check
cargo test --all-targets
```

The SQLite integration test uses `rusqlite` with the bundled SQLite engine. It
checks the public repository and cache traits against a persistent database,
including workers, idempotency, refunds, and database reopening.

See [CONTRIBUTING.md](CONTRIBUTING.md) for adapter and release guidance.
