use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct MonitorMetricAlertModel;

impl MonitorMetricAlertModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let has_dynamic = resource.attributes.contains_key("dynamic_criteria");

        let (meter_name, label) = if has_dynamic {
            (
                "Alerts Dynamic Threshold",
                "Metric alert (dynamic threshold)",
            )
        } else {
            ("Alerts Metric Monitored", "Metric alert (static threshold)")
        };

        let query = PriceQuery::azure("Azure Monitor", "")
            .filter("productName", "Azure Monitor")
            .filter("skuName", "Alerts")
            .filter("meterName", meter_name);

        match pricing.query_price(&query).await {
            Ok(unit_price) => Ok(vec![CostComponent {
                name: SmolStr::new(label),
                unit: BillingPeriod::Month,
                quantity: Decimal::ONE,
                quantity_unit: SmolStr::new("months"),
                unit_price: unit_price.price,
                monthly_cost: unit_price.price,
                quantity_max: None,
                monthly_cost_max: None,
            }]),
            Err(e) => {
                tracing::warn!("could not query metric alert price ({meter_name}): {e}");
                let fallback = if has_dynamic {
                    Decimal::ONE
                } else {
                    Decimal::new(10, 2)
                };
                let price = Money::usd(fallback);
                Ok(vec![CostComponent {
                    name: SmolStr::new(label),
                    unit: BillingPeriod::Month,
                    quantity: Decimal::ONE,
                    quantity_unit: SmolStr::new("months"),
                    unit_price: price,
                    monthly_cost: price,
                    quantity_max: None,
                    monthly_cost_max: None,
                }])
            }
        }
    }
}

impl ResourceCostModel for MonitorMetricAlertModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_monitor_metric_alert")
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
