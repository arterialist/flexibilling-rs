//! Provider-agnostic usage metering and balance billing.
//!
//! The crate follows the language-neutral contract in the reference repository.
//! Host applications implement [`BillingRepository`] and [`BillingCache`] for
//! their own storage and transaction boundaries. The included in-memory
//! adapters are intended for tests, examples, and small local programs.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

pub type Amount = Decimal;
pub type CustomerId = String;
pub type RecordId = String;
pub type AssetName = String;
pub type ServiceName = String;
pub type Metadata = BTreeMap<String, Value>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Fixed,
    Quantity,
    Duration,
    Units,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Usage,
    TopUp,
    MonthlyGrant,
    Expiration,
    Refund,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductStrategy {
    TopUp,
    MonthlyQuota,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BillingStatus {
    Pending,
    Processed,
    Failed,
    Skipped,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomerBalance {
    pub customer_id: CustomerId,
    pub asset_type: AssetName,
    pub amount: Amount,
    pub id: Option<RecordId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BalanceTransaction {
    pub customer_id: CustomerId,
    pub asset_type: AssetName,
    pub amount: Amount,
    pub balance_after: Amount,
    pub transaction_type: TransactionType,
    pub id: Option<RecordId>,
    pub source_usage_id: Option<RecordId>,
    pub payment_reference: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BalanceTransactionCreate {
    pub customer_id: CustomerId,
    pub asset_type: AssetName,
    pub amount: Amount,
    pub balance_after: Amount,
    pub transaction_type: TransactionType,
    pub source_usage_id: Option<RecordId>,
    pub payment_reference: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BillingRule {
    pub service: ServiceName,
    pub target_asset: AssetName,
    pub metric_type: MetricType,
    pub conversion_rate: Amount,
    pub priority: i32,
    pub filter_condition: Option<Metadata>,
    pub refund_service_type: Option<ServiceName>,
    pub is_active: bool,
    pub id: Option<RecordId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BillingProduct {
    pub external_product_id: String,
    pub asset_type: AssetName,
    pub amount: Amount,
    pub strategy: ProductStrategy,
    pub description: Option<String>,
    pub is_active: bool,
    pub id: Option<RecordId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UsageRecord {
    pub customer_id: CustomerId,
    pub service: ServiceName,
    pub variant: String,
    pub id: Option<RecordId>,
    pub reference_id: Option<RecordId>,
    pub quantity: Option<Amount>,
    pub duration_seconds: Option<Amount>,
    pub units: Option<i64>,
    pub input_units: Option<i64>,
    pub output_units: Option<i64>,
    pub cached_units: Option<i64>,
    pub billing_status: BillingStatus,
    pub billing_error_message: Option<String>,
    pub event_metadata: Option<Metadata>,
    pub created_at: Option<DateTime<Utc>>,
}

impl UsageRecord {
    pub fn new(customer_id: impl Into<String>, service: impl Into<String>) -> Self {
        Self {
            customer_id: customer_id.into(),
            service: service.into(),
            variant: "default".to_owned(),
            id: None,
            reference_id: None,
            quantity: None,
            duration_seconds: None,
            units: None,
            input_units: None,
            output_units: None,
            cached_units: None,
            billing_status: BillingStatus::Pending,
            billing_error_message: None,
            event_metadata: None,
            created_at: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UsageRecordCreate {
    pub customer_id: CustomerId,
    pub service: ServiceName,
    pub variant: String,
    pub reference_id: Option<RecordId>,
    pub quantity: Option<Amount>,
    pub duration_seconds: Option<Amount>,
    pub units: Option<i64>,
    pub input_units: Option<i64>,
    pub output_units: Option<i64>,
    pub cached_units: Option<i64>,
    pub billing_status: BillingStatus,
    pub billing_error_message: Option<String>,
    pub event_metadata: Option<Metadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BillingStats {
    pub usage_count: i64,
    pub quantity: Amount,
    pub spend: Amount,
    pub custom: BTreeMap<String, Amount>,
}

impl Default for BillingStats {
    fn default() -> Self {
        Self {
            usage_count: 0,
            quantity: Decimal::ZERO,
            spend: Decimal::ZERO,
            custom: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UsageSummary {
    pub service: ServiceName,
    pub variant: String,
    pub usage_count: i64,
    pub total_quantity: Option<Amount>,
    pub total_duration_seconds: Option<Amount>,
    pub total_units: Option<i64>,
    pub total_input_units: Option<i64>,
    pub total_output_units: Option<i64>,
    pub total_cached_units: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActivityEvent {
    pub time: String,
    pub action: String,
    pub cost: String,
    pub result: String,
}

#[derive(Debug, Error, PartialEq)]
pub enum BillingError {
    #[error("Customer {customer_id} has insufficient funds for service '{service}'")]
    InsufficientFunds {
        customer_id: CustomerId,
        service: ServiceName,
    },
    #[error("No billable usage for customer {customer_id} service '{service}'")]
    NoBillableUsage {
        customer_id: CustomerId,
        service: ServiceName,
    },
    #[error("No active billing rules found for service '{0}'")]
    RuleNotFound(ServiceName),
    #[error("Gatekeeper denied: customer {0} cannot transact")]
    GatekeeperDenied(CustomerId),
    #[error("billing configuration error: {0}")]
    Configuration(String),
    #[error("billing adapter error: {0}")]
    Adapter(String),
    #[error("{0} amount must be positive")]
    InvalidAmount(String),
}

pub trait BillingRepository {
    fn get_active_rules(&self, service: &str) -> Vec<BillingRule>;
    fn get_customer_balances(&self, customer_id: &str) -> Vec<CustomerBalance>;
    fn upsert_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
    ) -> Result<CustomerBalance, BillingError>;
    fn decrement_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        deduction: Amount,
    ) -> Result<Amount, BillingError>;
    fn increment_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        addition: Amount,
    ) -> Result<Amount, BillingError>;
    fn create_transaction(
        &mut self,
        data: BalanceTransactionCreate,
    ) -> Result<BalanceTransaction, BillingError>;
    fn get_transaction_for_usage(
        &self,
        reference_id: &str,
        service: &str,
        customer_id: &str,
    ) -> Option<BalanceTransaction>;
    fn get_transaction_by_reference(&self, payment_reference: &str) -> Option<BalanceTransaction>;
    fn get_products_for_external_ids(&self, product_ids: &[String]) -> Vec<BillingProduct>;
    fn get_pending_records(&self, limit: usize) -> Vec<UsageRecord>;
    fn mark_record_processed(&mut self, record_id: &str) -> Result<(), BillingError>;
    fn mark_record_failed(&mut self, record_id: &str, message: String) -> Result<(), BillingError>;
    fn mark_record_skipped(&mut self, record_id: &str) -> Result<(), BillingError>;
}

pub trait UsageRepository {
    fn create(&mut self, data: UsageRecordCreate) -> Result<Option<UsageRecord>, BillingError>;
    fn get_by_customer(&self, customer_id: &str, skip: usize, limit: usize) -> Vec<UsageRecord>;
    fn get_usage_summary(
        &self,
        customer_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Vec<UsageSummary>;
    fn get_usage_records(
        &self,
        customer_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        service: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> (Vec<UsageRecord>, usize);
}

pub trait BillingCache {
    fn set_balances(
        &mut self,
        customer_id: &str,
        balances: &BTreeMap<String, Amount>,
    ) -> Result<(), BillingError>;
    fn update_single_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
    ) -> Result<(), BillingError>;
    fn get_balances(&self, customer_id: &str) -> BTreeMap<String, String>;
    fn can_transact(&self, customer_id: &str) -> bool;
    fn get_asset_amount(&self, customer_id: &str, asset_type: &str) -> Option<Amount>;
    fn delete_balances(&mut self, customer_id: &str) -> Result<(), BillingError>;
    fn increment_stats(
        &mut self,
        customer_id: &str,
        month: &str,
        stats: &BillingStats,
    ) -> Result<(), BillingError>;
    fn get_stats(&self, customer_id: &str, month: &str) -> BTreeMap<String, String>;
    fn push_feed_event(
        &mut self,
        customer_id: &str,
        event: ActivityEvent,
    ) -> Result<(), BillingError>;
    fn get_feed(&self, customer_id: &str, limit: usize) -> Vec<ActivityEvent>;
    fn delete_customer_cache(&mut self, customer_id: &str) -> Result<(), BillingError>;
}

pub struct RatingEngine;

impl RatingEngine {
    pub fn calculate_cost(
        rule: &BillingRule,
        record: &UsageRecord,
    ) -> Result<Amount, BillingError> {
        match &rule.metric_type {
            MetricType::Fixed => Ok(rule.conversion_rate),
            MetricType::Quantity => {
                Ok(record.quantity.unwrap_or(Decimal::ZERO) * rule.conversion_rate)
            }
            MetricType::Duration => Ok(extract_duration(record) * rule.conversion_rate),
            MetricType::Units => Ok(Decimal::from(extract_units(record)) * rule.conversion_rate),
            MetricType::Custom(name) => Err(BillingError::Configuration(format!(
                "unknown metric type: {name}"
            ))),
        }
    }

    pub fn matches_filter(rule: &BillingRule, metadata: Option<&Metadata>) -> bool {
        let Some(condition) = &rule.filter_condition else {
            return true;
        };
        let Some(metadata) = metadata else {
            return false;
        };
        condition
            .iter()
            .all(|(key, expected)| resolve_dotted_key(metadata, key) == Some(expected))
    }
}

fn extract_duration(record: &UsageRecord) -> Amount {
    if let Some(metadata) = &record.event_metadata {
        if let Some(value) = metadata.get("duration_seconds") {
            if let Some(text) = value.as_str() {
                if let Ok(parsed) = text.parse() {
                    return parsed;
                }
            }
            if let Some(number) = value.as_f64() {
                if let Ok(parsed) = Decimal::try_from(number) {
                    return parsed;
                }
            }
        }
    }
    record.duration_seconds.unwrap_or(Decimal::ZERO)
}

fn extract_units(record: &UsageRecord) -> i64 {
    record
        .units
        .unwrap_or_else(|| record.input_units.unwrap_or(0) + record.output_units.unwrap_or(0))
}

fn resolve_dotted_key<'a>(metadata: &'a Metadata, key: &str) -> Option<&'a Value> {
    let mut current = metadata.get(key.split('.').next()?)?;
    for part in key.split('.').skip(1) {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaterfallResult {
    pub asset_type: AssetName,
    pub amount: Amount,
    pub rule: BillingRule,
    pub refund_service_type: Option<ServiceName>,
}

pub struct WaterfallEngine;

impl WaterfallEngine {
    pub fn evaluate(
        &self,
        rules: &[BillingRule],
        record: &UsageRecord,
        balances: &BTreeMap<String, Amount>,
    ) -> Result<WaterfallResult, BillingError> {
        if rules.is_empty() {
            return Err(BillingError::RuleNotFound(record.service.clone()));
        }
        let mut ordered: Vec<BillingRule> = rules
            .iter()
            .filter(|rule| rule.is_active)
            .cloned()
            .collect();
        ordered.sort_by_key(|rule| rule.priority);
        let mut saw_positive_cost = false;
        for rule in ordered {
            if !RatingEngine::matches_filter(&rule, record.event_metadata.as_ref()) {
                continue;
            }
            let cost = RatingEngine::calculate_cost(&rule, record)?;
            if cost <= Decimal::ZERO {
                continue;
            }
            saw_positive_cost = true;
            let available = balances
                .get(&rule.target_asset)
                .copied()
                .unwrap_or(Decimal::ZERO);
            if available >= cost {
                return Ok(WaterfallResult {
                    asset_type: rule.target_asset.clone(),
                    amount: cost,
                    refund_service_type: rule.refund_service_type.clone(),
                    rule,
                });
            }
        }
        if !saw_positive_cost {
            return Err(BillingError::NoBillableUsage {
                customer_id: record.customer_id.clone(),
                service: record.service.clone(),
            });
        }
        Err(BillingError::InsufficientFunds {
            customer_id: record.customer_id.clone(),
            service: record.service.clone(),
        })
    }
}

#[derive(Default)]
pub struct InMemoryBillingRepository {
    pub rules: Vec<BillingRule>,
    pub products: Vec<BillingProduct>,
    pub records: Vec<UsageRecord>,
    pub transactions: Vec<BalanceTransaction>,
    pub balances: HashMap<(CustomerId, AssetName), Amount>,
    next_transaction_id: u64,
}

impl InMemoryBillingRepository {
    pub fn new() -> Self {
        Self {
            next_transaction_id: 1,
            ..Self::default()
        }
    }

    fn record_mut(&mut self, record_id: &str) -> Result<&mut UsageRecord, BillingError> {
        self.records
            .iter_mut()
            .find(|record| record.id.as_deref() == Some(record_id))
            .ok_or_else(|| BillingError::Adapter(format!("unknown usage record: {record_id}")))
    }
}

impl BillingRepository for InMemoryBillingRepository {
    fn get_active_rules(&self, service: &str) -> Vec<BillingRule> {
        let mut rules: Vec<_> = self
            .rules
            .iter()
            .filter(|rule| rule.is_active && rule.service == service)
            .cloned()
            .collect();
        rules.sort_by_key(|rule| rule.priority);
        rules
    }

    fn get_customer_balances(&self, customer_id: &str) -> Vec<CustomerBalance> {
        self.balances
            .iter()
            .filter(|((customer, _), _)| customer == customer_id)
            .map(|((customer, asset), amount)| CustomerBalance {
                customer_id: customer.clone(),
                asset_type: asset.clone(),
                amount: *amount,
                id: None,
            })
            .collect()
    }

    fn upsert_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
    ) -> Result<CustomerBalance, BillingError> {
        self.balances
            .insert((customer_id.to_owned(), asset_type.to_owned()), amount);
        Ok(CustomerBalance {
            customer_id: customer_id.to_owned(),
            asset_type: asset_type.to_owned(),
            amount,
            id: None,
        })
    }

    fn decrement_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        deduction: Amount,
    ) -> Result<Amount, BillingError> {
        let key = (customer_id.to_owned(), asset_type.to_owned());
        let current = self.balances.get(&key).copied().unwrap_or(Decimal::ZERO);
        if current < deduction {
            return Err(BillingError::InsufficientFunds {
                customer_id: customer_id.to_owned(),
                service: "charge".to_owned(),
            });
        }
        let next = current - deduction;
        self.balances.insert(key, next);
        Ok(next)
    }

    fn increment_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        addition: Amount,
    ) -> Result<Amount, BillingError> {
        let key = (customer_id.to_owned(), asset_type.to_owned());
        let next = self.balances.get(&key).copied().unwrap_or(Decimal::ZERO) + addition;
        self.balances.insert(key, next);
        Ok(next)
    }

    fn create_transaction(
        &mut self,
        data: BalanceTransactionCreate,
    ) -> Result<BalanceTransaction, BillingError> {
        let transaction = BalanceTransaction {
            customer_id: data.customer_id,
            asset_type: data.asset_type,
            amount: data.amount,
            balance_after: data.balance_after,
            transaction_type: data.transaction_type,
            id: Some(self.next_transaction_id.to_string()),
            source_usage_id: data.source_usage_id,
            payment_reference: data.payment_reference,
            description: data.description,
            created_at: Some(Utc::now()),
        };
        self.next_transaction_id += 1;
        self.transactions.push(transaction.clone());
        Ok(transaction)
    }

    fn get_transaction_for_usage(
        &self,
        reference_id: &str,
        service: &str,
        customer_id: &str,
    ) -> Option<BalanceTransaction> {
        let ids: Vec<_> = self
            .records
            .iter()
            .filter(|record| {
                record.reference_id.as_deref() == Some(reference_id) && record.service == service
            })
            .filter_map(|record| record.id.clone())
            .collect();
        self.transactions
            .iter()
            .rev()
            .find(|transaction| {
                transaction.customer_id == customer_id
                    && transaction
                        .source_usage_id
                        .as_ref()
                        .is_some_and(|id| ids.contains(id))
                    && transaction.transaction_type == TransactionType::Usage
            })
            .cloned()
    }

    fn get_transaction_by_reference(&self, payment_reference: &str) -> Option<BalanceTransaction> {
        self.transactions
            .iter()
            .find(|transaction| transaction.payment_reference.as_deref() == Some(payment_reference))
            .cloned()
    }

    fn get_products_for_external_ids(&self, product_ids: &[String]) -> Vec<BillingProduct> {
        self.products
            .iter()
            .filter(|product| {
                product.is_active && product_ids.contains(&product.external_product_id)
            })
            .cloned()
            .collect()
    }

    fn get_pending_records(&self, limit: usize) -> Vec<UsageRecord> {
        self.records
            .iter()
            .filter(|record| record.billing_status == BillingStatus::Pending)
            .take(limit)
            .cloned()
            .collect()
    }

    fn mark_record_processed(&mut self, record_id: &str) -> Result<(), BillingError> {
        self.record_mut(record_id)?.billing_status = BillingStatus::Processed;
        Ok(())
    }

    fn mark_record_failed(&mut self, record_id: &str, message: String) -> Result<(), BillingError> {
        let record = self.record_mut(record_id)?;
        record.billing_status = BillingStatus::Failed;
        record.billing_error_message = Some(message);
        Ok(())
    }

    fn mark_record_skipped(&mut self, record_id: &str) -> Result<(), BillingError> {
        self.record_mut(record_id)?.billing_status = BillingStatus::Skipped;
        Ok(())
    }
}

impl UsageRepository for InMemoryBillingRepository {
    fn create(&mut self, data: UsageRecordCreate) -> Result<Option<UsageRecord>, BillingError> {
        let record = UsageRecord {
            customer_id: data.customer_id,
            service: data.service,
            variant: data.variant,
            id: Some(format!("usage-{}", self.records.len() + 1)),
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
        self.records.push(record.clone());
        Ok(Some(record))
    }

    fn get_by_customer(&self, customer_id: &str, skip: usize, limit: usize) -> Vec<UsageRecord> {
        let mut records: Vec<_> = self
            .records
            .iter()
            .filter(|record| record.customer_id == customer_id)
            .cloned()
            .collect();
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        records.into_iter().skip(skip).take(limit).collect()
    }

    fn get_usage_summary(
        &self,
        customer_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Vec<UsageSummary> {
        let records = filtered_records(&self.records, customer_id, from, to, None);
        let mut groups: BTreeMap<(String, String), Vec<UsageRecord>> = BTreeMap::new();
        for record in records {
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
        let mut records = filtered_records(&self.records, customer_id, from, to, service);
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        let total = records.len();
        (
            records.into_iter().skip(offset).take(limit).collect(),
            total,
        )
    }
}

fn filtered_records(
    records: &[UsageRecord],
    customer_id: &str,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    service: Option<&str>,
) -> Vec<UsageRecord> {
    records
        .iter()
        .filter(|record| {
            if record.customer_id != customer_id
                || service.is_some_and(|value| record.service != value)
            {
                return false;
            }
            if let Some(from) = from {
                if record.created_at.is_none_or(|value| value < from) {
                    return false;
                }
            }
            if let Some(to) = to {
                if record.created_at.is_none_or(|value| value > to) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

fn sum_amount(values: impl Iterator<Item = Option<Amount>>) -> Option<Amount> {
    let values: Vec<_> = values.flatten().collect();
    (!values.is_empty()).then(|| values.into_iter().sum())
}

fn sum_i64(values: impl Iterator<Item = Option<i64>>) -> Option<i64> {
    let values: Vec<_> = values.flatten().collect();
    (!values.is_empty()).then(|| values.into_iter().sum())
}

#[derive(Default)]
pub struct InMemoryBillingCache {
    balances: HashMap<CustomerId, BTreeMap<String, Amount>>,
    stats: HashMap<(CustomerId, String), BTreeMap<String, Amount>>,
    feed: HashMap<CustomerId, Vec<ActivityEvent>>,
}

impl BillingCache for InMemoryBillingCache {
    fn set_balances(
        &mut self,
        customer_id: &str,
        values: &BTreeMap<String, Amount>,
    ) -> Result<(), BillingError> {
        self.balances.insert(customer_id.to_owned(), values.clone());
        Ok(())
    }

    fn update_single_balance(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
    ) -> Result<(), BillingError> {
        self.balances
            .entry(customer_id.to_owned())
            .or_default()
            .insert(asset_type.to_owned(), amount);
        Ok(())
    }

    fn get_balances(&self, customer_id: &str) -> BTreeMap<String, String> {
        self.balances
            .get(customer_id)
            .map(|values| {
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.normalize().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn can_transact(&self, customer_id: &str) -> bool {
        self.balances
            .get(customer_id)
            .is_some_and(|values| values.values().any(|value| *value > Decimal::ZERO))
    }

    fn get_asset_amount(&self, customer_id: &str, asset_type: &str) -> Option<Amount> {
        self.balances.get(customer_id)?.get(asset_type).copied()
    }

    fn delete_balances(&mut self, customer_id: &str) -> Result<(), BillingError> {
        self.balances.remove(customer_id);
        Ok(())
    }

    fn increment_stats(
        &mut self,
        customer_id: &str,
        month: &str,
        stats: &BillingStats,
    ) -> Result<(), BillingError> {
        let current = self
            .stats
            .entry((customer_id.to_owned(), month.to_owned()))
            .or_default();
        if stats.usage_count != 0 {
            *current.entry("total_usage_count".to_owned()).or_default() +=
                Decimal::from(stats.usage_count);
        }
        if !stats.quantity.is_zero() {
            *current.entry("total_quantity".to_owned()).or_default() += stats.quantity;
        }
        if !stats.spend.is_zero() {
            *current.entry("total_spend".to_owned()).or_default() += stats.spend;
        }
        for (name, value) in &stats.custom {
            *current.entry(format!("total_custom:{name}")).or_default() += *value;
        }
        Ok(())
    }

    fn get_stats(&self, customer_id: &str, month: &str) -> BTreeMap<String, String> {
        self.stats
            .get(&(customer_id.to_owned(), month.to_owned()))
            .map(|values| {
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.normalize().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn push_feed_event(
        &mut self,
        customer_id: &str,
        event: ActivityEvent,
    ) -> Result<(), BillingError> {
        let events = self.feed.entry(customer_id.to_owned()).or_default();
        events.insert(0, event);
        events.truncate(50);
        Ok(())
    }

    fn get_feed(&self, customer_id: &str, limit: usize) -> Vec<ActivityEvent> {
        self.feed
            .get(customer_id)
            .map(|events| events.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    fn delete_customer_cache(&mut self, customer_id: &str) -> Result<(), BillingError> {
        self.balances.remove(customer_id);
        self.feed.remove(customer_id);
        self.stats
            .retain(|(customer, _), _| customer != customer_id);
        Ok(())
    }
}

pub struct NullBillingCache;

impl BillingCache for NullBillingCache {
    fn set_balances(&mut self, _: &str, _: &BTreeMap<String, Amount>) -> Result<(), BillingError> {
        Ok(())
    }
    fn update_single_balance(&mut self, _: &str, _: &str, _: Amount) -> Result<(), BillingError> {
        Ok(())
    }
    fn get_balances(&self, _: &str) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn can_transact(&self, _: &str) -> bool {
        false
    }
    fn get_asset_amount(&self, _: &str, _: &str) -> Option<Amount> {
        None
    }
    fn delete_balances(&mut self, _: &str) -> Result<(), BillingError> {
        Ok(())
    }
    fn increment_stats(&mut self, _: &str, _: &str, _: &BillingStats) -> Result<(), BillingError> {
        Ok(())
    }
    fn get_stats(&self, _: &str, _: &str) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn push_feed_event(&mut self, _: &str, _: ActivityEvent) -> Result<(), BillingError> {
        Ok(())
    }
    fn get_feed(&self, _: &str, _: usize) -> Vec<ActivityEvent> {
        Vec::new()
    }
    fn delete_customer_cache(&mut self, _: &str) -> Result<(), BillingError> {
        Ok(())
    }
}

pub struct BillingService<R, C> {
    pub repo: R,
    pub cache: C,
    clock: Box<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}

impl<R: BillingRepository, C: BillingCache> BillingService<R, C> {
    pub fn new(repo: R, cache: C) -> Self {
        Self {
            repo,
            cache,
            clock: Box::new(Utc::now),
        }
    }

    pub fn with_clock(
        repo: R,
        cache: C,
        clock: impl Fn() -> DateTime<Utc> + Send + Sync + 'static,
    ) -> Self {
        Self {
            repo,
            cache,
            clock: Box::new(clock),
        }
    }

    pub fn process_record(&mut self, record: &mut UsageRecord) -> Result<(), BillingError> {
        let rules = self.repo.get_active_rules(&record.service);
        let balances: BTreeMap<_, _> = self
            .repo
            .get_customer_balances(&record.customer_id)
            .into_iter()
            .map(|row| (row.asset_type, row.amount))
            .collect();
        let result = WaterfallEngine.evaluate(&rules, record, &balances)?;
        let new_amount =
            self.repo
                .decrement_balance(&record.customer_id, &result.asset_type, result.amount)?;
        self.repo.create_transaction(BalanceTransactionCreate {
            customer_id: record.customer_id.clone(),
            asset_type: result.asset_type.clone(),
            amount: -result.amount,
            balance_after: new_amount,
            transaction_type: TransactionType::Usage,
            source_usage_id: record.id.clone(),
            payment_reference: None,
            description: Some(format!(
                "{} usage: -{} {}",
                record.service, result.amount, result.asset_type
            )),
        })?;
        if let (Some(refund_service), Some(reference)) =
            (&result.refund_service_type, &record.reference_id)
        {
            self.handle_refund(record, refund_service, reference)?;
        }
        let record_id = record.id.clone().ok_or_else(|| {
            BillingError::Configuration(
                "a usage record must have an id before it can be processed".to_owned(),
            )
        })?;
        self.repo.mark_record_processed(&record_id)?;
        record.billing_status = BillingStatus::Processed;
        self.sync_cache(record, &result, new_amount)
    }

    pub fn check_permission(&self, customer_id: &str) -> Result<bool, BillingError> {
        if self.cache.get_balances(customer_id).is_empty() || !self.cache.can_transact(customer_id)
        {
            return Err(BillingError::GatekeeperDenied(customer_id.to_owned()));
        }
        Ok(true)
    }

    pub fn check_permission_silent(&self, customer_id: &str) -> bool {
        self.check_permission(customer_id).is_ok()
    }

    pub fn refresh_customer_balance_cache(
        &mut self,
        customer_id: &str,
    ) -> Result<(), BillingError> {
        let rows = self.repo.get_customer_balances(customer_id);
        if rows.is_empty() {
            return self.cache.delete_balances(customer_id);
        }
        let balances = rows
            .into_iter()
            .map(|row| (row.asset_type, row.amount))
            .collect();
        self.cache.set_balances(customer_id, &balances)
    }

    pub fn fund_customer(
        &mut self,
        customer_id: &str,
        product_ids: &[String],
        payment_reference: &str,
    ) -> Result<bool, BillingError> {
        if self
            .repo
            .get_transaction_by_reference(payment_reference)
            .is_some()
        {
            return Ok(false);
        }
        let products = self.repo.get_products_for_external_ids(product_ids);
        if products.is_empty() {
            return Ok(false);
        }
        for product in products {
            let (new_amount, transaction_type, description) = match product.strategy {
                ProductStrategy::TopUp => (
                    self.repo.increment_balance(
                        customer_id,
                        &product.asset_type,
                        product.amount,
                    )?,
                    TransactionType::TopUp,
                    format!(
                        "Top-up: +{} {} (product: {})",
                        product.amount, product.asset_type, product.external_product_id
                    ),
                ),
                ProductStrategy::MonthlyQuota => (
                    self.repo
                        .upsert_balance(customer_id, &product.asset_type, product.amount)?
                        .amount,
                    TransactionType::MonthlyGrant,
                    format!(
                        "Monthly quota reset: {} {} (product: {})",
                        product.amount, product.asset_type, product.external_product_id
                    ),
                ),
                ProductStrategy::Custom(name) => {
                    return Err(BillingError::Configuration(format!(
                        "unknown billing product strategy: {name}"
                    )))
                }
            };
            self.repo.create_transaction(BalanceTransactionCreate {
                customer_id: customer_id.to_owned(),
                asset_type: product.asset_type.clone(),
                amount: product.amount,
                balance_after: new_amount,
                transaction_type,
                source_usage_id: None,
                payment_reference: Some(payment_reference.to_owned()),
                description: Some(description),
            })?;
            self.cache
                .update_single_balance(customer_id, &product.asset_type, new_amount)?;
        }
        Ok(true)
    }

    pub fn charge(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
        description: Option<String>,
    ) -> Result<(), BillingError> {
        if amount <= Decimal::ZERO {
            return Err(BillingError::InvalidAmount("charge".to_owned()));
        }
        let new_balance = self
            .repo
            .decrement_balance(customer_id, asset_type, amount)?;
        self.repo.create_transaction(BalanceTransactionCreate {
            customer_id: customer_id.to_owned(),
            asset_type: asset_type.to_owned(),
            amount: -amount,
            balance_after: new_balance,
            transaction_type: TransactionType::Usage,
            source_usage_id: None,
            payment_reference: None,
            description: Some(
                description.unwrap_or_else(|| format!("charge: {asset_type} x {amount}")),
            ),
        })?;
        self.cache
            .update_single_balance(customer_id, asset_type, new_balance)?;
        let mut stats = BillingStats {
            usage_count: 1,
            quantity: amount,
            spend: amount,
            ..BillingStats::default()
        };
        stats.custom.insert(format!("asset:{asset_type}"), amount);
        self.cache
            .increment_stats(customer_id, &self.month(), &stats)
    }

    pub fn refund(
        &mut self,
        customer_id: &str,
        asset_type: &str,
        amount: Amount,
        description: Option<String>,
    ) -> Result<(), BillingError> {
        if amount <= Decimal::ZERO {
            return Err(BillingError::InvalidAmount("refund".to_owned()));
        }
        let new_balance = self
            .repo
            .increment_balance(customer_id, asset_type, amount)?;
        self.repo.create_transaction(BalanceTransactionCreate {
            customer_id: customer_id.to_owned(),
            asset_type: asset_type.to_owned(),
            amount,
            balance_after: new_balance,
            transaction_type: TransactionType::Refund,
            source_usage_id: None,
            payment_reference: None,
            description: Some(
                description.unwrap_or_else(|| format!("refund: {asset_type} x {amount}")),
            ),
        })?;
        self.cache
            .update_single_balance(customer_id, asset_type, new_balance)
    }

    fn handle_refund(
        &mut self,
        record: &UsageRecord,
        service: &str,
        reference: &str,
    ) -> Result<(), BillingError> {
        let Some(original) =
            self.repo
                .get_transaction_for_usage(reference, service, &record.customer_id)
        else {
            return Ok(());
        };
        let amount = original.amount.abs();
        let new_amount =
            self.repo
                .increment_balance(&record.customer_id, &original.asset_type, amount)?;
        self.repo.create_transaction(BalanceTransactionCreate {
            customer_id: record.customer_id.clone(),
            asset_type: original.asset_type.clone(),
            amount,
            balance_after: new_amount,
            transaction_type: TransactionType::Refund,
            source_usage_id: record.id.clone(),
            payment_reference: None,
            description: Some(format!(
                "Refund for reference {reference}: +{amount} {}",
                original.asset_type
            )),
        })?;
        self.cache
            .update_single_balance(&record.customer_id, &original.asset_type, new_amount)
    }

    fn sync_cache(
        &mut self,
        record: &UsageRecord,
        result: &WaterfallResult,
        new_balance: Amount,
    ) -> Result<(), BillingError> {
        self.cache
            .update_single_balance(&record.customer_id, &result.asset_type, new_balance)?;
        let mut stats = BillingStats {
            usage_count: 1,
            quantity: result.amount,
            spend: result.amount,
            ..BillingStats::default()
        };
        stats
            .custom
            .insert(format!("asset:{}", result.asset_type), result.amount);
        self.cache
            .increment_stats(&record.customer_id, &self.month(), &stats)?;
        self.cache.push_feed_event(
            &record.customer_id,
            ActivityEvent {
                time: Utc::now().to_rfc3339(),
                action: result.rule.service.clone(),
                cost: format!("{} {}", result.amount, result.asset_type),
                result: "Success".to_owned(),
            },
        )
    }

    fn month(&self) -> String {
        (self.clock)().format("%Y-%m").to_string()
    }
}

pub fn has_balance<C: BillingCache>(customer_id: &str, assets: &[String], cache: &C) -> bool {
    let balances = cache.get_balances(customer_id);
    assets
        .iter()
        .map(|asset| {
            balances
                .get(asset)
                .and_then(|value| value.parse::<Amount>().ok())
                .unwrap_or(Decimal::ZERO)
        })
        .sum::<Amount>()
        > Decimal::ZERO
}

pub fn require_balance<C: BillingCache>(
    customer_id: &str,
    assets: &[String],
    cache: &C,
) -> Result<(), BillingError> {
    if has_balance(customer_id, assets, cache) {
        Ok(())
    } else {
        Err(BillingError::GatekeeperDenied(customer_id.to_owned()))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageMetrics {
    pub quantity: Amount,
    pub duration_seconds: Amount,
    pub units: i64,
    pub input_units: i64,
    pub output_units: i64,
    pub cached_units: i64,
    pub events: i64,
}

impl UsageMetrics {
    pub fn is_empty(&self) -> bool {
        self.quantity.is_zero()
            && self.duration_seconds.is_zero()
            && self.units == 0
            && self.input_units == 0
            && self.output_units == 0
            && self.cached_units == 0
            && self.events == 0
    }
}

pub struct UsageContext {
    pub customer_id: CustomerId,
    pub service: ServiceName,
    pub variant: String,
    pub reference_id: Option<RecordId>,
    pub metadata: Metadata,
    pub metrics: UsageMetrics,
}

impl UsageContext {
    pub fn report(
        &mut self,
        quantity: Amount,
        duration_seconds: Amount,
        units: i64,
        input_units: i64,
        output_units: i64,
        cached_units: i64,
        events: i64,
    ) {
        self.metrics.quantity += quantity;
        self.metrics.duration_seconds += duration_seconds;
        self.metrics.units += units;
        self.metrics.input_units += input_units;
        self.metrics.output_units += output_units;
        self.metrics.cached_units += cached_units;
        self.metrics.events += events;
    }
}

pub fn write_usage_session<R: UsageRepository>(
    context: UsageContext,
    failed: bool,
    write_on_exception: bool,
    repository: &mut R,
) -> Result<Option<UsageRecord>, BillingError> {
    if context.metrics.is_empty() || (failed && !write_on_exception) {
        return Ok(None);
    }
    let mut metadata = context.metadata;
    if !context.metrics.duration_seconds.is_zero() {
        metadata
            .entry("duration_seconds".to_owned())
            .or_insert_with(|| Value::String(context.metrics.duration_seconds.to_string()));
    }
    repository.create(UsageRecordCreate {
        customer_id: context.customer_id,
        service: context.service,
        variant: context.variant,
        reference_id: context.reference_id,
        quantity: (!context.metrics.quantity.is_zero()).then_some(context.metrics.quantity),
        duration_seconds: (!context.metrics.duration_seconds.is_zero())
            .then_some(context.metrics.duration_seconds),
        units: (context.metrics.units != 0).then_some(context.metrics.units),
        input_units: (context.metrics.input_units != 0).then_some(context.metrics.input_units),
        output_units: (context.metrics.output_units != 0).then_some(context.metrics.output_units),
        cached_units: (context.metrics.cached_units != 0).then_some(context.metrics.cached_units),
        billing_status: BillingStatus::Pending,
        billing_error_message: None,
        event_metadata: (!metadata.is_empty()).then_some(metadata),
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkerCycleResult {
    pub fetched: usize,
    pub processed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub retried: usize,
}

pub struct BillingWorker<R, C> {
    pub service: BillingService<R, C>,
    pub batch_size: usize,
}

impl<R: BillingRepository, C: BillingCache> BillingWorker<R, C> {
    pub fn new(service: BillingService<R, C>, batch_size: usize) -> Self {
        Self {
            service,
            batch_size,
        }
    }

    pub fn run_once(&mut self) -> Result<WorkerCycleResult, BillingError> {
        let records = self.service.repo.get_pending_records(self.batch_size);
        let mut result = WorkerCycleResult {
            fetched: records.len(),
            ..WorkerCycleResult::default()
        };
        for mut record in records {
            let record_id = record.id.clone();
            match self.service.process_record(&mut record) {
                Ok(()) => result.processed += 1,
                Err(BillingError::NoBillableUsage { .. }) => {
                    result.skipped += 1;
                    if let Some(id) = record_id {
                        self.service.repo.mark_record_skipped(&id)?;
                    }
                }
                Err(error @ BillingError::InsufficientFunds { .. })
                | Err(error @ BillingError::RuleNotFound(_))
                | Err(error @ BillingError::Configuration(_)) => {
                    result.failed += 1;
                    if let Some(id) = record_id {
                        self.service
                            .repo
                            .mark_record_failed(&id, error.to_string())?;
                    }
                }
                Err(_) => result.retried += 1,
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsageMetric {
    pub used: f64,
    pub total: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsageSnapshot {
    pub period: String,
    pub metrics: BTreeMap<String, UsageMetric>,
}

pub fn get_usage_snapshot<C: BillingCache>(
    customer_id: &str,
    assets: &[String],
    cache: &C,
    now: DateTime<Utc>,
) -> UsageSnapshot {
    let period = now.format("%Y-%m").to_string();
    let balances = cache.get_balances(customer_id);
    let stats = cache.get_stats(customer_id, &period);
    let metrics = assets
        .iter()
        .map(|asset| {
            let used = stats
                .get(&format!("total_custom:asset:{asset}"))
                .and_then(|value| value.parse::<Amount>().ok())
                .unwrap_or(Decimal::ZERO);
            let remaining = balances
                .get(asset)
                .and_then(|value| value.parse::<Amount>().ok())
                .unwrap_or(Decimal::ZERO)
                .max(Decimal::ZERO);
            (
                asset.clone(),
                UsageMetric {
                    used: used.to_string().parse().unwrap_or(0.0),
                    total: (used + remaining).to_string().parse().unwrap_or(0.0),
                },
            )
        })
        .collect();
    UsageSnapshot { period, metrics }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(metric_type: MetricType, asset: &str, priority: i32) -> BillingRule {
        BillingRule {
            service: "api_request".into(),
            target_asset: asset.into(),
            metric_type,
            conversion_rate: Decimal::ONE,
            priority,
            filter_condition: None,
            refund_service_type: None,
            is_active: true,
            id: None,
        }
    }

    #[test]
    fn rating_and_waterfall_match_contract_vectors() {
        let mut record = UsageRecord::new("customer-1", "api_request");
        record.quantity = Some(Decimal::from(3));
        record.duration_seconds = Some(Decimal::from(60));
        record.input_units = Some(500);
        record.output_units = Some(200);
        assert_eq!(
            RatingEngine::calculate_cost(
                &BillingRule {
                    conversion_rate: Decimal::from(2),
                    ..rule(MetricType::Quantity, "units", 1)
                },
                &record
            )
            .unwrap(),
            Decimal::from(6)
        );
        assert_eq!(
            RatingEngine::calculate_cost(
                &BillingRule {
                    conversion_rate: Decimal::new(1, 3),
                    ..rule(MetricType::Units, "units", 1)
                },
                &record
            )
            .unwrap(),
            Decimal::new(7, 1)
        );
        record.units = Some(60);
        let result = WaterfallEngine
            .evaluate(
                &[
                    rule(MetricType::Units, "units", 10),
                    rule(MetricType::Units, "prepaid_units", 20),
                ],
                &record,
                &BTreeMap::from([
                    (String::from("units"), Decimal::ZERO),
                    (String::from("prepaid_units"), Decimal::from(200)),
                ]),
            )
            .unwrap();
        assert_eq!(result.asset_type, "prepaid_units");
        let zero = UsageRecord::new("customer-1", "api_request");
        assert!(matches!(
            WaterfallEngine.evaluate(
                &[rule(MetricType::Units, "units", 1)],
                &zero,
                &BTreeMap::from([(String::from("units"), Decimal::from(100))])
            ),
            Err(BillingError::NoBillableUsage { .. })
        ));
    }

    #[test]
    fn service_funding_is_idempotent_and_updates_cache() {
        let mut repository = InMemoryBillingRepository::new();
        repository.rules.push(rule(MetricType::Units, "units", 1));
        repository.products.push(BillingProduct {
            external_product_id: "plan-standard".into(),
            asset_type: "units".into(),
            amount: Decimal::from(100),
            strategy: ProductStrategy::TopUp,
            description: None,
            is_active: true,
            id: None,
        });
        repository
            .upsert_balance("customer-1", "units", Decimal::from(100))
            .unwrap();
        let mut service =
            BillingService::with_clock(repository, InMemoryBillingCache::default(), || Utc::now());
        let mut record = UsageRecord::new("customer-1", "api_request");
        record.id = Some("usage-1".into());
        record.units = Some(30);
        service.repo.records.push(record.clone());
        service.process_record(&mut record).unwrap();
        assert_eq!(
            service.cache.get_asset_amount("customer-1", "units"),
            Some(Decimal::from(70))
        );
        assert!(service
            .fund_customer("customer-1", &["plan-standard".into()], "payment-1")
            .unwrap());
        assert!(!service
            .fund_customer("customer-1", &["plan-standard".into()], "payment-1")
            .unwrap());
        assert_eq!(
            service.repo.get_customer_balances("customer-1")[0].amount,
            Decimal::from(170)
        );
    }

    #[test]
    fn charge_refund_snapshot_and_usage_session_work() {
        let mut repository = InMemoryBillingRepository::new();
        let mut context = UsageContext {
            customer_id: "customer-1".into(),
            service: "background_task".into(),
            variant: "default".into(),
            reference_id: Some("job-1".into()),
            metadata: BTreeMap::new(),
            metrics: UsageMetrics::default(),
        };
        context.report(Decimal::ZERO, Decimal::from(95), 0, 10, 0, 0, 0);
        write_usage_session(context, false, true, &mut repository).unwrap();
        repository
            .upsert_balance("customer-1", "units", Decimal::from(90))
            .unwrap();
        let mut service = BillingService::new(repository, InMemoryBillingCache::default());
        service
            .charge("customer-1", "units", Decimal::from(10), None)
            .unwrap();
        service
            .refund("customer-1", "units", Decimal::from(10), None)
            .unwrap();
        let snapshot =
            get_usage_snapshot("customer-1", &["units".into()], &service.cache, Utc::now());
        assert_eq!(snapshot.metrics["units"].used, 10.0);
    }
}
