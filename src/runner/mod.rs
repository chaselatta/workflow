mod variable_store;
mod workflow_delegate;

pub use self::variable_store::VariableStore;
pub use self::workflow_delegate::WorkflowDelegate;

use crate::stdlib::{starlark_stdlib, ParseDelegate, ParseDelegateHolder};
use starlark::environment::{Globals, GlobalsBuilder, LibraryExtension};
use starlark::eval::Evaluator;
use starlark::syntax::AstModule;
use starlark::syntax::Dialect;
use starlark::values::Value;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;

pub struct Runner {
    globals: Globals,
    delegate: ParseDelegateHolder,
    workflow_file: PathBuf,
}

impl Runner {
    pub fn new<T: ParseDelegate + std::fmt::Debug>(
        workflow_file: PathBuf,
        delegate: T,
    ) -> anyhow::Result<Self> {
        let globals = GlobalsBuilder::extended_by(&[LibraryExtension::Json])
            .with(starlark_stdlib)
            .build();

        Ok(Runner {
            globals: globals,
            delegate: ParseDelegateHolder::new(delegate),
            workflow_file: fs::canonicalize(workflow_file)?,
        })
    }

    pub fn parse_workflow<'a>(&'a self, eval: &mut Evaluator<'a, 'a>) -> anyhow::Result<Value> {
        let ast = AstModule::parse_file(self.workflow_file.as_path(), &Dialect::Standard)
            .map_err(|e| e.into_anyhow())?;
        self.parse_ast(ast, eval)
    }

    fn parse_ast<'a>(
        &'a self,
        ast: AstModule,
        eval: &mut Evaluator<'a, 'a>,
    ) -> anyhow::Result<Value> {
        eval.extra = Some(&self.delegate);

        self.delegate
            .deref()
            .will_parse_workflow(self.workflow_file.clone());
        let res = eval
            .eval_module(ast, &self.globals)
            .map_err(|e| e.into_anyhow())?;

        self.delegate.deref().did_parse_workflow();
        Ok(res)
    }

    pub fn delegate(&self) -> &ParseDelegateHolder {
        &self.delegate
    }

    pub fn working_dir(&self) -> PathBuf {
        let mut parent = self.workflow_file.clone();
        parent.pop();
        parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downcast_delegate_ref;
    use crate::stdlib::test_utils::TestParseDelegate;
    use starlark::environment::Module;
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_file_calls_will_and_did_parse() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/vars_only.workflow");
        let expected = file.clone();

        let runner = Runner::new(file, TestParseDelegate::default()).unwrap();
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);

        let result = runner.parse_workflow(&mut eval).unwrap();
        assert_eq!(result.unpack_i32(), Some(1));

        let holder = runner.delegate();
        assert_eq!(
            downcast_delegate_ref!(holder, TestParseDelegate)
                .unwrap()
                .workflow_file,
            expected.into()
        );
        assert_eq!(
            downcast_delegate_ref!(holder, TestParseDelegate)
                .unwrap()
                .completed,
            true.into()
        );
    }

    #[test]
    fn test_parser_create_success() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/vars_only.workflow");

        Runner::new(file, TestParseDelegate::default()).unwrap();
    }

    #[test]
    #[should_panic(expected = "No such file or directory")]
    fn test_parser_create_fail() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/__no_file__.workflow");

        Runner::new(file, TestParseDelegate::default()).unwrap();
    }

    #[test]
    fn test_working_dir() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/vars_only.workflow");

        let runner = Runner::new(file, TestParseDelegate::default()).unwrap();

        assert_eq!(
            runner.working_dir().file_name(),
            Some(OsStr::new("test_data")),
        )
    }

    #[test]
    fn test_json_support() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("json.workflow");
        let mut file = File::create(file_path.clone()).unwrap();
        writeln!(file, "json.decode('[1, 2, 3]')").unwrap();

        let runner = Runner::new(file_path.clone(), TestParseDelegate::default()).unwrap();

        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);

        let _result = runner.parse_workflow(&mut eval).unwrap();

        drop(file);
        dir.close().unwrap();
    }
}
