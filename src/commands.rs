use crate::OutputFormat;
use crate::output;
use miette::{Result, miette};
use precious_core::cost::{Breakdown, Diff};
use precious_providers::aws::AwsProvider;
use precious_providers::azure::AzureProvider;
use precious_providers::engine::{Engine, UnsupportedBehavior};
use precious_providers::gcp::GcpProvider;
use precious_providers::provider::Provider;
use precious_providers::usage;
use std::path::Path;

fn build_engine() -> Engine {
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(AwsProvider::new()),
        Box::new(AzureProvider::new()),
        Box::new(GcpProvider::new()),
    ];
    Engine::new(providers).with_unsupported_behavior(UnsupportedBehavior::Warn)
}

pub async fn breakdown(path: &str, usage_file: Option<&str>, reverse: bool, format: &OutputFormat) -> Result<()> {
    let state = precious_tf::loader::load_directory(Path::new(path)).map_err(|e| miette!("{e}"))?;

    let usage = match usage_file {
        Some(p) => Some(usage::load_usage_file(Path::new(p)).map_err(|e| miette!("{e}"))?),
        None => None,
    };

    let engine = build_engine();

    let mut breakdown = engine
        .estimate(&state, usage.as_ref())
        .await
        .map_err(|e| miette!("{e}"))?;

    breakdown.sort(reverse);

    match format {
        OutputFormat::Table => output::print_breakdown_table(&breakdown),
        OutputFormat::Json => output::print_json(&breakdown)?,
    }

    Ok(())
}

pub async fn diff(path: &str, compare_to: &str, format: &OutputFormat) -> Result<()> {
    let before_json =
        std::fs::read_to_string(compare_to).map_err(|e| miette!("failed to read baseline: {e}"))?;
    let before: Breakdown =
        serde_json::from_str(&before_json).map_err(|e| miette!("failed to parse baseline: {e}"))?;

    let state = precious_tf::loader::load_directory(Path::new(path)).map_err(|e| miette!("{e}"))?;

    let engine = build_engine();

    let after = engine
        .estimate(&state, None)
        .await
        .map_err(|e| miette!("{e}"))?;

    let diff = Diff::compute(&before, &after);

    match format {
        OutputFormat::Table => output::print_diff_table(&diff),
        OutputFormat::Json => output::print_json(&diff)?,
    }

    Ok(())
}
