use crate::registry::Registry;
use precious_core::resource::Cloud;
use precious_pricing::client::PricingClient;

pub trait Provider: Send + Sync {
    fn cloud(&self) -> Cloud;

    fn pricing_client(&self) -> &dyn PricingClient;

    fn register(&self, registry: &mut Registry);
}
