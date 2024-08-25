pub mod cmd;
pub mod parser;

use crate::cmd::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli.parse_and_run() {
        println!("ERROR: {}", e);
    }
}
