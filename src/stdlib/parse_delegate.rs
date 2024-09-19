use crate::stdlib::errors::StdlibError;
use crate::stdlib::VariableEntry;
use anyhow::bail;
use starlark::eval::Evaluator;
use starlark::values::ProvidesStaticType;
use std::any::Any;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;

/// A delegate for parse events
pub trait ParseDelegate: Any {
    fn as_any(&self) -> &dyn Any;

    /// Called when a variable is found
    fn on_variable(&self, _identifier: &str, _variable: VariableEntry) {}

    /// Called when the workflow parsing starts
    fn will_parse_workflow(&self, _workflow: PathBuf) {}
}

/// The ParseDelegateHolder provides a way to hold the delegate
/// so we can pass the delegate into the evaluator
#[derive(ProvidesStaticType)]
pub struct ParseDelegateHolder {
    inner: Box<dyn ParseDelegate + 'static>,
}

impl ParseDelegateHolder {
    pub fn new<T>(delegate: T) -> Self
    where
        T: ParseDelegate + Debug + 'static,
    {
        ParseDelegateHolder {
            inner: Box::new(delegate),
        }
    }

    pub fn from_evaluator<'a>(eval: &'a Evaluator) -> anyhow::Result<&'a ParseDelegateHolder> {
        if let Some(extra) = eval.extra {
            return Ok(extra.downcast_ref::<ParseDelegateHolder>().unwrap());
        }
        bail!(StdlibError::MissingDelegate);
    }
}

impl Deref for ParseDelegateHolder {
    type Target = Box<dyn ParseDelegate + 'static>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
    use crate::stdlib::downcast_delegate_ref;
    use crate::stdlib::test_utils::TestParseDelegate;
    use starlark::environment::Module;
    use std::ops::Deref;

    #[test]
    fn test_from_evaluator() {
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);
        let delegate: TestParseDelegate = TestParseDelegate::default();
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
        let delegate = TestParseDelegate::default();
        let holder = ParseDelegateHolder::new(delegate);

        eval.extra = Some(&holder);

        // box extends the lifetime.
        let eval = Box::new(eval);

        let holder = ParseDelegateHolder::from_evaluator(&eval).unwrap();
        holder.deref().will_parse_workflow(PathBuf::default());
    }

    #[test]
    fn test_downcast_delegate_ref_success() {
        let delegate = TestParseDelegate::default();
        let holder = ParseDelegateHolder::new(delegate);

        let d = downcast_delegate_ref!(holder, TestParseDelegate);
        assert!(d.is_some());
    }

    #[test]
    fn test_downcast_delegate_ref_fail() {
        let delegate = TestParseDelegate::default();
        let holder = ParseDelegateHolder::new(delegate);

        let d = downcast_delegate_ref!(holder, ParseDelegateHolder);
        assert!(d.is_none());
    }
}
