mod variable_store;
mod workflow_context;

pub use self::variable_store::VariableStore;
pub use self::workflow_context::WorkflowContext;

use crate::stdlib::{starlark_stdlib, ParseDelegate, ParseDelegateHolder};
use starlark::environment::{Globals, GlobalsBuilder};
use std::fs;
use std::path::PathBuf;

pub struct Workflow {
    globals: Globals,
    delegate: ParseDelegateHolder,
}

#[derive(Debug)]
struct TempDelegate {}
impl ParseDelegate for TempDelegate {
    fn on_variable(&self, i: u32) {
        todo!()
    }
}

impl Workflow {
    pub fn new(workflow_file: PathBuf) -> anyhow::Result<Self> {
        let globals = GlobalsBuilder::standard().with(starlark_stdlib).build();
        let delegate = TempDelegate {};
        let _x = fs::canonicalize(workflow_file)?;
        // let ctx = WorkflowContext::new(fs::canonicalize(workflow_file)?);

        Ok(Workflow {
            globals: globals,
            delegate: ParseDelegateHolder::new(delegate),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use starlark::environment::Module;
    // use starlark::eval::Evaluator;

    // #[test]
    // fn test_parse_file() {
    //     let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     file.push("src/test_data/vars_only.workflow");

    //     let parser = WorkflowRunner::new(file).unwrap();
    //     let module: Module = Module::new();
    //     let mut eval: Evaluator = Evaluator::new(&module);

    //     // parser.parse_workflow(&mut eval).unwrap();

    //     // assert_eq!(parser.ctx.snapshot().variables.len(), 3);
    //     // assert_eq!(parser.workflow_file, file);
    // }

    #[test]
    fn test_parser_create_success() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/vars_only.workflow");

        Workflow::new(file).unwrap();
    }

    #[test]
    #[should_panic(expected = "No such file or directory")]
    fn test_parser_create_fail() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/__no_file__.workflow");

        Workflow::new(file).unwrap();
    }
}
