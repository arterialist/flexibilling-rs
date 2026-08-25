# Quickstart

This guide uses the in-memory adapters. The service calls stay the same when
you implement the repository and cache traits for a real backend.

## 1. Add the crate

```toml
[dependencies]
flexibilling = "0.1"
rust_decimal = "1"
```

## 2. Define a rule and balance

```rust
use flexibilling::{
    BillingRule, InMemoryBillingCache, InMemoryBillingRepository, MetricType,
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
repository.upsert_balance("customer-001", "units", Decimal::from(100))?;
let cache = InMemoryBillingCache::default();
```

## 3. Process usage

```rust
use flexibilling::{BillingService, UsageRecord};

let mut service = BillingService::new(repository, cache);
let mut record = UsageRecord::new("customer-001", "api_request");
record.id = Some("usage-1".into());
record.units = Some(12);
service.repo.records.push(record.clone());
service.process_record(&mut record)?;
assert!(matches!(
    record.billing_status,
    flexibilling::BillingStatus::Processed
));
```

The service selects an active rule by priority, calculates the cost, deducts
the selected balance, writes a ledger transaction, updates the cache, and
marks the record processed.

## 4. Track an operation session

```rust
use flexibilling::{write_usage_session, UsageContext};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

let mut context = UsageContext {
    customer_id: "customer-001".into(),
    service: "api_request".into(),
    variant: "standard".into(),
    reference_id: Some("request-1002".into()),
    metadata: BTreeMap::new(),
    metrics: Default::default(),
};
context.report(Decimal::ZERO, Decimal::new(45, 2), 24, 0, 0, 0, 0);
let _record = write_usage_session(context, false, true, &mut usage_repository)?;
```

The session writes `duration_seconds` to the record and mirrors it into
metadata when the caller has not supplied that key. Set `write_on_exception` to
`false` to skip a record when the operation throws.

## 5. Use a custom backend

Implement `BillingRepository`, `UsageRepository`, and `BillingCache` for the
host database and cache. The in-memory adapters are not required in production.
