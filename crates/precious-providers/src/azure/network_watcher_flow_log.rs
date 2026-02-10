use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{TfResource, TfValue};
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct NetworkWatcherFlowLogModel;

fn is_traffic_analytics_enabled(resource: &TfResource) -> bool {
    match resource.attributes.get("traffic_analytics") {
        Some(TfValue::Map(m)) => m.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        _ => false,
    }
}

impl NetworkWatcherFlowLogModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let region = resource.get_string("location").unwrap_or("eastus");

        let flow_log_gb = usage.and_then(|u| u.get_metric("flow_log_gb"));

        let Some(gb) = flow_log_gb else {
            let mut components = vec![CostComponent {
                name: SmolStr::new("Flow log collection (usage-based)"),
                unit: BillingPeriod::GBMonth,
                quantity: Decimal::ZERO,
                unit_price: Money::zero(),
                monthly_cost: Money::zero(),
            }];

            if is_traffic_analytics_enabled(resource) {
                components.push(CostComponent {
                    name: SmolStr::new("Traffic Analytics (usage-based)"),
                    unit: BillingPeriod::GBMonth,
                    quantity: Decimal::ZERO,
                    unit_price: Money::zero(),
                    monthly_cost: Money::zero(),
                });
            }

            return Ok(components);
        };

        let collection_query = PriceQuery::azure("Network Watcher", region)
            .filter("productName", "Network Watcher")
            .filter("skuName", "Standard")
            .filter("meterName", "Standard Network Logs Collected");

        let collection_price = pricing.query_price(&collection_query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get flow log collection price: {e}"))
        })?;

        let collection_cost = collection_price.price.amount * gb;

        let mut components = vec![CostComponent {
            name: SmolStr::new("Flow log collection"),
            unit: BillingPeriod::GBMonth,
            quantity: gb,
            unit_price: collection_price.price,
            monthly_cost: Money::usd(collection_cost),
        }];

        if is_traffic_analytics_enabled(resource) {
            let ta_query = PriceQuery::azure("Network Watcher", region)
                .filter("productName", "Network Watcher")
                .filter("skuName", "Standard")
                .filter("meterName", "Standard Traffic Analytics Processing");

            let ta_price = pricing.query_price(&ta_query).await.map_err(|e| {
                PreciousError::PricingError(format!("failed to get Traffic Analytics price: {e}"))
            })?;

            let ta_cost = ta_price.price.amount * gb;

            components.push(CostComponent {
                name: SmolStr::new("Traffic Analytics"),
                unit: BillingPeriod::GBMonth,
                quantity: gb,
                unit_price: ta_price.price,
                monthly_cost: Money::usd(ta_cost),
            });
        }

        Ok(components)
    }
}

impl ResourceCostModel for NetworkWatcherFlowLogModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_network_watcher_flow_log")
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
