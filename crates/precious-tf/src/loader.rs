use indexmap::IndexMap;
use precious_core::error::PreciousError;
use precious_core::state::{State, TfValue};
use smol_str::SmolStr;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

const MAX_MODULE_DEPTH: usize = 16;

pub fn load_directory(dir: &Path) -> Result<State, PreciousError> {
    let canonical = dir.canonicalize().map_err(PreciousError::Io)?;
    let mut state = State::new();
    let mut visited = HashSet::new();
    let empty_vars = IndexMap::new();
    load_recursive(&canonical, &[], &empty_vars, &mut state, &mut visited, 0)?;
    Ok(state)
}

fn load_recursive(
    dir: &Path,
    module_path: &[SmolStr],
    variables: &IndexMap<String, TfValue>,
    state: &mut State,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), PreciousError> {
    if depth > MAX_MODULE_DEPTH {
        return Err(PreciousError::HclParse(format!(
            "module nesting depth exceeded {MAX_MODULE_DEPTH} at {}",
            dir.display()
        )));
    }

    if !dir.is_dir() {
        return Err(PreciousError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("directory not found: {}", dir.display()),
        )));
    }

    let canonical = dir.canonicalize().map_err(PreciousError::Io)?;
    if !visited.insert(canonical.clone()) {
        warn!(
            "circular module reference detected at {}, skipping",
            dir.display()
        );
        return Ok(());
    }

    let mut tf_files: Vec<_> = std::fs::read_dir(dir)
        .map_err(PreciousError::Io)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tf"))
        .map(|entry| entry.path())
        .collect();

    tf_files.sort();

    let mut locals = IndexMap::new();
    let mut var_defaults = IndexMap::new();
    for path in &tf_files {
        let result = crate::parser::parse_hcl_file(path)?;
        locals.extend(result.locals.clone());
        var_defaults.extend(result.variable_defaults.clone());
    }

    let mut effective_vars = var_defaults;
    for (k, v) in variables {
        if !matches!(v, TfValue::Null | TfValue::VarRef(_)) {
            effective_vars.insert(k.clone(), v.clone());
        }
    }
    for (k, v) in &locals {
        if !matches!(v, TfValue::Null | TfValue::VarRef(_)) {
            effective_vars.insert(k.clone(), v.clone());
        }
    }

    for path in &tf_files {
        info!("parsing {}", path.display());
        let result = crate::parser::parse_hcl_file(path)?;

        for mut resource in result.resources {
            if !effective_vars.is_empty() {
                resolve_variables(&mut resource.attributes, &effective_vars);
            }
            for m in module_path.iter().rev() {
                resource.address = resource.address.with_module(m.clone());
            }
            state.add_resource(resource);
        }

        for module in &result.modules {
            if !is_local_source(&module.source) {
                debug!(
                    "skipping non-local module source: {} (module {})",
                    module.source, module.name
                );
                continue;
            }

            let module_dir = dir.join(&module.source);
            let resolved = match module_dir.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        "cannot resolve module {} source {}: {e}",
                        module.name, module.source
                    );
                    continue;
                }
            };

            let mut child_path = module_path.to_vec();
            child_path.push(module.name.clone());

            let mut child_vars = module.inputs.clone();
            if !effective_vars.is_empty() {
                resolve_variables(&mut child_vars, &effective_vars);
            }

            debug!("entering module {} at {}", module.name, resolved.display());
            load_recursive(
                &resolved,
                &child_path,
                &child_vars,
                state,
                visited,
                depth + 1,
            )?;
        }
    }

    visited.remove(&canonical);
    Ok(())
}

fn resolve_variables(attrs: &mut IndexMap<String, TfValue>, variables: &IndexMap<String, TfValue>) {
    let keys: Vec<String> = attrs.keys().cloned().collect();
    for key in keys {
        let resolved = match attrs.get(&key) {
            Some(TfValue::VarRef(var_name)) => variables
                .get(var_name.as_str())
                .filter(|v| !matches!(v, TfValue::Null | TfValue::VarRef(_)))
                .cloned(),
            Some(TfValue::Null) => variables
                .get(&key)
                .filter(|v| !matches!(v, TfValue::Null))
                .cloned(),
            Some(TfValue::Map(inner)) => {
                let mut inner = inner.clone();
                resolve_variables(&mut inner, variables);
                Some(TfValue::Map(inner))
            }
            _ => None,
        };
        if let Some(val) = resolved {
            attrs.insert(key, val);
        }
    }
}

fn is_local_source(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../")
}
