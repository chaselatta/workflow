use crate::runner::VariableStore;
use crate::stdlib::legacy::tool::{FrozenTool, Tool};
use crate::stdlib::legacy::variable::{FrozenVariable, Variable};
use crate::stdlib::parser::StringInterpolator;
use anyhow::{anyhow, bail};
use regex::{Captures, Regex};
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseContextError {
    #[error("Variable(name = '{0}') already exists in this context")]
    VariableAlreadyExists(String),
    #[error("Variable(name = '{0}') does not exists in this context")]
    UnknownVariable(String),
    #[error("Tool(name = '{0}') already exists in this context")]
    ToolAlreadyExists(String),
    #[error("Tool(name = '{0}') does not exists in this context")]
    UnknownTool(String),
    #[error("Missing ParseContext from evaluator")]
    MissingParseContext,
}

#[derive(Debug, ProvidesStaticType, Default)]
pub struct ParseContext {
    vars: RefCell<HashMap<String, Variable>>,
    tools: RefCell<HashMap<String, Tool>>,
    workflow_file: PathBuf,
    variable_store: VariableStore,
}

pub struct ParseContextSnapshot {
    pub variables: Vec<FrozenVariable>,
    pub tools: Vec<FrozenTool>,
}

impl ParseContext {
    pub fn new(workflow_file: PathBuf) -> Self {
        return ParseContext {
            workflow_file: workflow_file,
            ..ParseContext::default()
        };
    }

    pub fn workflow_file(&self) -> &PathBuf {
        &self.workflow_file
    }

    pub fn variable_store(&self) -> &VariableStore {
        &self.variable_store
    }

    pub fn from_evaluator<'a>(eval: &'a Evaluator) -> anyhow::Result<&'a ParseContext> {
        if let Some(extra) = eval.extra {
            return Ok(extra.downcast_ref::<ParseContext>().unwrap());
        }
        bail!(ParseContextError::MissingParseContext);
    }

    pub fn snapshot(&self) -> ParseContextSnapshot {
        ParseContextSnapshot {
            variables: self
                .vars
                .borrow()
                .values()
                .map(|v| FrozenVariable::from(v))
                .collect(),
            tools: self
                .tools
                .borrow()
                .values()
                .map(|v| v.freeze(self))
                .collect(),
        }
    }

    /// Updates the context based on the environment.
    ///
    /// Updates the variables in the ctx based on the command line flags and
    /// environment variables.
    /// workflow_args is a list of strings that follows the form of
    /// ["--foo", "a", "--bar", "b"] where the value follows the flag.
    pub fn update_from_environment(&self, workflow_args: &Vec<String>) {
        let snapshot = self.snapshot();

        // We must realize the variables before anything else
        self.realize_variables(&snapshot.variables, workflow_args);
    }

    pub fn workflow_dir(&self) -> PathBuf {
        // workflow_file is the path to the workflow file but we want our paths to be
        // relative to the directory that the file is in.
        let mut workflow_path = self.workflow_file.clone();

        // Pop removes the filename (/path/to/foo.workflow -> /path/to)
        workflow_path.pop();
        workflow_path
    }

    fn realize_variables(&self, variables: &Vec<FrozenVariable>, workflow_args: &Vec<String>) {
        for frozen_var in variables {
            let _ = self.with_variable_mut(&frozen_var.name, |v| {
                // First, check to see if there is a command line flag that matches
                if v.try_update_value_from_cli_flag(workflow_args).is_ok() {
                    return Ok(());
                }
                // Next,  try to set the value from theenv
                if v.try_update_value_from_env().is_ok() {
                    return Ok(());
                }
                // Nothing to update, fall back to the default
                Ok(())
            });
        }
    }

    // - Variables

    pub fn with_variable<F, T>(&self, name: &str, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Variable) -> anyhow::Result<T>,
    {
        let vars = self.vars.borrow();
        if let Some(var) = vars.get(name) {
            f(var)
        } else {
            bail!(ParseContextError::UnknownVariable(name.to_string()))
        }
    }

    pub fn with_variable_mut<F, T>(&self, name: &str, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut Variable) -> anyhow::Result<T>,
    {
        let mut vars = self.vars.borrow_mut();
        if let Some(var) = vars.get_mut(name) {
            f(var)
        } else {
            bail!(ParseContextError::UnknownVariable(name.to_string()))
        }
    }

    pub fn add_variable(&self, var: Variable) -> anyhow::Result<()> {
        match self.vars.borrow_mut().insert(var.name(), var) {
            None => Ok(()),
            Some(var) => Err(anyhow!(ParseContextError::VariableAlreadyExists(
                var.name()
            ))),
        }
    }

    // - Tools
    pub fn add_tool(&self, tool: Tool) -> anyhow::Result<()> {
        match self.tools.borrow_mut().insert(tool.name(), tool) {
            None => Ok(()),
            Some(var) => Err(anyhow!(ParseContextError::ToolAlreadyExists(var.name()))),
        }
    }

    pub fn with_tool<F, T>(&self, name: &str, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Tool) -> anyhow::Result<T>,
    {
        let tools = self.tools.borrow();
        if let Some(tool) = tools.get(name) {
            f(tool)
        } else {
            bail!(ParseContextError::UnknownTool(name.to_string()))
        }
    }
}

impl StringInterpolator for ParseContext {
    fn interpolate(&self, s: &str, reader: &str) -> anyhow::Result<String> {
        let re = Regex::new(r"\{(?<func>\S+)\((?<arg>\S+)\)\}").unwrap();

        // Collect all of the new values first. We do this so that we can return an error if needed which
        // is not possible from inside the replace_all call. This leads to us iterating the regex twice
        // so we should try to optimize in the future.
        let mut func_results: HashMap<String, String> = HashMap::new();
        for caps in re.captures_iter(s) {
            if &caps["func"] == "variable" {
                func_results.insert(
                    caps[0].to_string(),
                    self.with_variable(&caps["arg"], |v| Ok(v.read_value(reader)?))?,
                );
            } else {
                bail!(
                    "Unknown function '{}' in string interpolation",
                    &caps["func"]
                );
            }
        }

        let result = re.replace_all(s, |caps: &Captures| {
            format!(
                "{}",
                func_results.get(&caps[0]).unwrap_or(&caps[0].to_string())
            )
        });

        return Ok(result.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::TempEnvVar;
    use starlark::environment::Module;

    #[test]
    #[should_panic]
    fn test_from_evaluator_none() {
        let module: Module = Module::new();
        let eval: Evaluator = Evaluator::new(&module);

        ParseContext::from_evaluator(&eval).unwrap();
    }

    // #[test]
    // fn test_from_evaluator_some() {
    //     let module: Module = Module::new();
    //     let mut eval: Evaluator = Evaluator::new(&module);
    //     let ctx = ParseContext::default();

    //     eval.extra = Some(&ctx);
    //     // Need to box this otherwise it will go out of scope
    //     // before we use it
    //     let boxed = Box::new(eval); // box extends the lifetime.
    //     // assert_eq!(ParseContext::from_evaluator(&*boxed).unwrap(), &ctx);
    // }

    #[test]
    fn test_add_variable_success() {
        let ctx = ParseContext::default();
        assert!(ctx
            .add_variable(Variable::for_test("foo", None, None, None))
            .is_ok());
    }

    #[test]
    #[should_panic]
    fn test_add_variable_twice_fails() {
        let ctx = ParseContext::default();
        ctx.add_variable(Variable::for_test("foo", None, None, None))
            .unwrap();
        // Fail here
        ctx.add_variable(Variable::for_test("foo", None, None, None))
            .unwrap();
    }

    #[test]
    fn test_with_variable_success() {
        let ctx = ParseContext::default();
        let _ = ctx.add_variable(Variable::for_test("foo", None, None, None));

        let _ = ctx.with_variable("foo", |v| {
            assert_eq!(&v.name(), "foo");
            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Variable(name = 'foo') does not exists in this context")]
    fn test_with_variable_fails_if_missing_variable() {
        let ctx = ParseContext::default();
        ctx.with_variable("foo", |_v| Ok(())).unwrap();
    }

    #[test]
    #[should_panic(expected = "Variable(name = 'foo') does not exists in this context")]
    fn test_with_variable_mut_fails_if_missing_variable() {
        let ctx = ParseContext::default();
        ctx.with_variable_mut("foo", |_v| Ok(())).unwrap();
    }

    #[test]
    fn test_with_variable_mutable_success() {
        let ctx = ParseContext::default();
        let _ = ctx.add_variable(Variable::for_test("foo", None, None, None));

        let _ = ctx.with_variable_mut("foo", |v| {
            v.write_value("new", "test_writer")?;
            Ok(())
        });

        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("test_writer")?) })
                .unwrap(),
            "new".to_string()
        );
    }

    #[test]
    fn test_realize_variables_env() {
        let ctx = ParseContext::default();
        let env = TempEnvVar::new("ENV_VAR_FOR_test_realize_variables_env", "some_value");
        let var = Variable::for_test(
            /* name */ "foo",
            /* default */ Some(""),
            /* cli_flag */ None,
            /* env */ Some(&env.key.clone()),
        );
        ctx.add_variable(var).unwrap();

        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "".to_string()
        );

        ctx.realize_variables(&ctx.snapshot().variables, &vec![]);
        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "some_value".to_string()
        );
    }

    #[test]
    fn test_realize_variables_cli_flag() {
        let ctx = ParseContext::default();
        let var = Variable::for_test(
            /* name */ "foo",
            /* default */ Some(""),
            /* cli_flag */ Some("--foo"),
            /* env */ None,
        );
        ctx.add_variable(var).unwrap();

        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "".to_string()
        );

        ctx.realize_variables(
            &ctx.snapshot().variables,
            &vec![
                "--bar".to_string(),
                "a".to_string(),
                "--foo".to_string(),
                "b".to_string(),
            ],
        );
        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "b".to_string(),
        );
    }

    #[test]
    fn test_realize_variables_honors_order() {
        let env = TempEnvVar::new(
            "ENV_VAR_FOR_test_realize_variables_honors_order",
            "some_value",
        );
        let ctx = ParseContext::default();
        let var = Variable::for_test(
            /* name */ "foo",
            /* default */ Some(""),
            /* cli_flag */ Some("--foo"),
            /* env */ Some(&env.key.clone()),
        );
        ctx.add_variable(var).unwrap();

        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "".to_string()
        );

        ctx.realize_variables(
            &ctx.snapshot().variables,
            &vec![
                "--bar".to_string(),
                "a".to_string(),
                "--foo".to_string(),
                "b".to_string(),
            ],
        );
        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "b".to_string(),
        );
    }

    // - Tool Tests

    #[test]
    #[should_panic(expected = "Tool(name = 'foo') already exists in this context")]
    fn test_add_tool_twice_fails() {
        let ctx = ParseContext::default();
        ctx.add_tool(Tool::for_test("foo")).unwrap();
        // Fail here
        ctx.add_tool(Tool::for_test("foo")).unwrap();
    }

    #[test]
    fn test_with_tool_success() {
        let ctx = ParseContext::default();
        let _ = ctx.add_tool(Tool::for_test("foo"));

        let _ = ctx.with_tool("foo", |t| {
            assert_eq!(&t.name(), "foo");
            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Tool(name = 'foo') does not exists in this context")]
    fn test_with_tool_fails_if_missing_tool() {
        let ctx = ParseContext::default();
        ctx.with_tool("foo", |_| Ok(())).unwrap();
    }

    // - Snapshot

    #[test]
    fn test_snapshot() {
        let ctx = ParseContext::default();

        // variables
        let _ = ctx.add_variable(Variable::for_test("foo", None, None, None));
        let _ = ctx.add_variable(Variable::for_test("bar", None, None, None));

        // tools
        let _ = ctx.add_tool(Tool::for_test("foo")).unwrap();
        let _ = ctx.add_tool(Tool::for_test("bar")).unwrap();

        let snapshot = ctx.snapshot();
        assert_eq!(snapshot.variables.len(), 2);
        assert_eq!(snapshot.tools.len(), 2);
    }

    // - StringInterpolator

    #[test]
    fn test_interpolate_string_no_captures() {
        let ctx = ParseContext::default();
        assert_eq!(ctx.interpolate("", "").unwrap(), "".to_string());

        assert_eq!(ctx.interpolate("hello", "").unwrap(), "hello".to_string());

        assert_eq!(
            ctx.interpolate("\"hello\"", "").unwrap(),
            "\"hello\"".to_string()
        );
    }

    #[test]
    fn test_interpolate_string_for_variable() {
        let ctx = ParseContext::default();
        let _ = ctx.add_variable(Variable::for_test("foo", Some("foo-value"), None, None));
        let _ = ctx.add_variable(Variable::for_test("bar", Some("bar-value"), None, None));

        assert_eq!(
            ctx.interpolate("foo = {variable(foo)}", "").unwrap(),
            "foo = foo-value".to_string()
        );

        assert_eq!(
            ctx.interpolate("{variable(bar)}, {variable(foo)}", "")
                .unwrap(),
            "bar-value, foo-value".to_string()
        );
    }

    #[test]
    #[should_panic(expected = "Unknown function '__not_a_function__' in string interpolation")]
    fn test_interpolate_string_fail_unknown_function() {
        let ctx = ParseContext::default();

        ctx.interpolate("foo = {__not_a_function__(foo)}", "")
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "'A' not in allowed scopes to read var 'foo'")]
    fn test_interpolate_string_fail_restricted() {
        let ctx = ParseContext::default();
        let _ = ctx.add_variable(Variable::for_test_restricted("foo", "B"));

        ctx.interpolate("foo = {variable(foo)}", "A").unwrap();
    }

    #[test]
    #[should_panic(expected = "Variable(name = 'foo') does not exists in this context")]
    fn test_interpolate_string_unknown_variable() {
        let ctx = ParseContext::default();
        ctx.interpolate("foo = {variable(foo)}", "").unwrap();
    }

    #[test]
    #[should_panic(expected = "No value set for 'foo'")]
    fn test_interpolate_string_variable_not_set() {
        let ctx = ParseContext::default();
        let _ = ctx.add_variable(Variable::for_test("foo", None, None, None));
        ctx.interpolate("foo = {variable(foo)}", "").unwrap();
    }
}
