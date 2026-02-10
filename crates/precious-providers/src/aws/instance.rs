use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::TfResource;
use precious_pricing::client::PricingClient;
use precious_pricing::types::{PriceFilter, PriceQuery};
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct AwsInstanceModel;

impl AwsInstanceModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let instance_type =
            resource
                .get_string("instance_type")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "instance_type".to_string(),
                })?;

        let region = resource.get_string("region").unwrap_or("us-east-1");

        let query = PriceQuery {
            cloud: Cloud::Aws,
            service: "AmazonEC2".to_string(),
            region: region.to_string(),
            filters: vec![
                PriceFilter {
                    field: "instanceType".to_string(),
                    value: instance_type.to_string(),
                },
                PriceFilter {
                    field: "operatingSystem".to_string(),
                    value: "Linux".to_string(),
                },
                PriceFilter {
                    field: "tenancy".to_string(),
                    value: "Shared".to_string(),
                },
                PriceFilter {
                    field: "preInstalledSw".to_string(),
                    value: "NA".to_string(),
                },
            ],
        };

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!("failed to get price for {instance_type}: {e}"))
        })?;

        let hourly_rate = unit_price.price.amount;
        let monthly_hours = BillingPeriod::hours_per_month();
        let monthly_cost = hourly_rate * monthly_hours;

        Ok(vec![CostComponent {
            name: SmolStr::new(format!("Linux/UNIX usage ({instance_type})")),
            unit: BillingPeriod::Hour,
            quantity: monthly_hours,
            unit_price: unit_price.price,
            monthly_cost: Money::usd(monthly_cost),
        }])
    }
}

impl ResourceCostModel for AwsInstanceModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Aws, "aws_instance")
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
