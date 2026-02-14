use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct LogAnalyticsWorkspaceModel;

impl LogAnalyticsWorkspaceModel {
    async fn do_estimate(
        &self,
        _resource: &TfResource,
        _usage: Option<&UsageEntry>,
        _pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let per_gb_price = Decimal::new(276, 2);

        Ok(vec![CostComponent {
            name: SmolStr::new("Data ingestion (pay-as-you-go)"),
            unit: BillingPeriod::GBMonth,
            quantity: Decimal::ZERO,
            quantity_unit: SmolStr::new("GB/month"),
            unit_price: Money::usd(per_gb_price),
            monthly_cost: Money::zero(),
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for LogAnalyticsWorkspaceModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_log_analytics_workspace")
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
