use crate::provider::Provider;
use crate::registry::Registry;
use precious_core::resource::Cloud;
use precious_pricing::client::{PricingClient, StaticPricingClient};

pub struct GcpProvider {
    pricing: StaticPricingClient,
}

impl GcpProvider {
    pub fn new() -> Self {
        Self {
            pricing: StaticPricingClient::new(vec![]),
        }
    }
}

impl Default for GcpProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for GcpProvider {
    fn cloud(&self) -> Cloud {
        Cloud::Gcp
    }

    fn pricing_client(&self) -> &dyn PricingClient {
        &self.pricing
    }

    fn register(&self, _registry: &mut Registry) {
        // TODO: register GCP resource cost models
    }
}
