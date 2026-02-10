mod commands;
mod output;

use clap::Parser;
use miette::Result;

#[derive(Parser)]
#[command(
    name = "precious",
    about = "My precious... cloud cost estimator for Terraform",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Show cost breakdown for Terraform resources
    Breakdown {
        /// Path to Terraform directory
        #[arg(short, long, default_value = ".")]
        path: String,

        /// Path to usage file
        #[arg(long)]
        usage_file: Option<String>,

        /// Sort most expensive last (default); use --reverse for most expensive first
        #[arg(long)]
        reverse: bool,

        /// Max directory depth for auto-detecting root modules
        #[arg(long, default_value = "10")]
        max_search_depth: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
    /// Show cost diff between two states
    Diff {
        /// Path to current Terraform directory
        #[arg(short, long)]
        path: String,

        /// Path to baseline JSON file for comparison
        #[arg(long)]
        compare_to: String,

        /// Max directory depth for auto-detecting root modules
        #[arg(long, default_value = "10")]
        max_search_depth: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    match cli.command {
        Commands::Breakdown {
            path,
            usage_file,
            reverse,
            max_search_depth,
            format,
        } => rt.block_on(commands::breakdown(
            &path,
            usage_file.as_deref(),
            reverse,
            max_search_depth,
            &format,
        )),
        Commands::Diff {
            path,
            compare_to,
            max_search_depth,
            format,
        } => rt.block_on(commands::diff(&path, &compare_to, max_search_depth, &format)),
    }
}
