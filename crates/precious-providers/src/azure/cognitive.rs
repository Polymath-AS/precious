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

pub struct CognitiveDeploymentModel;

impl CognitiveDeploymentModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let sku_name = resource
            .get_nested_str(&["sku", "name"])
            .unwrap_or("Standard");

        if sku_name != "ProvisionedManaged" && sku_name != "Provisioned-Managed" {
            return Ok(vec![CostComponent {
                name: SmolStr::new(format!("Deployment ({sku_name}, usage-based)")),
                unit: BillingPeriod::Month,
                quantity: Decimal::ZERO,
                unit_price: Money::zero(),
                monthly_cost: Money::zero(),
            }]);
        }

        let capacity = resource.get_nested_f64(&["sku", "capacity"]).unwrap_or(1.0);
        let capacity_dec = Decimal::try_from(capacity).map_err(|e| {
            PreciousError::PricingError(format!("invalid PTU capacity '{capacity}': {e}"))
        })?;

        let model_name = resource
            .get_nested_str(&["model", "name"])
            .unwrap_or("gpt-4o");

        let query = PriceQuery::azure("Cognitive Services", "")
            .filter("productName", "Azure OpenAI")
            .filter("skuName", "Provisioned Managed");

        match pricing.query_price(&query).await {
            Ok(unit_price) => {
                let monthly_hours = BillingPeriod::hours_per_month();
                let hourly_rate = unit_price.price.amount;
                let monthly_cost = hourly_rate * capacity_dec * monthly_hours;

                Ok(vec![CostComponent {
                    name: SmolStr::new(format!(
                        "Provisioned Throughput ({model_name}, {capacity} PTUs)"
                    )),
                    unit: BillingPeriod::Hour,
                    quantity: capacity_dec * monthly_hours,
                    unit_price: unit_price.price,
                    monthly_cost: Money::usd(monthly_cost),
                }])
            }
            Err(e) => {
                tracing::warn!("could not query PTU price for {model_name}, reporting $0: {e}");
                Ok(vec![CostComponent {
                    name: SmolStr::new(format!(
                        "Provisioned Throughput ({model_name}, {capacity} PTUs, price unavailable)"
                    )),
                    unit: BillingPeriod::Month,
                    quantity: capacity_dec,
                    unit_price: Money::zero(),
                    monthly_cost: Money::zero(),
                }])
            }
        }
    }
}

impl ResourceCostModel for CognitiveDeploymentModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_cognitive_deployment")
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
