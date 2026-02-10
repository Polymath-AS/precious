use crate::OutputFormat;
use crate::output;
use miette::{Result, miette};
use precious_core::cost::{
    Breakdown, Diff, MultiBreakdown, MultiDiff, ProjectBreakdown, ProjectDiff,
};
use precious_providers::aws::AwsProvider;
use precious_providers::azure::AzureProvider;
use precious_providers::engine::{Engine, UnsupportedBehavior};
use precious_providers::gcp::GcpProvider;
use precious_providers::provider::Provider;
use precious_providers::usage;
use precious_tf::loader;
use std::path::Path;
use tracing::info;

fn build_engine() -> Engine {
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(AwsProvider::new()),
        Box::new(AzureProvider::new()),
        Box::new(GcpProvider::new()),
    ];
    Engine::new(providers).with_unsupported_behavior(UnsupportedBehavior::Warn)
}

pub async fn breakdown(
    path: &str,
    usage_file: Option<&str>,
    reverse: bool,
    max_search_depth: usize,
    format: &OutputFormat,
) -> Result<()> {
    let dir = Path::new(path);

    let usage = match usage_file {
        Some(p) => Some(usage::load_usage_file(Path::new(p)).map_err(|e| miette!("{e}"))?),
        None => None,
    };

    let engine = build_engine();

    if loader::dir_has_tf_files(dir) {
        let state = loader::load_directory(dir).map_err(|e| miette!("{e}"))?;
        let mut breakdown = engine
            .estimate(&state, usage.as_ref())
            .await
            .map_err(|e| miette!("{e}"))?;
        breakdown.sort(reverse);

        match format {
            OutputFormat::Table => output::print_breakdown_table(&breakdown),
            OutputFormat::Json => output::print_json(&breakdown)?,
        }
    } else {
        let roots = loader::discover_root_modules(dir, max_search_depth)
            .map_err(|e| miette!("{e}"))?;

        if roots.is_empty() {
            return Err(miette!("no Terraform files found under {}", dir.display()));
        }

        info!("auto-detected {} root module(s)", roots.len());

        let base = dir.canonicalize().map_err(|e| miette!("{e}"))?;
        let mut projects = Vec::with_capacity(roots.len());

        for root in &roots {
            let state = loader::load_directory(root).map_err(|e| miette!("{e}"))?;
            let breakdown = engine
                .estimate(&state, usage.as_ref())
                .await
                .map_err(|e| miette!("{e}"))?;

            let rel_path = root
                .strip_prefix(&base)
                .unwrap_or(root)
                .to_path_buf();

            projects.push(ProjectBreakdown {
                path: rel_path,
                breakdown,
            });
        }

        let mut multi = MultiBreakdown::new(projects);
        multi.sort(reverse);

        match format {
            OutputFormat::Table => output::print_multi_breakdown_table(&multi),
            OutputFormat::Json => output::print_json(&multi)?,
        }
    }

    Ok(())
}

pub async fn diff(
    path: &str,
    compare_to: &str,
    max_search_depth: usize,
    format: &OutputFormat,
) -> Result<()> {
    let dir = Path::new(path);
    let engine = build_engine();

    if loader::dir_has_tf_files(dir) {
        let before_json = std::fs::read_to_string(compare_to)
            .map_err(|e| miette!("failed to read baseline: {e}"))?;
        let before: Breakdown = serde_json::from_str(&before_json)
            .map_err(|e| miette!("failed to parse baseline: {e}"))?;

        let state = loader::load_directory(dir).map_err(|e| miette!("{e}"))?;
        let after = engine
            .estimate(&state, None)
            .await
            .map_err(|e| miette!("{e}"))?;

        let diff = Diff::compute(&before, &after);

        match format {
            OutputFormat::Table => output::print_diff_table(&diff),
            OutputFormat::Json => output::print_json(&diff)?,
        }
    } else {
        let before_json = std::fs::read_to_string(compare_to)
            .map_err(|e| miette!("failed to read baseline: {e}"))?;
        let before: MultiBreakdown = serde_json::from_str(&before_json)
            .map_err(|e| miette!("failed to parse baseline: {e}"))?;

        let roots = loader::discover_root_modules(dir, max_search_depth)
            .map_err(|e| miette!("{e}"))?;

        if roots.is_empty() {
            return Err(miette!("no Terraform files found under {}", dir.display()));
        }

        info!("auto-detected {} root module(s)", roots.len());

        let base = dir.canonicalize().map_err(|e| miette!("{e}"))?;
        let mut project_diffs = Vec::new();

        for root in &roots {
            let state = loader::load_directory(root).map_err(|e| miette!("{e}"))?;
            let after = engine
                .estimate(&state, None)
                .await
                .map_err(|e| miette!("{e}"))?;

            let rel_path = root
                .strip_prefix(&base)
                .unwrap_or(root)
                .to_path_buf();

            let before_breakdown = before
                .projects
                .iter()
                .find(|p| p.path == rel_path)
                .map(|p| &p.breakdown)
                .cloned()
                .unwrap_or_else(|| Breakdown::new(Vec::new()));

            let diff = Diff::compute(&before_breakdown, &after);
            project_diffs.push(ProjectDiff {
                path: rel_path,
                diff,
            });
        }

        let multi_diff = MultiDiff::new(project_diffs);

        match format {
            OutputFormat::Table => output::print_multi_diff_table(&multi_diff),
            OutputFormat::Json => output::print_json(&multi_diff)?,
        }
    }

    Ok(())
}
