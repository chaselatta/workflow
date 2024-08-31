pub mod cmd;
pub mod stdlib;

use crate::cmd::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.parse_and_run()
}
