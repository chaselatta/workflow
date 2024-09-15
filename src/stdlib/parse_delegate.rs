use anyhow::bail;
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::fmt::Debug;
use std::ops::Deref;

use crate::stdlib::errors::StdlibError;

/// A delegate for parse events
pub trait ParseDelegate {
    /// Called when a variable is found
    fn on_variable(&self, i: u32);
}

/// The ParseDelegateHolder provides a way to hold the delegate
/// so we can pass the delegate into the evaluator
#[derive(ProvidesStaticType)]
pub struct ParseDelegateHolder {
    pub delegate: Box<dyn ParseDelegate>,
}

impl ParseDelegateHolder {
    pub fn new<T>(delegate: T) -> Self
    where
        T: ParseDelegate + Debug + 'static,
    {
        ParseDelegateHolder {
            delegate: Box::new(delegate),
        }
    }

    pub fn from_evaluator<'a>(eval: &'a Evaluator) -> anyhow::Result<&'a ParseDelegateHolder> {
        if let Some(extra) = eval.extra {
            return Ok(extra.downcast_ref::<ParseDelegateHolder>().unwrap());
        }
        bail!(StdlibError::MissingDelegate);
    }
}

impl Debug for ParseDelegateHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ParseDelegateHolder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;

    #[derive(Debug)]
    struct StubDelegate {}
    impl ParseDelegate for StubDelegate {
        fn on_variable(&self, i: u32) {}
    }

    #[test]
    fn test_from_evaluator() {
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);
        let delegate = StubDelegate {};
        let holder = ParseDelegateHolder::new(delegate);

        eval.extra = Some(&holder);

        // box extends the lifetime.
        let eval = Box::new(eval);

        ParseDelegateHolder::from_evaluator(&eval).unwrap();
    }

    #[test]
    #[should_panic(expected = "Expected to find a delegate but none found")]
    fn test_from_evaluator_fail() {
        let module: Module = Module::new();
        let eval: Evaluator = Evaluator::new(&module);
        // box extends the lifetime.
        let eval = Box::new(eval);
        ParseDelegateHolder::from_evaluator(&eval).unwrap();
    }

    #[test]
    fn test_can_call_delegate() {
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);
        let delegate = StubDelegate {};
        let holder = ParseDelegateHolder::new(delegate);

        eval.extra = Some(&holder);

        // box extends the lifetime.
        let eval = Box::new(eval);

        let holder = ParseDelegateHolder::from_evaluator(&eval).unwrap();
        holder.delegate.deref().on_variable(1);
    }
}
