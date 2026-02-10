use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cloud {
    Aws,
    Azure,
    Gcp,
    DigitalOcean,
    Cloudflare,
    PlanetScale,
}

impl fmt::Display for Cloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cloud::Aws => write!(f, "aws"),
            Cloud::Azure => write!(f, "azurerm"),
            Cloud::Gcp => write!(f, "google"),
            Cloud::DigitalOcean => write!(f, "digitalocean"),
            Cloud::Cloudflare => write!(f, "cloudflare"),
            Cloud::PlanetScale => write!(f, "planetscale"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceTypeName(pub SmolStr);

impl fmt::Display for ResourceTypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceKind {
    pub cloud: Cloud,
    pub type_name: ResourceTypeName,
}

impl ResourceKind {
    pub fn new(cloud: Cloud, type_name: impl Into<SmolStr>) -> Self {
        Self {
            cloud,
            type_name: ResourceTypeName(type_name.into()),
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceAddress {
    pub module_path: Vec<SmolStr>,
    pub kind: ResourceKind,
    pub name: SmolStr,
}

impl ResourceAddress {
    pub fn new(kind: ResourceKind, name: impl Into<SmolStr>) -> Self {
        Self {
            module_path: Vec::new(),
            kind,
            name: name.into(),
        }
    }

    pub fn with_module(mut self, module: impl Into<SmolStr>) -> Self {
        self.module_path.push(module.into());
        self
    }
}

impl fmt::Display for ResourceAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for m in &self.module_path {
            write!(f, "module.{}.", m)?;
        }
        write!(f, "{}.{}", self.kind, self.name)
    }
}
