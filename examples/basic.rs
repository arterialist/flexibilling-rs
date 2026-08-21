use flexibilling::{
    BillingRepository, BillingRule, BillingService, InMemoryBillingCache,
    InMemoryBillingRepository, MetricType, UsageRecord,
};
use rust_decimal::Decimal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    println!("processed: {:?}", record.billing_status);
    Ok(())
}
