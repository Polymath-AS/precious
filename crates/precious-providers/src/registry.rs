use crate::usage::UsageEntry;
use indexmap::IndexMap;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::resource::ResourceKind;
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

pub trait ResourceCostModel: Send + Sync {
    fn kind(&self) -> ResourceKind;

    fn estimate<'a>(
        &'a self,
        resource: &'a TfResource,
        usage: Option<&'a UsageEntry>,
        pricing: &'a dyn PricingClient,
        state: &'a State,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CostComponent>, PreciousError>> + Send + 'a>>;
}

pub struct Registry {
    models: IndexMap<String, Box<dyn ResourceCostModel>>,
    free: HashSet<&'static str>,
}

impl Registry {
    pub fn new() -> Self {
        let free = HashSet::from([
            "random_id",
            "random_integer",
            "random_password",
            "random_pet",
            "random_shuffle",
            "random_string",
            "random_uuid",
            "time_offset",
            "time_rotating",
            "time_sleep",
            "time_static",
            "null_resource",
            "terraform_data",
        ]);

        Self {
            models: IndexMap::new(),
            free,
        }
    }

    pub fn register(&mut self, model: Box<dyn ResourceCostModel>) {
        let key = model.kind().type_name.0.to_string();
        self.models.insert(key, model);
    }

    pub fn register_free(&mut self, type_names: &[&'static str]) {
        self.free.extend(type_names);
    }

    pub fn get(&self, type_name: &str) -> Option<&dyn ResourceCostModel> {
        self.models.get(type_name).map(|m| m.as_ref())
    }

    pub fn is_free(&self, type_name: &str) -> bool {
        self.free.contains(type_name)
    }

    pub fn supported_types(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
