use crate::resource::ResourceAddress;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TfAttribute {
    pub key: String,
    pub value: TfValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TfValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<TfValue>),
    Map(IndexMap<String, TfValue>),
    /// Unresolved variable reference (e.g. `var.backend_cpu` → `VarRef("backend_cpu")`)
    VarRef(String),
    Null,
}

impl TfValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            TfValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TfValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TfValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TfResource {
    pub address: ResourceAddress,
    pub attributes: IndexMap<String, TfValue>,
}

impl TfResource {
    pub fn get(&self, key: &str) -> Option<&TfValue> {
        self.attributes.get(key)
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(TfValue::as_str)
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(TfValue::as_f64)
    }

    pub fn get_nested(&self, path: &[&str]) -> Option<&TfValue> {
        let first = *path.first()?;
        let mut current = self.attributes.get(first)?;
        for key in &path[1..] {
            match current {
                TfValue::Map(m) => current = m.get(*key)?,
                _ => return None,
            }
        }
        Some(current)
    }

    pub fn get_nested_str(&self, path: &[&str]) -> Option<&str> {
        self.get_nested(path).and_then(TfValue::as_str)
    }

    pub fn get_nested_f64(&self, path: &[&str]) -> Option<f64> {
        self.get_nested(path).and_then(TfValue::as_f64)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub resources: IndexMap<String, TfResource>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_resource(&mut self, resource: TfResource) {
        let key = resource.address.to_string();
        self.resources.insert(key, resource);
    }
}
