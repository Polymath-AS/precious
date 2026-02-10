use indexmap::IndexMap;
use precious_core::error::PreciousError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageFile {
    pub version: u32,
    pub resources: Vec<UsageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub address: String,
    #[serde(flatten)]
    pub metrics: IndexMap<String, Decimal>,
}

impl UsageEntry {
    pub fn get_metric(&self, name: &str) -> Option<Decimal> {
        self.metrics.get(name).copied()
    }
}

pub fn load_usage_file(path: &Path) -> Result<UsageFile, PreciousError> {
    let content = std::fs::read_to_string(path).map_err(PreciousError::Io)?;
    serde_yaml::from_str(&content)
        .map_err(|e| PreciousError::Serialization(format!("failed to parse usage file: {e}")))
}

pub fn find_usage<'a>(usage: &'a UsageFile, address: &str) -> Option<&'a UsageEntry> {
    usage.resources.iter().find(|e| e.address == address)
}
