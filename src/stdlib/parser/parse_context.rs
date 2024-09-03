use crate::stdlib::variable::{FrozenVariable, Variable};
use anyhow::{anyhow, bail};
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseContextError {
    #[error("Variable(name = '{0}') already exists in this context")]
    VariableAlreadyExists(String),
    #[error("Variable(name = '{0}') does not exists in this context")]
    UnknownVariable(String),
    #[error("Missing ParseContext from evaluator")]
    MissingParseContext,
}

#[derive(Debug, ProvidesStaticType, Default, PartialEq)]
pub struct ParseContext {
    vars: RefCell<HashMap<String, Variable>>,
}

impl ParseContext {
    pub fn from_evaluator<'a>(eval: &'a Evaluator) -> anyhow::Result<&'a ParseContext> {
        if let Some(extra) = eval.extra {
            return Ok(extra.downcast_ref::<ParseContext>().unwrap());
        }
        bail!(ParseContextError::MissingParseContext);
    }

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

    /// Returns a Vec of FrozenVariable objects which represent the
    /// variables at the time of calling. The variables cannot be
    /// updated and should not be held at a later time.
    pub fn snapshot_variables(&self) -> Vec<FrozenVariable> {
        self.vars
            .borrow()
            .values()
            .map(|v| FrozenVariable::from(v))
            .collect()
    }

    /// Updates the variables in the ctx based on the command line flags and
    /// environment variables.
    /// workflow_args is a list of strings that follows the form of
    /// ["--foo", "a", "--bar", "b"] where the value follows the flag.
    pub fn realize_variables(&self, workflow_args: &Vec<String>) {
        let variables = self.snapshot_variables();
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

    #[test]
    fn test_from_evaluator_some() {
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);
        let ctx = ParseContext::default();

        eval.extra = Some(&ctx);
        // Need to box this otherwise it will go out of scope
        // before we use it
        let boxed = Box::new(eval); // box extends the lifetime.
        assert_eq!(ParseContext::from_evaluator(&*boxed).unwrap(), &ctx);
    }

    #[test]
    fn test_add_variable_success() {
        let ctx = ParseContext::default();
        let var = Variable::new("foo");

        assert!(ctx.add_variable(var).is_ok());
    }

    #[test]
    #[should_panic]
    fn test_add_variable_twice_fails() {
        let ctx = ParseContext::default();
        let var1 = Variable::new("foo");
        let var2 = Variable::new("foo");

        ctx.add_variable(var1).unwrap();
        // Fail here
        ctx.add_variable(var2).unwrap();
    }

    #[test]
    fn test_with_variable_success() {
        let ctx = ParseContext::default();
        let var = Variable::new("foo");
        let _ = ctx.add_variable(var);

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
    fn test_variable_count() {
        let ctx = ParseContext::default();
        let var1 = Variable::new("foo");
        let var2 = Variable::new("bar");
        let _ = ctx.add_variable(var1);
        let _ = ctx.add_variable(var2);

        assert_eq!(ctx.snapshot_variables().len(), 2);
    }

    #[test]
    fn test_with_variable_mutable_success() {
        let ctx = ParseContext::default();
        let var = Variable::new("foo");
        let _ = ctx.add_variable(var);

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
    fn test_frozen_variables() {
        let ctx = ParseContext::default();
        let var1 = Variable::new("foo");
        let var2 = Variable::new("bar");
        let _ = ctx.add_variable(var1);
        let _ = ctx.add_variable(var2);

        let frozen = ctx.snapshot_variables();

        assert_eq!(frozen.len(), 2);
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

        ctx.realize_variables(&vec![]);
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

        ctx.realize_variables(&vec![
            "--bar".to_string(),
            "a".to_string(),
            "--foo".to_string(),
            "b".to_string(),
        ]);
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

        ctx.realize_variables(&vec![
            "--bar".to_string(),
            "a".to_string(),
            "--foo".to_string(),
            "b".to_string(),
        ]);
        assert_eq!(
            ctx.with_variable("foo", |v| { Ok(v.read_value("reader")?) })
                .unwrap(),
            "b".to_string(),
        );
    }
}
