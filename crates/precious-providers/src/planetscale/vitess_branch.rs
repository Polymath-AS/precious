use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::BillingPeriod;
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::TfResource;
use precious_pricing::client::PricingClient;
use precious_pricing::types::{PriceFilter, PriceQuery};
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

const DEFAULT_CLUSTER_SIZE: &str = "PS-10";

pub struct VitessBranchModel;

impl VitessBranchModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let cluster_size = resource
            .get_string("cluster_size")
            .unwrap_or(DEFAULT_CLUSTER_SIZE);

        let query = PriceQuery {
            cloud: Cloud::PlanetScale,
            service: "PlanetScale Vitess".to_string(),
            region: String::new(),
            filters: vec![PriceFilter {
                field: "clusterSize".to_string(),
                value: cluster_size.to_string(),
            }],
        };

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!(
                "failed to get PlanetScale Vitess price for {cluster_size}: {e}"
            ))
        })?;

        Ok(vec![CostComponent {
            name: SmolStr::new(format!("Scaler Pro ({cluster_size})")),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            quantity_unit: SmolStr::new("months"),
            unit_price: unit_price.price,
            monthly_cost: unit_price.price,
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for VitessBranchModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::PlanetScale, "planetscale_vitess_branch")
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
