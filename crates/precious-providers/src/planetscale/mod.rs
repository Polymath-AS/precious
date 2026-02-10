pub mod postgres_branch;
pub mod vitess_branch;

use crate::provider::Provider;
use crate::registry::Registry;
use precious_core::resource::Cloud;
use precious_pricing::client::{PlanetScalePricingClient, PricingClient};

pub struct PlanetScaleProvider {
    pricing: PlanetScalePricingClient,
}

impl PlanetScaleProvider {
    pub fn new() -> Self {
        Self {
            pricing: PlanetScalePricingClient::new(),
        }
    }
}

impl Default for PlanetScaleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for PlanetScaleProvider {
    fn cloud(&self) -> Cloud {
        Cloud::PlanetScale
    }

    fn pricing_client(&self) -> &dyn PricingClient {
        &self.pricing
    }

    fn register(&self, registry: &mut Registry) {
        registry.register(Box::new(postgres_branch::PostgresBranchModel));
        registry.register(Box::new(vitess_branch::VitessBranchModel));
    }
}
