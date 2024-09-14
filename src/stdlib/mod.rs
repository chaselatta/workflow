pub mod errors;
pub mod format;
pub mod legacy;
pub mod parser;
pub mod variable;
pub mod variable_resolver;
pub mod variable_store;

use crate::stdlib::format::format_impl;
use crate::stdlib::format::ValueFormatter;
use crate::stdlib::variable::variable_impl;
use crate::stdlib::variable::VariableRef;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::list::ListOf;
use starlark::values::tuple::UnpackTuple;
use starlark::values::Value;

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
}
