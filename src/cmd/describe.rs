use crate::cmd::{GlobalArgs, RunCommand};
use anyhow::bail;
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct DescribeArgs {
    /// The path to the workflow to describe
    pub workflow: PathBuf,
}

impl RunCommand for DescribeArgs {
    fn run(&self, _global_args: &GlobalArgs) -> anyhow::Result<()> {
        println!("RUNNING RUN COMMAND");
        if self.workflow.exists() {
            println!("Parsing workflow at {:?}", self.workflow);
        } else {
            bail!("Workflow does not exist at path {:?}", self.workflow);
        }
        Ok(())
    }
}
