pub mod errors;
pub mod format;
pub mod legacy;
pub mod parse_delegate;
pub mod parser;
pub mod variable;
pub mod variable_resolver;

pub use self::parse_delegate::{ParseDelegate, ParseDelegateHolder};
pub use crate::stdlib::variable::{ValueContext, ValueUpdatedBy, VariableEntry, VariableRef};

use crate::stdlib::format::format_impl;
use crate::stdlib::format::ValueFormatter;
use crate::stdlib::variable::variable_impl;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::list::ListOf;
use starlark::values::tuple::UnpackTuple;
use starlark::values::Value;

/// A macro to downcast the delegate to an Option<T> without having
/// to deal with lifetimes.
///
/// let delegate: Option<Foo> = downcast_delegate_ref!(holder, Foo);
#[macro_export]
macro_rules! downcast_delegate_ref {
    ($y:ident, $x:tt) => {
        (&*$y.deref()).as_any().downcast_ref::<$x>()
    };
}

pub use downcast_delegate_ref;

/// The workflow standard library. All functions in this module
/// are added to the workflow parser to be made availalbe to workflows.
#[starlark_module]
pub fn starlark_stdlib(builder: &mut GlobalsBuilder) {
    /// The variable definition
    fn variable(
        #[starlark(require = named)] default: Option<&str>,
        #[starlark(require = named)] env: Option<&str>,
        #[starlark(require = named)] cli_flag: Option<&str>,
        #[starlark(require = named)] readers: Option<ListOf<String>>,
        #[starlark(require = named)] writers: Option<ListOf<String>>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<VariableRef> {
        variable_impl(default, env, cli_flag, readers, writers, eval)
    }

    /// The format definition
    fn format(
        #[starlark(require = pos)] fmt_str: &str,
        #[starlark(args)] args: UnpackTuple<Value>,
    ) -> anyhow::Result<ValueFormatter> {
        format_impl(fmt_str, args)
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use starlark::assert::Assert;
    use std::any::Any;
    use std::cell::RefCell;
    use std::path::PathBuf;

    pub struct TempEnvVar {
        pub key: String,
        pub original: Option<String>,
    }

    impl TempEnvVar {
        pub fn new(key: &str, val: &str) -> Self {
            let original = std::env::var(&key).ok();
            std::env::set_var(key, val.to_string());
            TempEnvVar {
                key: key.to_string(),
                original: original,
            }
        }
    }

    impl Drop for TempEnvVar {
        fn drop(&mut self) {
            if let Some(original) = &self.original {
                std::env::set_var(&self.key, original.clone());
            } else {
                std::env::remove_var(&self.key);
            }
        }
    }

    pub fn assert_env<'a>() -> Assert<'a> {
        let mut env = Assert::new();
        env.globals_add(starlark_stdlib);
        env
    }

    #[derive(Debug, Default)]
    pub struct TestParseDelegate {
        pub on_variable_call_count: RefCell<u32>,
        pub workflow_file: RefCell<PathBuf>,
        pub completed: RefCell<bool>,
    }

    impl ParseDelegate for TestParseDelegate {
        fn on_variable(&self, _id: &str, _v: VariableEntry) {
            let v = *self.on_variable_call_count.borrow() + 1;
            self.on_variable_call_count.replace(v);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn will_parse_workflow(&self, workflow: PathBuf) {
            self.workflow_file.replace(workflow);
        }

        fn did_parse_workflow(&self) {
            self.completed.replace(true);
        }
    }
}
