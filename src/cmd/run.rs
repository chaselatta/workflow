use crate::cmd::{GlobalArgs, RunCommand};
use crate::downcast_delegate_ref;
use crate::runner::{Runner, WorkflowDelegate};
use crate::stdlib::Workflow;
use anyhow::bail;
use clap::Args;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use std::ops::Deref;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The path to the workflow to describe
    pub workflow: PathBuf,

    /// The additional arguments that will be passed along to the workflow
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub workflow_args: Vec<String>,
}

impl RunCommand for RunArgs {
    fn run(&self, _global_args: &GlobalArgs) -> anyhow::Result<()> {
        if self.workflow.exists() {
            let runner = Runner::new(
                self.workflow.clone(),
                WorkflowDelegate::with_args(self.workflow_args.clone()),
            )?;
            let module: Module = Module::new();
            let mut eval: Evaluator = Evaluator::new(&module);

            let _result = runner.parse_workflow(&mut eval).unwrap();

            let holder = runner.delegate();
            let delegate = downcast_delegate_ref!(holder, WorkflowDelegate).unwrap();
            let working_dir = runner.working_dir();

            // TOOD: add run_workflow function instead of looking for main
            if let Some(main) = module.get("main") {
                let workflow = Workflow::from_value(main).unwrap();
                let _ = workflow.run(delegate, &working_dir, &mut eval);
            }
        } else {
            bail!("Workflow does not exist at path {:?}", self.workflow);
        }
        Ok(())
    }
}
