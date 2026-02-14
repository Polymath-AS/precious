use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource, TfValue};
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct FrontDoorWafModel;

fn count_blocks(resource: &TfResource, block_name: &str) -> Decimal {
    match resource.attributes.get(block_name) {
        Some(TfValue::Map(_)) => Decimal::ONE,
        Some(TfValue::List(items)) => Decimal::from(items.len() as u32),
        _ => Decimal::ZERO,
    }
}

impl FrontDoorWafModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let mut components = Vec::with_capacity(3);

        let policy_query = PriceQuery::azure("Azure Front Door Service", "")
            .filter("productName", "Azure Front Door Service")
            .filter("meterName", "Standard Policy");

        let policy_price = pricing.query_price(&policy_query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get WAF policy price: {e}"))
        })?;

        components.push(CostComponent {
            name: SmolStr::new("WAF policy"),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            quantity_unit: SmolStr::new("months"),
            unit_price: policy_price.price,
            monthly_cost: policy_price.price,
            quantity_max: None,
            monthly_cost_max: None,
        });

        let custom_rule_count = count_blocks(resource, "custom_rule");
        if custom_rule_count > Decimal::ZERO {
            let rule_query = PriceQuery::azure("Azure Front Door Service", "")
                .filter("productName", "Azure Front Door Service")
                .filter("meterName", "Standard Rule");

            let rule_price = pricing.query_price(&rule_query).await.map_err(|e| {
                PreciousError::PricingError(format!("failed to get WAF custom rule price: {e}"))
            })?;

            let rule_cost = rule_price.price.amount * custom_rule_count;

            components.push(CostComponent {
                name: SmolStr::new("Custom rules"),
                unit: BillingPeriod::Month,
                quantity: custom_rule_count,
                quantity_unit: SmolStr::new("rules"),
                unit_price: rule_price.price,
                monthly_cost: Money::usd(rule_cost),
                quantity_max: None,
                monthly_cost_max: None,
            });
        }

        let managed_rule_count = count_blocks(resource, "managed_rule");
        if managed_rule_count > Decimal::ZERO {
            let managed_query = PriceQuery::azure("Azure Front Door Service", "")
                .filter("productName", "Azure Front Door Service")
                .filter("meterName", "Standard Default Ruleset");

            let managed_price = pricing.query_price(&managed_query).await.map_err(|e| {
                PreciousError::PricingError(format!("failed to get WAF managed ruleset price: {e}"))
            })?;

            let managed_cost = managed_price.price.amount * managed_rule_count;

            components.push(CostComponent {
                name: SmolStr::new("Managed rule sets"),
                unit: BillingPeriod::Month,
                quantity: managed_rule_count,
                quantity_unit: SmolStr::new("rule sets"),
                unit_price: managed_price.price,
                monthly_cost: Money::usd(managed_cost),
                quantity_max: None,
                monthly_cost_max: None,
            });
        }

        Ok(components)
    }
}

impl ResourceCostModel for FrontDoorWafModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_cdn_frontdoor_firewall_policy")
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
