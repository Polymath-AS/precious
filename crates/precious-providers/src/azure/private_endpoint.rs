use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct PrivateEndpointModel;

impl PrivateEndpointModel {
    async fn do_estimate(
        &self,
        _resource: &TfResource,
        usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let query = PriceQuery::azure("Virtual Network", "Global")
            .filter("productName", "Virtual Network Private Link")
            .filter("meterName", "Standard Private Endpoint");

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get Private Endpoint price: {e}"))
        })?;

        let monthly_hours = BillingPeriod::hours_per_month();
        let monthly_cost = unit_price.price.amount * monthly_hours;

        let mut components = vec![CostComponent {
            name: SmolStr::new("Private Endpoint"),
            unit: BillingPeriod::Hour,
            quantity: monthly_hours,
            quantity_unit: SmolStr::new("hours"),
            unit_price: unit_price.price,
            monthly_cost: Money::usd(monthly_cost),
            quantity_max: None,
            monthly_cost_max: None,
        }];

        if let Some(inbound_gb) = usage.and_then(|u| u.get_metric("inbound_data_gb")) {
            let data_query = PriceQuery::azure("Virtual Network", "Global")
                .filter("productName", "Virtual Network Private Link")
                .filter("meterName", "Standard Data Processed - Ingress");

            let data_price = pricing.query_price(&data_query).await.map_err(|e| {
                PreciousError::PricingError(format!(
                    "failed to get Private Endpoint data processing price: {e}"
                ))
            })?;

            let data_cost = data_price.price.amount * inbound_gb;

            components.push(CostComponent {
                name: SmolStr::new("Inbound data processing"),
                unit: BillingPeriod::GBMonth,
                quantity: inbound_gb,
                quantity_unit: SmolStr::new("GB/month"),
                unit_price: data_price.price,
                monthly_cost: Money::usd(data_cost),
                quantity_max: None,
                monthly_cost_max: None,
            });
        }

        Ok(components)
    }
}

impl ResourceCostModel for PrivateEndpointModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_private_endpoint")
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
