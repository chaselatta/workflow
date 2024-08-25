use crate::cmd::{GlobalArgs, RunCommand};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct DumpArgs {
    /// The path to the workflow to dump
    pub workflow: PathBuf,

    /// If we should dump vars
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub vars: bool,
}

impl RunCommand for DumpArgs {
    fn run(&self, _global_args: &GlobalArgs) -> Result<(), String> {
        println!("RUNNING RUN COMMAND");
        if self.workflow.exists() {
            println!("Parsing workflow at {:?}", self.workflow);
        } else {
            return Err(format!(
                "Workflow does not exist at path {:?}",
                self.workflow
            ));
        }
        Ok(())
    }
}
