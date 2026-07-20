use anyhow::Result;
use clap::Parser;
use tracing::info;

mod apply;
mod cli;
mod config;
mod detection;
mod discovery;
mod encoding_layer;
mod extraction;
mod index;
mod migrate;
mod pack;
mod version;

use cli::{CacheSubcommands, Cli, Commands};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let config = config::load_config(cli.config.as_deref())?;
    let warnings = config::validate_config(&config);
    for w in &warnings {
        tracing::warn!("{}", w);
    }

    match cli.command {
        Commands::Init => {
            init::run_init()?;
        }
        Commands::Pack(args) => {
            pack::run_pack(&config, &args)?;
        }
        Commands::Apply(args) => {
            apply::run_apply(&config, &args)?;
        }
        Commands::Status(args) => {
            pack::run_status(&config, &args)?;
        }
        Commands::Tree(args) => {
            pack::run_tree(&config, &args)?;
        }
        Commands::Prompt(args) => {
            pack::run_prompt(&config, &args)?;
        }
        Commands::MigratePrefix(args) => {
            migrate::run_migrate_prefix(&config, &args)?;
        }
        Commands::Cache(args) => match args.subcommand {
            CacheSubcommands::Clean(clean_args) => {
                pack::run_cache_clean(&config, &clean_args)?;
            }
            CacheSubcommands::Info => {
                pack::run_cache_info(&config)?;
            }
        },
    }

    info!("done");
    Ok(())
}

mod init {
    use anyhow::Result;

    pub fn run_init() -> Result<()> {
        let content = r#"# ctx-pack configuration
# Run `ctx-pack pack` to generate context files

global:
  # XML tag prefix used in output file
  prefix: "ctx"
  # Anchor line interval (0 = disabled). Annotates every Nth line with line number.
  anchor_interval: 10
  # Replace leading spaces with [N] prefix to save tokens
  indent_encoding: true
  # Tab width in spaces
  tab_width: 4
  # How to handle binary files: skip | warn | abort
  binary_policy: skip
  # Auto-detect and convert non-UTF-8 files
  encoding_detection: true
  # Total output size warning threshold
  max_content_size: "500KB"
  # Per-file size warning threshold
  max_file_size: "100KB"
  # Size policy: warn | abort | ignore
  size_policy: warn
  # Index file path (tracks file versions)
  index_file: ".ctx-index.yaml"
  # Cache directory for snapshots
  cache_dir: ".ctx-cache"
  # Number of recent snapshots to retain per file
  cache_retention: 5
  # Generate manifest file alongside output
  manifest: true
  # Generate protocol explanation prompt
  prompt_generation: true

profiles:
  default:
    roots:
      - path: "."
        label: "project"
    discovery:
      use_gitignore: true
      include: []
      exclude:
        - "*.ctx"
        - "*.ctx.manifest"
        - "*.rej"
        - ".ctx-cache/**"
        - ".ctx-index.yaml"
        - "ctx-pack.yaml"
    extraction:
      default_mode: full
      rules: []
    versioning:
      auto_diff: true
      replace_threshold: 0.5
      max_patches_before_replace: 5
    output:
      file: "context.ctx"
      manifest: "context.ctx.manifest"
"#;

        let path = std::path::Path::new("ctx-pack.yaml");
        if path.exists() {
            anyhow::bail!("ctx-pack.yaml already exists");
        }
        std::fs::write(path, content)?;
        println!("Created ctx-pack.yaml");
        Ok(())
    }
}
