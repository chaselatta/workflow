use crate::cmd::{GlobalArgs, RunCommand};
use crate::stdlib::parser::Parser;
use anyhow::bail;
use clap::Args;
use std::path::PathBuf;

use starlark::environment::Globals;
use starlark::environment::GlobalsBuilder;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};

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

            let parser = Parser::new();
            let module: Module = Module::new();
            let mut eval: Evaluator = Evaluator::new(&module);

            parser.parse_workflow_file(self.workflow.clone(), &mut eval)?;

            for v in parser.ctx.snapshot_variables() {
                println!("var = {:?}", v);
            }
        } else {
            bail!("Workflow does not exist at path {:?}", self.workflow);
        }
        Ok(())
    }
}
