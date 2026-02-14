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

pub struct PrivateDnsZoneModel;

impl PrivateDnsZoneModel {
    async fn do_estimate(
        &self,
        _resource: &TfResource,
        usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let query = PriceQuery::azure("Azure DNS", "")
            .filter("productName", "Azure DNS")
            .filter("skuName", "Private")
            .filter("meterName", "Private Zone");

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get Private DNS Zone price: {e}"))
        })?;

        let mut components = vec![CostComponent {
            name: SmolStr::new("Hosted zone"),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            quantity_unit: SmolStr::new("months"),
            unit_price: unit_price.price,
            monthly_cost: unit_price.price,
            quantity_max: None,
            monthly_cost_max: None,
        }];

        if let Some(queries_millions) = usage.and_then(|u| u.get_metric("dns_queries_millions")) {
            let query_price_query = PriceQuery::azure("Azure DNS", "")
                .filter("productName", "Azure DNS")
                .filter("skuName", "Private")
                .filter("meterName", "Private Queries");

            let dns_price = pricing.query_price(&query_price_query).await.map_err(|e| {
                PreciousError::PricingError(format!("failed to get Private DNS query price: {e}"))
            })?;

            let dns_cost = dns_price.price.amount * queries_millions;

            components.push(CostComponent {
                name: SmolStr::new("DNS queries (per million)"),
                unit: BillingPeriod::Month,
                quantity: queries_millions,
                quantity_unit: SmolStr::new("millions"),
                unit_price: dns_price.price,
                monthly_cost: Money::usd(dns_cost),
                quantity_max: None,
                monthly_cost_max: None,
            });
        }

        Ok(components)
    }
}

impl ResourceCostModel for PrivateDnsZoneModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_private_dns_zone")
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
