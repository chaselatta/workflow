pub mod variable;

use crate::stdlib::variable::Variable;
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::cell::Ref;
use std::cell::RefCell;

#[derive(Debug, ProvidesStaticType, Default, PartialEq)]
pub struct ParseContext {
    vars: RefCell<Vec<Variable>>,
}

impl ParseContext {
    pub fn from_evaluator<'a>(eval: &'a Evaluator) -> Option<&'a ParseContext> {
        if let Some(extra) = eval.extra {
            return extra.downcast_ref::<ParseContext>();
        }
        None
    }

    //TODO: TEST
    pub fn variables(&self) -> Ref<Vec<Variable>> {
        self.vars.borrow()
    }

    //TODO: TEST
    pub fn add_variable(&self, var: Variable) {
        self.vars.borrow_mut().push(var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;

    #[test]
    fn test_from_evaluator_none() {
        let module: Module = Module::new();
        let eval: Evaluator = Evaluator::new(&module);

        assert_eq!(ParseContext::from_evaluator(&eval), None);
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
        assert_eq!(ParseContext::from_evaluator(&*boxed), Some(&ctx));
    }
}
