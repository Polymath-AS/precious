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

pub struct MonitorQueryAlertModel;

fn meter_name_for_frequency(freq: &str) -> &'static str {
    match freq {
        "PT1M" => "Alerts System Log Monitored at 1 Minute Frequency",
        "PT5M" => "Alerts System Log Monitored at 5 Minute Frequency",
        "PT10M" => "Alerts System Log Monitored at 10 Minute Frequency",
        "PT15M" | "PT30M" => "Alerts System Log Monitored at 15 Minute Frequency",
        _ => "Alerts System Log Monitored at 5 Minute Frequency",
    }
}

impl MonitorQueryAlertModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let eval_freq = resource
            .get_string("evaluation_frequency")
            .unwrap_or("PT5M");

        let meter_name = meter_name_for_frequency(eval_freq);

        let query = PriceQuery::azure("Azure Monitor", "")
            .filter("productName", "Azure Monitor")
            .filter("skuName", "Alerts")
            .filter("meterName", meter_name);

        match pricing.query_price(&query).await {
            Ok(unit_price) => Ok(vec![CostComponent {
                name: SmolStr::new(format!("Log alert rule ({eval_freq})")),
                unit: BillingPeriod::Month,
                quantity: Decimal::ONE,
                quantity_unit: SmolStr::new("months"),
                unit_price: unit_price.price,
                monthly_cost: unit_price.price,
                quantity_max: None,
                monthly_cost_max: None,
            }]),
            Err(e) => {
                tracing::warn!("could not query log alert rule price for {eval_freq}: {e}");
                let fallback = match eval_freq {
                    "PT1M" => Decimal::new(150, 2),
                    "PT5M" => Decimal::new(150, 2),
                    "PT10M" => Decimal::new(100, 2),
                    "PT15M" | "PT30M" => Decimal::new(50, 2),
                    _ => Decimal::new(150, 2),
                };
                let price = Money::usd(fallback);
                Ok(vec![CostComponent {
                    name: SmolStr::new(format!("Log alert rule ({eval_freq})")),
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

impl ResourceCostModel for MonitorQueryAlertModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(
            Cloud::Azure,
            "azurerm_monitor_scheduled_query_rules_alert_v2",
        )
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
