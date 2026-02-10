use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::TfResource;
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct PostgresqlFlexibleServerModel;

struct SkuInfo {
    tier: &'static str,
    series: String,
    vcores: u32,
}

fn parse_sku_name(sku_name: &str) -> Result<SkuInfo, PreciousError> {
    let parts: Vec<&str> = sku_name.split('_').collect();
    if parts.len() < 3 {
        return Err(PreciousError::PricingError(format!(
            "invalid sku_name format: {sku_name}"
        )));
    }

    let tier = match parts[0] {
        "GP" => "General Purpose",
        "MO" => "Memory Optimized",
        "B" => "Burstable",
        other => {
            return Err(PreciousError::PricingError(format!(
                "unknown sku tier prefix: {other}"
            )));
        }
    };

    let vm_part = parts[2..].join("_");

    let vcores = vm_part
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .map_err(|_| {
            PreciousError::PricingError(format!(
                "failed to extract vCores from sku_name: {sku_name}"
            ))
        })?;

    let letter_prefix = vm_part
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .collect::<String>();
    let suffix = vm_part
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();

    let series = format!("{letter_prefix}{suffix}").replace('_', "");

    Ok(SkuInfo {
        tier,
        series,
        vcores,
    })
}

impl PostgresqlFlexibleServerModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let sku_name =
            resource
                .get_string("sku_name")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "sku_name".to_string(),
                })?;

        let region =
            resource
                .get_string("location")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "location".to_string(),
                })?;

        let sku = parse_sku_name(sku_name)?;

        let compute_query = PriceQuery::azure("Azure Database for PostgreSQL", region)
            .filter(
                "productName",
                &format!(
                    "Azure Database for PostgreSQL Flexible Server {} - {} Series Compute",
                    sku.tier, sku.series
                ),
            )
            .filter("skuName", &format!("{} vCore", sku.vcores))
            .filter("priceType", "Consumption");

        let compute_unit_price = pricing.query_price(&compute_query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get compute price for {sku_name}: {e}"))
        })?;

        let hourly_rate = compute_unit_price.price.amount;
        let monthly_hours = BillingPeriod::hours_per_month();
        let compute_monthly_cost = hourly_rate * monthly_hours;

        let storage_mb = resource.get_number("storage_mb").unwrap_or(32768.0);
        let storage_gb = Decimal::from_f64_retain(storage_mb / 1024.0).unwrap_or(Decimal::from(32));

        let storage_query = PriceQuery::azure("Azure Database for PostgreSQL", region)
            .filter(
                "productName",
                "Az DB for PostgreSQL Flexible Server Storage",
            )
            .filter("meterName", "Storage Data Stored")
            .filter("priceType", "Consumption");

        let storage_unit_price = pricing.query_price(&storage_query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get storage price for {sku_name}: {e}"))
        })?;

        let storage_monthly_cost = storage_unit_price.price.amount * storage_gb;

        Ok(vec![
            CostComponent {
                name: SmolStr::new(format!("Compute ({sku_name})")),
                unit: BillingPeriod::Hour,
                quantity: monthly_hours,
                quantity_unit: SmolStr::new("hours"),
                unit_price: compute_unit_price.price,
                monthly_cost: Money::usd(compute_monthly_cost),
                quantity_max: None,
                monthly_cost_max: None,
            },
            CostComponent {
                name: SmolStr::new("Storage"),
                unit: BillingPeriod::GBMonth,
                quantity: storage_gb,
                quantity_unit: SmolStr::new("GB/month"),
                unit_price: storage_unit_price.price,
                monthly_cost: Money::usd(storage_monthly_cost),
                quantity_max: None,
                monthly_cost_max: None,
            },
        ])
    }
}

impl ResourceCostModel for PostgresqlFlexibleServerModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_postgresql_flexible_server")
    }

    fn estimate<'a>(
        &'a self,
        resource: &'a TfResource,
        usage: Option<&'a UsageEntry>,
        pricing: &'a dyn PricingClient,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CostComponent>, PreciousError>> + Send + 'a>> {
        Box::pin(self.do_estimate(resource, usage, pricing))
    }
}
