use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ctx-pack", about = "Configuration-driven source packing for LLM context")]
pub struct Cli {
    #[arg(short, long, global = true, help = "Config file path")]
    pub config: Option<std::path::PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init,
    Pack(PackArgs),
    Apply(ApplyArgs),
    Status(StatusArgs),
    Tree(TreeArgs),
    Prompt(PromptArgs),
    MigratePrefix(MigratePrefixArgs),
    Cache(CacheArgs),
}

#[derive(clap::Args, Debug)]
pub struct PackArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,
    #[arg(long)]
    pub stdin: bool,
    #[arg(long, group = "mode")]
    pub full: bool,
    #[arg(long, group = "mode")]
    pub diff: bool,
    #[arg(long, group = "mode")]
    pub auto: bool,
}

#[derive(clap::Args, Debug)]
pub struct ApplyArgs {
    pub file: Option<std::path::PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(clap::Args, Debug)]
pub struct StatusArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(clap::Args, Debug)]
pub struct TreeArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(clap::Args, Debug)]
pub struct PromptArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(clap::Args, Debug)]
pub struct MigratePrefixArgs {
    pub old: String,
    pub new: String,
}

#[derive(clap::Args, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub subcommand: CacheSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum CacheSubcommands {
    Clean(CacheCleanArgs),
    Info,
}

#[derive(clap::Args, Debug)]
pub struct CacheCleanArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}
