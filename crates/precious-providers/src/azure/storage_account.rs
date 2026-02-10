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
use std::str::FromStr;

pub struct StorageAccountModel;

impl StorageAccountModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        _pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let account_tier = resource.get_string("account_tier").unwrap_or("Standard");
        let replication_type = resource
            .get_string("account_replication_type")
            .unwrap_or("LRS");

        let per_gb = match (account_tier, replication_type) {
            ("Standard", "LRS") => "0.0184",
            ("Standard", "ZRS") => "0.023",
            ("Standard", "GRS" | "RAGRS") => "0.0368",
            ("Standard", "GZRS" | "RAGZRS") => "0.046",
            ("Premium", "LRS") => "0.15",
            ("Premium", "ZRS") => "0.1875",
            _ => "0.0184",
        };

        let unit_price_amount = Decimal::from_str(per_gb)
            .unwrap_or(Decimal::from_str("0.0184").expect("valid decimal"));

        Ok(vec![CostComponent {
            name: SmolStr::new(format!(
                "Data storage ({account_tier} {replication_type}, Hot tier)"
            )),
            unit: BillingPeriod::GBMonth,
            quantity: Decimal::ZERO,
            quantity_unit: SmolStr::new("GB/month"),
            unit_price: Money::usd(unit_price_amount),
            monthly_cost: Money::zero(),
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for StorageAccountModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_storage_account")
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
