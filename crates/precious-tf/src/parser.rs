use indexmap::IndexMap;
use precious_core::error::PreciousError;
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{TfResource, TfValue};
use smol_str::SmolStr;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TfModule {
    pub name: SmolStr,
    pub source: String,
    pub inputs: IndexMap<String, TfValue>,
}

#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub resources: Vec<TfResource>,
    pub modules: Vec<TfModule>,
    pub locals: IndexMap<String, TfValue>,
    pub variable_defaults: IndexMap<String, TfValue>,
}

pub fn parse_hcl_file(path: &Path) -> Result<ParseResult, PreciousError> {
    let content = std::fs::read_to_string(path).map_err(PreciousError::Io)?;
    parse_hcl(&content)
}

pub fn parse_hcl(content: &str) -> Result<ParseResult, PreciousError> {
    let body: hcl::Body =
        hcl::from_str(content).map_err(|e| PreciousError::HclParse(e.to_string()))?;

    let mut result = ParseResult::default();

    for block in body.blocks() {
        match block.identifier() {
            "resource" => {
                let labels: Vec<&str> = block.labels().iter().map(|l| l.as_str()).collect();
                if labels.len() < 2 {
                    tracing::warn!("resource block with insufficient labels, skipping");
                    continue;
                }

                let type_name = labels[0];
                let resource_name = labels[1];

                let cloud = detect_cloud(type_name);
                let kind = ResourceKind::new(cloud, SmolStr::new(type_name));
                let address = precious_core::resource::ResourceAddress::new(
                    kind,
                    SmolStr::new(resource_name),
                );

                let attributes = extract_attributes(block.body());

                result.resources.push(TfResource {
                    address,
                    attributes,
                });
            }
            "module" => {
                let labels: Vec<&str> = block.labels().iter().map(|l| l.as_str()).collect();
                if labels.is_empty() {
                    tracing::warn!("module block with no name label, skipping");
                    continue;
                }

                let module_name = labels[0];
                let attrs = extract_attributes(block.body());

                let source = attrs
                    .get("source")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let Some(source) = source else {
                    tracing::warn!("module {module_name} has no source attribute, skipping");
                    continue;
                };

                let mut inputs = attrs;
                inputs.swap_remove("source");

                result.modules.push(TfModule {
                    name: SmolStr::new(module_name),
                    source,
                    inputs,
                });
            }
            "locals" => {
                let attrs = extract_attributes(block.body());
                result.locals.extend(attrs);
            }
            "variable" => {
                let labels: Vec<&str> = block.labels().iter().map(|l| l.as_str()).collect();
                if let Some(var_name) = labels.first() {
                    let attrs = extract_attributes(block.body());
                    if let Some(default) = attrs.get("default") {
                        if !matches!(default, TfValue::Null) {
                            result
                                .variable_defaults
                                .insert(var_name.to_string(), default.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(result)
}

fn detect_cloud(type_name: &str) -> Cloud {
    if type_name.starts_with("aws_") {
        Cloud::Aws
    } else if type_name.starts_with("azurerm_") {
        Cloud::Azure
    } else if type_name.starts_with("google_") {
        Cloud::Gcp
    } else if type_name.starts_with("digitalocean_") {
        Cloud::DigitalOcean
    } else if type_name.starts_with("cloudflare_") {
        Cloud::Cloudflare
    } else if type_name.starts_with("planetscale_") {
        Cloud::PlanetScale
    } else {
        Cloud::Aws
    }
}

fn extract_attributes(body: &hcl::Body) -> IndexMap<String, TfValue> {
    let mut attrs = IndexMap::new();
    for attr in body.attributes() {
        let key = attr.key().to_string();
        let value = expression_to_value(attr.expr());
        attrs.insert(key, value);
    }
    for block in body.blocks() {
        let key = block.identifier().to_string();
        let nested = extract_attributes(block.body());
        attrs.insert(key, TfValue::Map(nested));
    }
    attrs
}

fn expression_to_value(expr: &hcl::Expression) -> TfValue {
    match expr {
        hcl::Expression::String(s) => TfValue::String(s.clone()),
        hcl::Expression::Number(n) => {
            if let Some(f) = n.as_f64() {
                TfValue::Number(f)
            } else {
                TfValue::Null
            }
        }
        hcl::Expression::Bool(b) => TfValue::Bool(*b),
        hcl::Expression::Null => TfValue::Null,
        hcl::Expression::Array(arr) => TfValue::List(arr.iter().map(expression_to_value).collect()),
        hcl::Expression::Object(obj) => {
            let map: IndexMap<String, TfValue> = obj
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        hcl::expr::ObjectKey::Identifier(id) => id.to_string(),
                        hcl::expr::ObjectKey::Expression(e) => match e {
                            hcl::Expression::String(s) => s.clone(),
                            _ => format!("{e:?}"),
                        },
                        _ => format!("{k:?}"),
                    };
                    (key, expression_to_value(v))
                })
                .collect();
            TfValue::Map(map)
        }
        hcl::Expression::Variable(v) => TfValue::VarRef(v.to_string()),
        hcl::Expression::Traversal(t) => {
            if let hcl::Expression::Variable(v) = &t.expr {
                let root = v.to_string();
                if (root == "var" || root == "local") && t.operators.len() == 1 {
                    if let hcl::expr::TraversalOperator::GetAttr(attr) = &t.operators[0] {
                        return TfValue::VarRef(attr.to_string());
                    }
                }
            }
            tracing::debug!("unresolved traversal: {:?}, treating as null", expr);
            TfValue::Null
        }
        _ => {
            tracing::debug!("unresolved expression: {:?}, treating as null", expr);
            TfValue::Null
        }
    }
}
