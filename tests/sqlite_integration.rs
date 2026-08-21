use chrono::{DateTime, Utc};
use flexibilling::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

type Db = Rc<RefCell<Connection>>;

fn adapter_error(error: rusqlite::Error) -> BillingError {
    BillingError::Adapter(error.to_string())
}

fn amount(value: &str) -> Amount {
    value.parse().unwrap()
}

fn transaction_name(value: &TransactionType) -> String {
    match value {
        TransactionType::Usage => "usage".into(),
        TransactionType::TopUp => "top_up".into(),
        TransactionType::MonthlyGrant => "monthly_grant".into(),
        TransactionType::Expiration => "expiration".into(),
        TransactionType::Refund => "refund".into(),
        TransactionType::Custom(value) => value.clone(),
    }
}

fn status_name(value: &BillingStatus) -> String {
    match value {
        BillingStatus::Pending => "pending".into(),
        BillingStatus::Processed => "processed".into(),
        BillingStatus::Failed => "failed".into(),
        BillingStatus::Skipped => "skipped".into(),
        BillingStatus::Custom(value) => value.clone(),
    }
}

fn initialize(db: &Db) {
    db.borrow()
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS rules (
                id TEXT PRIMARY KEY,
                service TEXT NOT NULL,
                active INTEGER NOT NULL,
                priority INTEGER NOT NULL,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS products (
                external_product_id TEXT PRIMARY KEY,
                active INTEGER NOT NULL,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS balances (
                customer_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                amount TEXT NOT NULL,
                PRIMARY KEY (customer_id, asset_type)
            );
            CREATE TABLE IF NOT EXISTS usage_records (
                id TEXT PRIMARY KEY,
                customer_id TEXT NOT NULL,
                service TEXT NOT NULL,
                status TEXT NOT NULL,
                reference_id TEXT,
                created_at TEXT NOT NULL,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                customer_id TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                payment_reference TEXT,
                source_usage_id TEXT,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cache_balances (
                customer_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                amount TEXT NOT NULL,
                PRIMARY KEY (customer_id, asset_type)
            );
            CREATE TABLE IF NOT EXISTS cache_stats (
                customer_id TEXT NOT NULL,
                month TEXT NOT NULL,
                metric TEXT NOT NULL,
                amount TEXT NOT NULL,
                PRIMARY KEY (customer_id, month, metric)
            );
            CREATE TABLE IF NOT EXISTS cache_feed (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                customer_id TEXT NOT NULL,
                payload TEXT NOT NULL
            );
            ",
        )
        .unwrap();
}

#[derive(Clone)]
struct SqliteRepository {
    db: Db,
}

impl SqliteRepository {
    fn new(db: Db) -> Self {
        initialize(&db);
        Self { db }
    }

    fn seed_rule(&self, rule: BillingRule) {
        let id = rule.id.clone().unwrap_or_else(|| "rule-1".into());
        self.db
            .borrow()
            .execute(
                "INSERT INTO rules (id, service, active, priority, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id,
                    rule.service,
                    if rule.is_active { 1 } else { 0 },
                    rule.priority,
                    serde_json::to_string(&rule).unwrap()
                ],
            )
            .unwrap();
    }

    fn seed_product(&self, product: BillingProduct) {
        self.db
            .borrow()
            .execute(
                "INSERT INTO products (external_product_id, active, payload) VALUES (?1, ?2, ?3)",
                params![
                    product.external_product_id,
                    if product.is_active { 1 } else { 0 },
                    serde_json::to_string(&product).unwrap()
                ],
            )
            .unwrap();
    }

    fn transaction_count(&self) -> usize {
        self.db
            .borrow()
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap() as usize
    }

    fn records(&self, customer_id: &str) -> Vec<UsageRecord> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT payload FROM usage_records WHERE customer_id = ?1")
            .unwrap();
        statement
            .query_map(params![customer_id], |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).unwrap())
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn all_records(&self) -> Vec<UsageRecord> {
        let conn = self.db.borrow();
        let mut statement = conn.prepare("SELECT payload FROM usage_records").unwrap();
        statement
            .query_map([], |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).unwrap())
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn save_record(&self, record: &UsageRecord) -> Result<(), BillingError> {
        let id = record.id.clone().unwrap();
        self.db
            .borrow()
            .execute(
                "UPDATE usage_records SET status = ?1, payload = ?2 WHERE id = ?3",
                params![
                    status_name(&record.billing_status),
                    serde_json::to_string(record).unwrap(),
                    id
                ],
            )
            .map(|_| ())
            .map_err(adapter_error)
    }
}

impl BillingRepository for SqliteRepository {
    fn get_active_rules(&self, service: &str) -> Vec<BillingRule> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare(
                "SELECT payload FROM rules WHERE service = ?1 AND active = 1 ORDER BY priority",
            )
            .unwrap();
        statement
            .query_map(params![service], |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).unwrap())
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn get_customer_balances(&self, customer_id: &str) -> Vec<CustomerBalance> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT asset_type, amount FROM balances WHERE customer_id = ?1")
            .unwrap();
        statement
            .query_map(params![customer_id], |row| {
                let asset_type: String = row.get(0)?;
                let amount: String = row.get(1)?;
                Ok(CustomerBalance {
                    customer_id: customer_id.to_owned(),
                    asset_type,
                    amount: amount.parse().unwrap(),
                    id: None,
                })
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn upsert_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        value: Amount,
    ) -> Result<CustomerBalance, BillingError> {
        self.db
            .borrow()
            .execute(
                "INSERT INTO balances (customer_id, asset_type, amount) VALUES (?1, ?2, ?3) ON CONFLICT(customer_id, asset_type) DO UPDATE SET amount = excluded.amount",
                params![customer_id, asset_type, value.to_string()],
            )
            .map_err(adapter_error)?;
        Ok(CustomerBalance {
            customer_id: customer_id.to_owned(),
            asset_type: asset_type.to_owned(),
            amount: value,
            id: None,
        })
    }

    fn decrement_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        deduction: Amount,
    ) -> Result<Amount, BillingError> {
        let current = self
            .db
            .borrow()
            .query_row(
                "SELECT amount FROM balances WHERE customer_id = ?1 AND asset_type = ?2",
                params![customer_id, asset_type],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(adapter_error)?
            .map(|value| amount(&value))
            .unwrap_or_default();
        if current < deduction {
            return Err(BillingError::InsufficientFunds {
                customer_id: customer_id.to_owned(),
                service: "charge".into(),
            });
        }
        let next = current - deduction;
        self.db
            .borrow()
            .execute(
                "UPDATE balances SET amount = ?1 WHERE customer_id = ?2 AND asset_type = ?3",
                params![next.to_string(), customer_id, asset_type],
            )
            .map_err(adapter_error)?;
        Ok(next)
    }

    fn increment_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        addition: Amount,
    ) -> Result<Amount, BillingError> {
        let current = self
            .db
            .borrow()
            .query_row(
                "SELECT amount FROM balances WHERE customer_id = ?1 AND asset_type = ?2",
                params![customer_id, asset_type],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(adapter_error)?
            .map(|value| amount(&value))
            .unwrap_or_default();
        let next = current + addition;
        self.db
            .borrow()
            .execute(
                "INSERT INTO balances (customer_id, asset_type, amount) VALUES (?1, ?2, ?3) ON CONFLICT(customer_id, asset_type) DO UPDATE SET amount = excluded.amount",
                params![customer_id, asset_type, next.to_string()],
            )
            .map_err(adapter_error)?;
        Ok(next)
    }

    fn create_transaction(
        &mut self,
        data: BalanceTransactionCreate,
    ) -> Result<BalanceTransaction, BillingError> {
        let created_at = Utc::now();
        let transaction_type = data.transaction_type.clone();
        let transaction = BalanceTransaction {
            customer_id: data.customer_id,
            asset_type: data.asset_type,
            amount: data.amount,
            balance_after: data.balance_after,
            transaction_type,
            id: None,
            source_usage_id: data.source_usage_id,
            payment_reference: data.payment_reference,
            description: data.description,
            created_at: Some(created_at),
        };
        let result = self
            .db
            .borrow()
            .execute(
                "INSERT INTO transactions (customer_id, transaction_type, payment_reference, source_usage_id, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    transaction.customer_id,
                    transaction_name(&transaction.transaction_type),
                    transaction.payment_reference,
                    transaction.source_usage_id,
                    serde_json::to_string(&transaction).unwrap()
                ],
            )
            .map_err(adapter_error)?;
        let id = self.db.borrow().last_insert_rowid().to_string();
        let mut transaction = transaction;
        transaction.id = Some(id);
        self.db
            .borrow()
            .execute(
                "UPDATE transactions SET payload = ?1 WHERE rowid = last_insert_rowid()",
                params![serde_json::to_string(&transaction).unwrap()],
            )
            .map_err(adapter_error)?;
        let _ = result;
        Ok(transaction)
    }

    fn get_transaction_for_usage(
        &self,
        reference_id: &str,
        service: &str,
        customer_id: &str,
    ) -> Option<BalanceTransaction> {
        let ids: Vec<String> = self
            .records(customer_id)
            .into_iter()
            .filter(|record| {
                record.reference_id.as_deref() == Some(reference_id) && record.service == service
            })
            .filter_map(|record| record.id)
            .collect();
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT payload FROM transactions WHERE customer_id = ?1 AND transaction_type = 'usage' ORDER BY id DESC")
            .unwrap();
        let payloads: Vec<String> = statement
            .query_map(params![customer_id], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        payloads
            .into_iter()
            .map(|payload| serde_json::from_str::<BalanceTransaction>(&payload).unwrap())
            .find(|transaction| {
                transaction
                    .source_usage_id
                    .as_ref()
                    .is_some_and(|id| ids.contains(id))
            })
    }

    fn get_transaction_by_reference(&self, payment_reference: &str) -> Option<BalanceTransaction> {
        self.db
            .borrow()
            .query_row(
                "SELECT payload FROM transactions WHERE payment_reference = ?1 ORDER BY id LIMIT 1",
                params![payment_reference],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .unwrap()
            .map(|payload| serde_json::from_str(&payload).unwrap())
    }

    fn get_products_for_external_ids(&self, product_ids: &[String]) -> Vec<BillingProduct> {
        let conn = self.db.borrow();
        product_ids
            .iter()
            .filter_map(|product_id| {
                conn.query_row(
                    "SELECT payload FROM products WHERE external_product_id = ?1 AND active = 1",
                    params![product_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .unwrap()
                .map(|payload| serde_json::from_str(&payload).unwrap())
            })
            .collect()
    }

    fn get_pending_records(&self, limit: usize) -> Vec<UsageRecord> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT payload FROM usage_records WHERE status = 'pending' ORDER BY created_at LIMIT ?1")
            .unwrap();
        statement
            .query_map(params![limit as i64], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
            .collect()
    }

    fn mark_record_processed(&mut self, record_id: &str) -> Result<(), BillingError> {
        let mut record = self
            .all_records()
            .into_iter()
            .find(|record| record.id.as_deref() == Some(record_id))
            .ok_or_else(|| BillingError::Adapter(format!("unknown usage record: {record_id}")))?;
        record.billing_status = BillingStatus::Processed;
        self.save_record(&record)
    }

    fn mark_record_failed(&mut self, record_id: &str, message: String) -> Result<(), BillingError> {
        let mut record = self
            .all_records()
            .into_iter()
            .find(|record| record.id.as_deref() == Some(record_id))
            .ok_or_else(|| BillingError::Adapter(format!("unknown usage record: {record_id}")))?;
        record.billing_status = BillingStatus::Failed;
        record.billing_error_message = Some(message);
        self.save_record(&record)
    }

    fn mark_record_skipped(&mut self, record_id: &str) -> Result<(), BillingError> {
        let mut record = self
            .all_records()
            .into_iter()
            .find(|record| record.id.as_deref() == Some(record_id))
            .ok_or_else(|| BillingError::Adapter(format!("unknown usage record: {record_id}")))?;
        record.billing_status = BillingStatus::Skipped;
        self.save_record(&record)
    }
}

impl UsageRepository for SqliteRepository {
    fn create(&mut self, data: UsageRecordCreate) -> Result<Option<UsageRecord>, BillingError> {
        let next_id = self
            .db
            .borrow()
            .query_row("SELECT COUNT(*) FROM usage_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(adapter_error)?
            + 1;
        let record = UsageRecord {
            customer_id: data.customer_id,
            service: data.service,
            variant: data.variant,
            id: Some(format!("usage-{next_id}")),
            reference_id: data.reference_id,
            quantity: data.quantity,
            duration_seconds: data.duration_seconds,
            units: data.units,
            input_units: data.input_units,
            output_units: data.output_units,
            cached_units: data.cached_units,
            billing_status: data.billing_status,
            billing_error_message: data.billing_error_message,
            event_metadata: data.event_metadata,
            created_at: Some(Utc::now()),
        };
        self.db
            .borrow()
            .execute(
                "INSERT INTO usage_records (id, customer_id, service, status, reference_id, created_at, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.id,
                    record.customer_id,
                    record.service,
                    status_name(&record.billing_status),
                    record.reference_id,
                    record.created_at.map(|value| value.to_rfc3339()),
                    serde_json::to_string(&record).unwrap()
                ],
            )
            .map_err(adapter_error)?;
        Ok(Some(record))
    }

    fn get_by_customer(&self, customer_id: &str, skip: usize, limit: usize) -> Vec<UsageRecord> {
        let mut records = self.records(customer_id);
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        records.into_iter().skip(skip).take(limit).collect()
    }

    fn get_usage_summary(
        &self,
        customer_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Vec<UsageSummary> {
        let mut groups: BTreeMap<(String, String), Vec<UsageRecord>> = BTreeMap::new();
        for record in self.records(customer_id).into_iter().filter(|record| {
            from.is_none_or(|value| record.created_at.is_some_and(|created| created >= value))
                && to.is_none_or(|value| record.created_at.is_some_and(|created| created <= value))
        }) {
            groups
                .entry((record.service.clone(), record.variant.clone()))
                .or_default()
                .push(record);
        }
        groups
            .into_iter()
            .map(|((service, variant), records)| UsageSummary {
                service,
                variant,
                usage_count: records.len() as i64,
                total_quantity: sum_amount(records.iter().map(|record| record.quantity)),
                total_duration_seconds: sum_amount(
                    records.iter().map(|record| record.duration_seconds),
                ),
                total_units: sum_i64(records.iter().map(|record| record.units)),
                total_input_units: sum_i64(records.iter().map(|record| record.input_units)),
                total_output_units: sum_i64(records.iter().map(|record| record.output_units)),
                total_cached_units: sum_i64(records.iter().map(|record| record.cached_units)),
            })
            .collect()
    }

    fn get_usage_records(
        &self,
        customer_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        service: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> (Vec<UsageRecord>, usize) {
        let mut records: Vec<_> = self
            .records(customer_id)
            .into_iter()
            .filter(|record| service.is_none_or(|value| record.service == value))
            .filter(|record| {
                from.is_none_or(|value| record.created_at.is_some_and(|created| created >= value))
                    && to.is_none_or(|value| {
                        record.created_at.is_some_and(|created| created <= value)
                    })
            })
            .collect();
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        let total = records.len();
        (
            records.into_iter().skip(offset).take(limit).collect(),
            total,
        )
    }
}

fn sum_amount(values: impl Iterator<Item = Option<Amount>>) -> Option<Amount> {
    let values: Vec<_> = values.flatten().collect();
    (!values.is_empty()).then(|| values.into_iter().sum())
}

fn sum_i64(values: impl Iterator<Item = Option<i64>>) -> Option<i64> {
    let values: Vec<_> = values.flatten().collect();
    (!values.is_empty()).then(|| values.into_iter().sum())
}

#[derive(Clone)]
struct SqliteCache {
    db: Db,
}

impl SqliteCache {
    fn new(db: Db) -> Self {
        initialize(&db);
        Self { db }
    }

    fn add_stat(
        &mut self,
        customer_id: &str,
        month: &str,
        metric: &str,
        value: Amount,
    ) -> Result<(), BillingError> {
        if value.is_zero() {
            return Ok(());
        }
        let current = self
            .db
            .borrow()
            .query_row(
                "SELECT amount FROM cache_stats WHERE customer_id = ?1 AND month = ?2 AND metric = ?3",
                params![customer_id, month, metric],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(adapter_error)?
            .map(|value| amount(&value))
            .unwrap_or_default();
        self.db
            .borrow()
            .execute(
                "INSERT INTO cache_stats (customer_id, month, metric, amount) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(customer_id, month, metric) DO UPDATE SET amount = excluded.amount",
                params![customer_id, month, metric, (current + value).to_string()],
            )
            .map_err(adapter_error)?;
        Ok(())
    }
}

impl BillingCache for SqliteCache {
    fn set_balances(
        &mut self,
        customer_id: &str,
        balances: &BTreeMap<String, Amount>,
    ) -> Result<(), BillingError> {
        self.delete_balances(customer_id)?;
        for (asset, value) in balances {
            self.update_single_balance(customer_id, asset, *value)?;
        }
        Ok(())
    }

    fn update_single_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        value: Amount,
    ) -> Result<(), BillingError> {
        self.db
            .borrow()
            .execute(
                "INSERT INTO cache_balances (customer_id, asset_type, amount) VALUES (?1, ?2, ?3) ON CONFLICT(customer_id, asset_type) DO UPDATE SET amount = excluded.amount",
                params![customer_id, asset_type, value.to_string()],
            )
            .map_err(adapter_error)?;
        Ok(())
    }

    fn get_balances(&self, customer_id: &str) -> BTreeMap<String, String> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT asset_type, amount FROM cache_balances WHERE customer_id = ?1")
            .unwrap();
        statement
            .query_map(params![customer_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn can_transact(&self, customer_id: &str) -> bool {
        self.get_balances(customer_id)
            .values()
            .any(|value| amount(value) > Amount::ZERO)
    }

    fn get_asset_amount(&self, customer_id: &str, asset_type: &str) -> Option<Amount> {
        self.db
            .borrow()
            .query_row(
                "SELECT amount FROM cache_balances WHERE customer_id = ?1 AND asset_type = ?2",
                params![customer_id, asset_type],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .unwrap()
            .map(|value| amount(&value))
    }

    fn delete_balances(&mut self, customer_id: &str) -> Result<(), BillingError> {
        self.db
            .borrow()
            .execute(
                "DELETE FROM cache_balances WHERE customer_id = ?1",
                params![customer_id],
            )
            .map(|_| ())
            .map_err(adapter_error)
    }

    fn increment_stats(
        &mut self,
        customer_id: &str,
        month: &str,
        stats: &BillingStats,
    ) -> Result<(), BillingError> {
        self.add_stat(
            customer_id,
            month,
            "total_usage_count",
            Amount::from(stats.usage_count),
        )?;
        self.add_stat(customer_id, month, "total_quantity", stats.quantity)?;
        self.add_stat(customer_id, month, "total_spend", stats.spend)?;
        for (name, value) in &stats.custom {
            self.add_stat(customer_id, month, &format!("total_custom:{name}"), *value)?;
        }
        Ok(())
    }

    fn get_stats(&self, customer_id: &str, month: &str) -> BTreeMap<String, String> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare("SELECT metric, amount FROM cache_stats WHERE customer_id = ?1 AND month = ?2")
            .unwrap();
        statement
            .query_map(params![customer_id, month], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn push_feed_event(
        &mut self,
        customer_id: &str,
        event: ActivityEvent,
    ) -> Result<(), BillingError> {
        self.db
            .borrow()
            .execute(
                "INSERT INTO cache_feed (customer_id, payload) VALUES (?1, ?2)",
                params![customer_id, serde_json::to_string(&event).unwrap()],
            )
            .map(|_| ())
            .map_err(adapter_error)
    }

    fn get_feed(&self, customer_id: &str, limit: usize) -> Vec<ActivityEvent> {
        let conn = self.db.borrow();
        let mut statement = conn
            .prepare(
                "SELECT payload FROM cache_feed WHERE customer_id = ?1 ORDER BY id DESC LIMIT ?2",
            )
            .unwrap();
        statement
            .query_map(params![customer_id, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .unwrap()
            .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
            .collect()
    }

    fn delete_customer_cache(&mut self, customer_id: &str) -> Result<(), BillingError> {
        self.delete_balances(customer_id)?;
        self.db
            .borrow()
            .execute(
                "DELETE FROM cache_stats WHERE customer_id = ?1",
                params![customer_id],
            )
            .map_err(adapter_error)?;
        self.db
            .borrow()
            .execute(
                "DELETE FROM cache_feed WHERE customer_id = ?1",
                params![customer_id],
            )
            .map(|_| ())
            .map_err(adapter_error)
    }
}

fn usage_data(customer_id: &str, units: Option<i64>) -> UsageRecordCreate {
    UsageRecordCreate {
        customer_id: customer_id.into(),
        service: "api_request".into(),
        variant: "default".into(),
        reference_id: None,
        quantity: None,
        duration_seconds: None,
        units,
        input_units: None,
        output_units: None,
        cached_units: None,
        billing_status: BillingStatus::Pending,
        billing_error_message: None,
        event_metadata: None,
    }
}

fn rule() -> BillingRule {
    BillingRule {
        service: "api_request".into(),
        target_asset: "units".into(),
        metric_type: MetricType::Units,
        conversion_rate: Amount::ONE,
        priority: 10,
        filter_condition: None,
        refund_service_type: None,
        is_active: true,
        id: Some("rule-1".into()),
    }
}

fn database_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!(
        "flexibilling-rs-{}-{nanos}.sqlite",
        std::process::id()
    ))
}

#[test]
fn public_ports_work_with_persistent_sqlite_backend() {
    let path = database_path();
    let db = Rc::new(RefCell::new(Connection::open(&path).unwrap()));
    let repository = SqliteRepository::new(db.clone());
    let cache = SqliteCache::new(db.clone());
    repository.seed_rule(rule());
    repository.seed_product(BillingProduct {
        external_product_id: "plan-standard".into(),
        asset_type: "units".into(),
        amount: Amount::from(100),
        strategy: ProductStrategy::TopUp,
        description: None,
        is_active: true,
        id: None,
    });
    let mut service = BillingService::with_clock(repository, cache, || {
        "2026-08-20T00:00:00Z".parse().unwrap()
    });
    service
        .repo
        .upsert_balance("customer-1", "units", Amount::from(100))
        .unwrap();
    let mut first = service
        .repo
        .create(usage_data("customer-1", Some(4)))
        .unwrap()
        .unwrap();
    service.process_record(&mut first).unwrap();
    assert_eq!(first.billing_status, BillingStatus::Processed);
    assert_eq!(
        service.repo.get_customer_balances("customer-1")[0].amount,
        Amount::from(96)
    );
    assert_eq!(service.repo.transaction_count(), 1);

    assert!(
        service
            .fund_customer("customer-1", &["plan-standard".into()], "payment-1")
            .unwrap()
    );
    assert!(
        !service
            .fund_customer("customer-1", &["plan-standard".into()], "payment-1")
            .unwrap()
    );
    assert_eq!(
        service.repo.get_customer_balances("customer-1")[0].amount,
        Amount::from(196)
    );

    let mut context = UsageContext {
        customer_id: "customer-1".into(),
        service: "api_request".into(),
        variant: "default".into(),
        reference_id: Some("request-2".into()),
        metadata: BTreeMap::new(),
        metrics: UsageMetrics::default(),
    };
    context.report(Amount::ZERO, amount("1.5"), 2, 0, 0, 0, 0);
    write_usage_session(context, false, true, &mut service.repo).unwrap();
    let mut worker = BillingWorker::new(service, 10);
    let cycle = worker.run_once().unwrap();
    assert_eq!(cycle.processed, 1);

    let failed = worker
        .service
        .repo
        .create(usage_data("customer-1", Some(999)))
        .unwrap()
        .unwrap();
    let cycle = worker.run_once().unwrap();
    assert_eq!(cycle.failed, 1);
    assert_eq!(
        worker
            .service
            .repo
            .records("customer-1")
            .into_iter()
            .find(|record| record.id == failed.id)
            .unwrap()
            .billing_status,
        BillingStatus::Failed
    );

    worker
        .service
        .charge("customer-1", "units", Amount::from(10), None)
        .unwrap();
    worker
        .service
        .refund("customer-1", "units", Amount::from(10), None)
        .unwrap();
    let snapshot = get_usage_snapshot(
        "customer-1",
        &["units".into()],
        &worker.service.cache,
        "2026-08-20T00:00:00Z".parse().unwrap(),
    );
    assert_eq!(snapshot.metrics["units"].used, 16.0);
    assert_eq!(snapshot.metrics["units"].total, 210.0);
    assert_eq!(worker.service.repo.transaction_count(), 5);

    drop(worker);
    drop(db);
    let reopened = Connection::open(&path).unwrap();
    assert_eq!(
        reopened
            .query_row(
                "SELECT amount FROM balances WHERE customer_id = 'customer-1' AND asset_type = 'units'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "194"
    );
    assert_eq!(
        reopened
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        5
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
