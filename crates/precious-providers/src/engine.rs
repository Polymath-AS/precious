use crate::provider::Provider;
use crate::registry::Registry;
use crate::usage::{UsageFile, find_usage};
use precious_core::cost::{Breakdown, ResourceCost};
use precious_core::error::PreciousError;
use precious_core::resource::Cloud;
use precious_core::state::{State, TfResource, TfValue};
use precious_pricing::client::PricingClient;
use tracing::{info, warn};

pub enum UnsupportedBehavior {
    Error,
    Skip,
    Warn,
}

pub struct Engine {
    registry: Registry,
    providers: Vec<Box<dyn Provider>>,
    unsupported: UnsupportedBehavior,
}

impl Engine {
    pub fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        let mut registry = Registry::new();
        for p in &providers {
            p.register(&mut registry);
        }
        Self {
            registry,
            providers,
            unsupported: UnsupportedBehavior::Warn,
        }
    }

    pub fn with_unsupported_behavior(mut self, behavior: UnsupportedBehavior) -> Self {
        self.unsupported = behavior;
        self
    }

    fn pricing_client_for(&self, cloud: Cloud) -> Option<&dyn PricingClient> {
        self.providers
            .iter()
            .find(|p| p.cloud() == cloud)
            .map(|p| p.pricing_client())
    }

    pub async fn estimate(
        &self,
        state: &State,
        usage: Option<&UsageFile>,
    ) -> Result<Breakdown, PreciousError> {
        let mut resource_costs = Vec::new();

        for (addr_str, resource) in &state.resources {
            if is_count_zero(resource) {
                tracing::debug!("skipping {addr_str}: count = 0");
                continue;
            }

            let type_name = resource.address.kind.type_name.0.as_str();

            if self.registry.is_free(type_name) {
                tracing::debug!("skipping free resource: {type_name}");
                continue;
            }

            let model = match self.registry.get(type_name) {
                Some(m) => m,
                None => match self.unsupported {
                    UnsupportedBehavior::Error => {
                        return Err(PreciousError::UnsupportedResource(type_name.to_string()));
                    }
                    UnsupportedBehavior::Skip => continue,
                    UnsupportedBehavior::Warn => {
                        warn!("unsupported resource type: {type_name}, skipping");
                        continue;
                    }
                },
            };

            let cloud = resource.address.kind.cloud;
            let pricing = self.pricing_client_for(cloud).ok_or_else(|| {
                PreciousError::PricingError(format!("no provider registered for cloud {cloud}"))
            })?;

            let usage_entry = usage.and_then(|u| find_usage(u, addr_str));
            let components = model.estimate(resource, usage_entry, pricing, state).await?;

            let cost = ResourceCost::new(resource.address.clone(), type_name, components);

            info!("{}: {}", addr_str, cost.monthly_total);
            resource_costs.push(cost);
        }

        Ok(Breakdown::new(resource_costs))
    }
}

fn is_count_zero(resource: &TfResource) -> bool {
    match resource.attributes.get("count") {
        Some(TfValue::Number(n)) => *n == 0.0,
        Some(TfValue::Bool(false)) => true,
        _ => false,
    }
}
