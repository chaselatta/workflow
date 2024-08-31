pub mod describe;
use crate::cmd::describe::DescribeArgs;
use clap::{Args, Parser, Subcommand};

pub trait RunCommand {
    fn run(&self, global_args: &GlobalArgs) -> anyhow::Result<()>;
}

#[derive(Args)]
pub struct GlobalArgs {
    /// If set, will suppress extra log information
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Describes the given workflow
    Describe(DescribeArgs),
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
    pub fn parse_and_run(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Describe(args) => {
                return args.run(&self.global_args);
            }
        }
    }
}
