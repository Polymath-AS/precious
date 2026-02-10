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

/// Check whether a directory directly contains any `.tf` files (non-recursive).
pub fn dir_has_tf_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|e| e.path().extension().is_some_and(|ext| ext == "tf"))
        })
        .unwrap_or(false)
}

/// Discover root Terraform modules under `base`.
///
/// Walks directories up to `max_depth`, finds all dirs containing `.tf` files,
/// parses module blocks to identify child modules, and returns only root modules
/// (those not referenced as a local module source by any other module).
pub fn discover_root_modules(
    base: &Path,
    max_depth: usize,
) -> Result<Vec<PathBuf>, PreciousError> {
    let base = base.canonicalize().map_err(PreciousError::Io)?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut child_modules: HashSet<PathBuf> = HashSet::new();

    collect_tf_dirs(&base, 0, max_depth, &mut candidates)?;

    for dir in &candidates {
        let tf_files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(PreciousError::Io)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "tf"))
            .map(|e| e.path())
            .collect();

        for path in &tf_files {
            let result = match crate::parser::parse_hcl_file(path) {
                Ok(r) => r,
                Err(e) => {
                    warn!("failed to parse {} during discovery: {e}", path.display());
                    continue;
                }
            };

            for module in &result.modules {
                if !is_local_source(&module.source) {
                    continue;
                }
                let module_dir = dir.join(&module.source);
                if let Ok(resolved) = module_dir.canonicalize() {
                    child_modules.insert(resolved);
                }
            }
        }
    }

    let mut roots: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|dir| !child_modules.contains(dir))
        .collect();
    roots.sort();
    Ok(roots)
}

fn collect_tf_dirs(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    candidates: &mut Vec<PathBuf>,
) -> Result<(), PreciousError> {
    if depth > max_depth {
        return Ok(());
    }

    if dir_has_tf_files(dir) {
        candidates.push(dir.to_path_buf());
    }

    let entries = std::fs::read_dir(dir).map_err(PreciousError::Io)?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        collect_tf_dirs(&path, depth + 1, max_depth, candidates)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn flat_repo_single_root() {
        let dir = fixture("flat_repo");
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], dir.canonicalize().unwrap());
    }

    #[test]
    fn multi_root_no_cross_refs() {
        let dir = fixture("multi_root");
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert_eq!(roots.len(), 2);
        let names: Vec<String> = roots
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"infra".to_string()));
        assert!(names.contains(&"legacy".to_string()));
    }

    #[test]
    fn root_with_child_module_excluded() {
        let dir = fixture("with_child_module");
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], dir.canonicalize().unwrap());
    }

    #[test]
    fn hidden_dirs_skipped() {
        let dir = fixture("hidden_dir");
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], dir.canonicalize().unwrap());
    }

    #[test]
    fn max_depth_respected() {
        let dir = fixture("deep_nested");
        let roots = discover_root_modules(&dir, 1).unwrap();
        assert!(roots.is_empty());
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert_eq!(roots.len(), 1);
    }

    #[test]
    fn empty_dir_no_roots() {
        let dir = fixture("empty_dir");
        let roots = discover_root_modules(&dir, 10).unwrap();
        assert!(roots.is_empty());
    }

    #[test]
    fn dir_has_tf_files_true() {
        assert!(dir_has_tf_files(&fixture("flat_repo")));
    }

    #[test]
    fn dir_has_tf_files_false() {
        assert!(!dir_has_tf_files(&fixture("no_tf")));
    }
}
