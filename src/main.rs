pub mod cmd;
pub mod stdlib;
pub mod workflow;

use crate::cmd::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.parse_and_run()
}
