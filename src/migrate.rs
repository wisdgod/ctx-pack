pub mod prefix;

use anyhow::Result;

use crate::cli::MigratePrefixArgs;
use crate::config::Config;

pub fn run_migrate_prefix(config: &Config, args: &MigratePrefixArgs) -> Result<()> {
    prefix::migrate_prefix(&args.old, &args.new, config)
}
