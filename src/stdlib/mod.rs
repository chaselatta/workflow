pub mod variable;

use crate::stdlib::variable::Variable;
use anyhow::{anyhow, bail};
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseContextError {
    #[error("Variable(name = {name}) already exists in this context")]
    VariableAlreadyExists { name: String },
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
        F: FnOnce(Option<&Variable>) -> anyhow::Result<T>,
    {
        let vars = self.vars.borrow();
        let var = vars.get(name);
        f(var)
    }

    pub fn add_variable(&self, var: Variable) -> anyhow::Result<()> {
        match self.vars.borrow_mut().insert(var.name(), var) {
            None => Ok(()),
            Some(var) => Err(anyhow!(ParseContextError::VariableAlreadyExists {
                name: var.name()
            })),
        }
    }

    pub fn variable_count(&self) -> usize {
        self.vars.borrow().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            assert_eq!(&v.unwrap().name(), "foo");
            Ok(())
        });
    }

    #[test]
    fn test_variable_cound() {
        let ctx = ParseContext::default();
        let var1 = Variable::new("foo");
        let var2 = Variable::new("bar");
        let _ = ctx.add_variable(var1);
        let _ = ctx.add_variable(var2);

        assert_eq!(ctx.variable_count(), 2);
    }
}
