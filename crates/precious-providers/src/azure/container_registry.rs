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

pub struct ContainerRegistryModel;

impl ContainerRegistryModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        _pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let sku = resource
            .get_string("sku")
            .ok_or_else(|| PreciousError::MissingField {
                resource: resource.address.to_string(),
                field: "sku".to_string(),
            })?;

        let monthly_amount = match sku {
            "Basic" => Decimal::new(500, 2),
            "Standard" => Decimal::new(2000, 2),
            "Premium" => Decimal::new(5000, 2),
            other => {
                return Err(PreciousError::InvalidField {
                    resource: resource.address.to_string(),
                    field: "sku".to_string(),
                    reason: format!("unknown container registry SKU: {other}"),
                });
            }
        };

        let unit_price = Money::usd(monthly_amount);

        Ok(vec![CostComponent {
            name: SmolStr::new(format!("Registry unit ({sku})")),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            unit_price,
            monthly_cost: unit_price,
        }])
    }
}

impl ResourceCostModel for ContainerRegistryModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_container_registry")
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
