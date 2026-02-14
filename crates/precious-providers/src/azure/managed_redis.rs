use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use precious_pricing::types::{PriceFilter, PriceQuery};
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct ManagedRedisModel;

impl ManagedRedisModel {
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

        let region = resource.get_string("location").unwrap_or("swedencentral");

        let parts: Vec<&str> = sku_name.splitn(2, '_').collect();
        if parts.len() < 2 {
            return Err(PreciousError::InvalidField {
                resource: resource.address.to_string(),
                field: "sku_name".to_string(),
                reason: format!("expected format Tier_Size, got {sku_name}"),
            });
        }

        let tier = match parts[0] {
            "Balanced" => "Balanced",
            "Memory" => "Memory Optimized",
            "Compute" => "Compute Optimized",
            "Flash" => "Flash Optimized",
            other => {
                return Err(PreciousError::InvalidField {
                    resource: resource.address.to_string(),
                    field: "sku_name".to_string(),
                    reason: format!("unknown tier: {other}"),
                });
            }
        };
        let size = parts[1];

        let query = PriceQuery {
            cloud: Cloud::Azure,
            service: "Redis Cache".to_string(),
            region: region.to_string(),
            filters: vec![
                PriceFilter {
                    field: "productName".to_string(),
                    value: format!("Azure Managed Redis - {tier}"),
                },
                PriceFilter {
                    field: "skuName".to_string(),
                    value: size.to_string(),
                },
                PriceFilter {
                    field: "meterName".to_string(),
                    value: format!("{size} Cache Instance"),
                },
            ],
        };

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!(
                "failed to get price for managed redis {sku_name}: {e}"
            ))
        })?;

        let hourly_rate = unit_price.price.amount;
        let monthly_hours = BillingPeriod::hours_per_month();
        let monthly_cost = hourly_rate * monthly_hours;

        Ok(vec![CostComponent {
            name: SmolStr::new(format!("Redis cache ({sku_name})")),
            unit: BillingPeriod::Hour,
            quantity: monthly_hours,
            quantity_unit: SmolStr::new("hours"),
            unit_price: unit_price.price,
            monthly_cost: Money::usd(monthly_cost),
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for ManagedRedisModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_managed_redis")
    }

    fn estimate<'a>(
        &'a self,
        resource: &'a TfResource,
        usage: Option<&'a UsageEntry>,
        pricing: &'a dyn PricingClient,
        _state: &'a State,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CostComponent>, PreciousError>> + Send + 'a>> {
        Box::pin(self.do_estimate(resource, usage, pricing))
    }
}
