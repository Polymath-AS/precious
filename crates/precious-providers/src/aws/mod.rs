pub mod instance;

use crate::provider::Provider;
use crate::registry::Registry;
use precious_core::resource::Cloud;
use precious_pricing::client::{AwsPricingClient, PricingClient};

pub struct AwsProvider {
    pricing: AwsPricingClient,
}

impl AwsProvider {
    pub fn new() -> Self {
        Self {
            pricing: AwsPricingClient::new(),
        }
    }
}

impl Default for AwsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for AwsProvider {
    fn cloud(&self) -> Cloud {
        Cloud::Aws
    }

    fn pricing_client(&self) -> &dyn PricingClient {
        &self.pricing
    }

    fn register(&self, registry: &mut Registry) {
        registry.register(Box::new(instance::AwsInstanceModel));
    }
}
