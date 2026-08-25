# Backend integration

FlexiBilling uses traits rather than a required database schema. Keep the host's
existing models and implement the methods in the public repository ports.

## Ports

- `BillingRepository` handles rules, balances, products, ledger transactions, and queue status.
- `UsageRepository` handles session-created records and usage queries.
- `BillingCache` handles balance snapshots, period statistics, and activity events.
- A transaction factory can provide a transaction for standalone charges and refunds.

Repository methods receive the transaction value defined by the trait. The host
can use a SQL transaction, a unit-of-work object, or an application-specific
guard.

## Minimal implementation shape

```rust
struct DatabaseRepository {
    pool: DatabasePool,
}

impl BillingRepository for DatabaseRepository {
    fn get_active_rules(
        &mut self,
        service: &str,
    ) -> Result<Vec<BillingRule>, BillingError> {
        self.pool.active_rules(service)
    }

    // Implement the remaining required trait methods.
}
```

The traits are intentionally explicit. Rust will check that the complete port
is implemented before the backend can be used with `BillingService`.

## In-memory adapters

`InMemoryBillingRepository` and `InMemoryBillingCache` are useful for tests,
examples, and local experiments. They do not persist across processes and do
not provide cross-process locking.

## SQL databases

The crate does not choose a SQL client. Map record fields directly to the host
schema. Store `Decimal` values as exact decimal or numeric values. Store
`event_metadata` as JSON when the database supports it, and index the customer,
status, service, and created-at fields used by the worker and usage queries.
