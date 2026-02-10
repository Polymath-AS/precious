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

pub struct SecurityCenterPricingModel;

impl SecurityCenterPricingModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let tier = resource.get_string("tier").unwrap_or("Free");

        if tier == "Free" {
            return Ok(vec![]);
        }

        let resource_type =
            resource
                .get_string("resource_type")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "resource_type".to_string(),
                })?;

        let (service_name, product_name, meter_name) = match resource_type {
            "VirtualMachines" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for Servers",
                "Standard Node",
            ),
            "StorageAccounts" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for Storage",
                "Standard Node",
            ),
            "AppServices" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for App Service",
                "Standard Node",
            ),
            "SqlServers" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for SQL",
                "Standard Node",
            ),
            "KeyVaults" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for Key Vault",
                "Per node Std Node",
            ),
            "Arm" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for Resource Manager",
                "Standard API Calls",
            ),
            "Dns" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for DNS",
                "Standard Queries",
            ),
            "Containers" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for Containers",
                "Standard vCore vCore Pack",
            ),
            "OpenSourceRelationalDatabases" => (
                "Microsoft Defender for Cloud",
                "Microsoft Defender for PostgreSQL",
                "Standard Node",
            ),
            _ => {
                tracing::warn!(
                    "unknown Defender plan resource_type: {resource_type}, skipping pricing"
                );
                return Ok(vec![CostComponent {
                    name: SmolStr::new(format!("Defender for {resource_type} (price unavailable)")),
                    unit: BillingPeriod::Month,
                    quantity: Decimal::ONE,
                    quantity_unit: SmolStr::new("months"),
                    unit_price: Money::zero(),
                    monthly_cost: Money::zero(),
                    quantity_max: None,
                    monthly_cost_max: None,
                }]);
            }
        };

        let query = PriceQuery::azure(service_name, "")
            .filter("productName", product_name)
            .filter("meterName", meter_name);

        let quantity = usage
            .and_then(|u| u.get_metric("resource_count"))
            .unwrap_or(Decimal::ONE);

        match pricing.query_price(&query).await {
            Ok(unit_price) => {
                let monthly_cost = unit_price.price.amount * quantity;

                Ok(vec![CostComponent {
                    name: SmolStr::new(format!("Defender for {resource_type}")),
                    unit: BillingPeriod::Month,
                    quantity,
                    quantity_unit: SmolStr::new("months"),
                    unit_price: unit_price.price,
                    monthly_cost: Money::usd(monthly_cost),
                    quantity_max: None,
                    monthly_cost_max: None,
                }])
            }
            Err(e) => {
                tracing::warn!("could not query Defender price for {resource_type}: {e}");
                Ok(vec![CostComponent {
                    name: SmolStr::new(format!("Defender for {resource_type} (price unavailable)")),
                    unit: BillingPeriod::Month,
                    quantity,
                    quantity_unit: SmolStr::new("months"),
                    unit_price: Money::zero(),
                    monthly_cost: Money::zero(),
                    quantity_max: None,
                    monthly_cost_max: None,
                }])
            }
        }
    }
}

impl ResourceCostModel for SecurityCenterPricingModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_security_center_subscription_pricing")
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
