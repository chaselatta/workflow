pub mod parser;
pub mod tool;
pub mod variable;
pub mod variables;

use crate::stdlib::variables::format::format_impl;
use crate::stdlib::variables::format::ValueFormatter;
use crate::stdlib::variables::variable::variable_impl;
use crate::stdlib::variables::variable::VariableRef;
use anyhow::bail;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::list::ListOf;
use starlark::values::tuple::UnpackTuple;
use starlark::values::Value;
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum StdlibError {
    #[error("Invalid attribute '{attr}', {reason} got {value:?}")]
    InvalidAttribute {
        attr: String,
        value: String,
        reason: String,
    },
}

impl StdlibError {
    fn new_invalid_attr<T: Into<String>>(attr: &str, reason: &str, value: T) -> Self {
        StdlibError::InvalidAttribute {
            attr: attr.to_string(),
            reason: reason.to_string(),
            value: value.into(),
        }
    }
}

fn validate_name(name: &str) -> anyhow::Result<String> {
    if name.is_empty() {
        bail!(StdlibError::new_invalid_attr(
            "name",
            "cannot be empty",
            name
        ));
    }
    if name.contains(" ") {
        bail!(StdlibError::new_invalid_attr(
            "name",
            "cannot contain spaces",
            name
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn validate_name_success() {
        assert_eq!(validate_name("foo").unwrap(), "foo".to_string());
        assert_eq!(validate_name("1").unwrap(), "1".to_string());
    }

    #[test]
    #[should_panic]
    fn validate_name_fail_empty() {
        validate_name("").unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_name_fail_spaces() {
        validate_name("a b").unwrap();
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

    #[cfg(test)]
    pub fn assert_env<'a>() -> Assert<'a> {
        let mut env = Assert::new();
        env.globals_add(starlark_stdlib);
        env
    }
}
