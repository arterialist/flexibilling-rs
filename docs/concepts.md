# Concepts

## Core objects

| Object | Purpose |
| --- | --- |
| `CustomerBalance` | Current amount of one named asset for a customer. |
| `BillingRule` | Maps a service and metric to an asset and conversion rate. |
| `UsageRecord` | A billable event waiting to be rated and processed. |
| `BalanceTransaction` | Immutable ledger movement from funding, usage, or refunds. |
| `BillingProduct` | External product identifier mapped to a balance grant. |
| `BillingCache` | Materialized balances, period counters, and activity feed. |

Customer IDs, service names, asset names, variants, and reference IDs are
application-defined strings.

## Rating metrics

| Metric | Cost calculation |
| --- | --- |
| `Fixed` | `conversion_rate` once per record. |
| `Quantity` | `quantity` multiplied by the rate. |
| `Duration` | `duration_seconds` multiplied by the rate. |
| `Units` | `units`, or input plus output units, multiplied by the rate. |

Duration can come from `UsageRecord.duration_seconds` or the
`event_metadata["duration_seconds"]` value. Metadata takes precedence, which
lets a host record an authoritative elapsed value without changing its model.

## The priority waterfall

The engine sorts active rules by ascending `priority`:

1. It skips a rule whose metadata filter does not match.
2. It calculates a positive cost.
3. It selects the first target asset with enough balance.
4. It reports no billable usage when no rule produces a positive cost.
5. It reports insufficient funds when costs exist but no rule can be funded.

This supports a primary quota with a prepaid fallback without reserving either
asset name.

## Transactions and idempotency

The repository owns the transaction that locks a balance, deducts the amount,
inserts the ledger entry, and updates record status. `fund_customer` is
idempotent on `payment_reference`, so a retried payment event does not grant a
product twice.

`charge` and `refund` can use a caller-owned transaction or a configured
transaction factory.

## Queue states

The worker marks expected outcomes explicitly:

- processed records become `Processed`;
- records without billable usage become `Skipped`;
- insufficient funds and other expected billing failures become `Failed`;
- unexpected errors remain pending for a later retry.
