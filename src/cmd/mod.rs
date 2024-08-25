pub mod dump;
use crate::cmd::dump::DumpArgs;
use clap::{Args, Parser, Subcommand};

pub trait RunCommand {
    fn run(&self, global_args: &GlobalArgs) -> Result<(), String>;
}

#[derive(Args)]
pub struct GlobalArgs {
    /// If set, will suppress extra log information
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Dumps the given workflow
    Dump(DumpArgs),
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[clap(flatten)]
    pub global_args: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn parse_and_run(&self) -> Result<(), String> {
        match &self.command {
            Commands::Dump(args) => {
                return Ok(args.run(&self.global_args)?);
            }
        }
    }
}
