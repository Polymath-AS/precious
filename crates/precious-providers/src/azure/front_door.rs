use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::TfResource;
use precious_pricing::client::PricingClient;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct FrontDoorProfileModel;

impl FrontDoorProfileModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        _pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let sku_name =
            resource
                .get_string("sku_name")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "sku_name".to_string(),
                })?;

        let (tier, monthly_amount) = if sku_name.starts_with("Standard") {
            ("Standard", Decimal::new(3500, 2))
        } else if sku_name.starts_with("Premium") {
            ("Premium", Decimal::new(33000, 2))
        } else {
            return Err(PreciousError::InvalidField {
                resource: resource.address.to_string(),
                field: "sku_name".to_string(),
                reason: format!("unknown Front Door SKU: {sku_name}"),
            });
        };

        let unit_price = Money::usd(monthly_amount);

        Ok(vec![CostComponent {
            name: SmolStr::new(format!("Front Door profile ({tier})")),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            quantity_unit: SmolStr::new("months"),
            unit_price,
            monthly_cost: unit_price,
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for FrontDoorProfileModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_cdn_frontdoor_profile")
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
