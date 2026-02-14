use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use precious_pricing::types::{PriceFilter, PriceQuery};
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

const DEFAULT_CLUSTER_SIZE: &str = "PS-10";
const INCLUDED_STORAGE_GB: u32 = 10;
const HA_NODE_COUNT: u32 = 3;

pub struct PostgresBranchModel;

impl PostgresBranchModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let cluster_size = resource
            .get_string("cluster_size")
            .unwrap_or(DEFAULT_CLUSTER_SIZE);

        let query = PriceQuery {
            cloud: Cloud::PlanetScale,
            service: "PlanetScale Postgres".to_string(),
            region: String::new(),
            filters: vec![PriceFilter {
                field: "clusterSize".to_string(),
                value: cluster_size.to_string(),
            }],
        };

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!(
                "failed to get PlanetScale Postgres price for {cluster_size}: {e}"
            ))
        })?;

        let mut components = vec![CostComponent {
            name: SmolStr::new(format!("Cluster ({cluster_size}, {HA_NODE_COUNT}-node HA)")),
            unit: BillingPeriod::Month,
            quantity: Decimal::ONE,
            quantity_unit: SmolStr::new("months"),
            unit_price: unit_price.price,
            monthly_cost: unit_price.price,
            quantity_max: None,
            monthly_cost_max: None,
        }];

        let extra_storage_gb = usage
            .and_then(|u| u.get_metric("storage_gb"))
            .and_then(|total| {
                let included = Decimal::from(INCLUDED_STORAGE_GB * HA_NODE_COUNT);
                let overage = total - included;
                if overage > Decimal::ZERO {
                    Some(overage)
                } else {
                    None
                }
            });

        if let Some(overage_gb) = extra_storage_gb {
            let storage_query = PriceQuery {
                cloud: Cloud::PlanetScale,
                service: "PlanetScale Storage".to_string(),
                region: String::new(),
                filters: vec![],
            };

            let storage_price = pricing.query_price(&storage_query).await.map_err(|e| {
                PreciousError::PricingError(format!("failed to get PlanetScale storage price: {e}"))
            })?;

            let storage_cost = storage_price.price.amount * overage_gb;

            components.push(CostComponent {
                name: SmolStr::new("Storage (overage)"),
                unit: BillingPeriod::GBMonth,
                quantity: overage_gb,
                quantity_unit: SmolStr::new("GB/month"),
                unit_price: storage_price.price,
                monthly_cost: Money::usd(storage_cost),
                quantity_max: None,
                monthly_cost_max: None,
            });
        }

        Ok(components)
    }
}

impl ResourceCostModel for PostgresBranchModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::PlanetScale, "planetscale_postgres_branch")
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
