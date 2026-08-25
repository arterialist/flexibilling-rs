# Framework integrations

The crate has no required web framework. Bind billing to the host request layer
explicitly.

## Request boundary

Resolve the customer from authenticated request state, check the cache or
repository before expensive work, then create a `UsageContext` around the
operation:

```rust
let customer_id = authenticated_customer_id(&request)?;
require_balance(&customer_id, &["units".into()], &cache)?;

let mut context = UsageContext {
    customer_id: customer_id.clone(),
    service: "report_generation".into(),
    variant: "standard".into(),
    reference_id: Some(request.id.clone()),
    metadata: BTreeMap::new(),
    metrics: Default::default(),
};
let report = generate_report(&request.body).await?;
context.report(
    Decimal::ZERO,
    report.duration_seconds,
    report.units,
    0,
    0,
    0,
    0,
);
write_usage_session(context, false, true, &mut usage_repository)?;
```

The host decides how an insufficient-balance error becomes an HTTP 402 or a
domain-specific response.

## Request wrappers

Rust attributes are not required. A function can call `require_balance`,
`BillingService::charge`, or `write_usage_session` directly. This keeps the
integration compatible with Axum, Actix Web, Rocket, gRPC, and worker runtimes.

## Background worker

`BillingWorker` drains pending records from `BillingRepository`:

```rust
let mut worker = BillingWorker::new(service, 50);
let result = worker.run_once()?;
println!("processed {} records", result.processed);
```

Run the worker from the host's async or scheduled process. The repository must
make record claiming and balance updates safe under concurrent workers.

## Metrics

The crate does not register a metrics library. Count processed, skipped, and
failed records at the worker boundary and export them through the host's
OpenTelemetry, Prometheus, or logging integration.
