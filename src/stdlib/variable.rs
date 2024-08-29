use crate::stdlib::ParseContext;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::list::ListOf;
use starlark::values::none::NoneType;

use anyhow::bail;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VariableError {
    #[error("Invalid attribute Variable::{attr}, {reason} got {value:?})")]
    InvalidAttribute {
        attr: String,
        value: String,
        reason: String,
    },
}

impl VariableError {
    fn new_invalid_attr<T: Into<String>>(attr: &str, reason: &str, value: T) -> Self {
        VariableError::InvalidAttribute {
            attr: attr.to_string(),
            reason: reason.to_string(),
            value: value.into(),
        }
    }
}

#[starlark_module]
pub fn starlark_variable(builder: &mut GlobalsBuilder) {
    fn variable(
        #[starlark(require = named)] name: &str,
        #[starlark(require = named)] default: Option<&str>,
        #[starlark(require = named)] env: Option<&str>,
        #[starlark(require = named)] cli_flag: Option<&str>,
        #[starlark(require = named)] readers: Option<ListOf<String>>,
        #[starlark(require = named)] writers: Option<ListOf<String>>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let ctx = ParseContext::from_evaluator(eval)?;
        let var = Variable::from_starlark(name, default, env, cli_flag, readers, writers)?;
        ctx.add_variable(var)?;
        Ok(NoneType)
    }
}

/// A enum representing the scope of a variable.
///
/// Variables are scoped to actions by their name.
#[derive(Debug, Default, PartialEq)]
pub enum VariableScope {
    /// Can be accessed by any action.
    #[default]
    Global,

    /// Scope is restried to the given names.
    Restricted(Vec<String>),
}
//TODO: Add current value

/// A type representing a variable in a workflow.
#[derive(Debug, PartialEq, Default)]
pub struct Variable {
    name: String,
    default: Option<String>,
    env: Option<String>,
    cli_flag: Option<String>,
    readers: VariableScope,
    writers: VariableScope,
}

fn validate_name(name: &str) -> anyhow::Result<String> {
    if name.is_empty() {
        bail!(VariableError::new_invalid_attr(
            "name",
            "cannot be empty",
            name
        ));
    }
    if name.contains(" ") {
        bail!(VariableError::new_invalid_attr(
            "name",
            "cannot contain spaces",
            name
        ));
    }
    Ok(name.to_string())
}

fn validate_env(env: Option<&str>) -> anyhow::Result<Option<String>> {
    if let Some(env) = env {
        if env.is_empty() {
            bail!(VariableError::new_invalid_attr(
                "env",
                "cannot be empty",
                env
            ));
        }
        if env.contains(" ") {
            bail!(VariableError::new_invalid_attr(
                "env",
                "cannot contain spaces",
                env
            ));
        }
        return Ok(Some(env.to_string()));
    }
    Ok(None)
}

fn validate_cli_flag(cli_flag: Option<&str>) -> anyhow::Result<Option<String>> {
    if let Some(flag) = cli_flag {
        if flag.is_empty() {
            bail!(VariableError::new_invalid_attr(
                "cli_flag",
                "cannot be empty",
                flag
            ));
        }

        if flag.contains(" ") {
            bail!(VariableError::new_invalid_attr(
                "cli_flag",
                "cannot contain spaces",
                flag
            ));
        }

        if flag.len() == 2 && (!flag.starts_with("-") || flag == "--") {
            bail!(VariableError::new_invalid_attr(
                "cli_flag",
                "short flags must take the form -v",
                flag
            ));
        }
        if flag.len() > 2 && !flag.starts_with("--") {
            bail!(VariableError::new_invalid_attr(
                "cli_flag",
                "long flags must take the form --value",
                flag
            ));
        }
        return Ok(Some(flag.to_string()));
    }
    Ok(None)
}

fn validate_scope(scopes: Option<Vec<String>>) -> anyhow::Result<VariableScope> {
    if let Some(scopes) = scopes {
        for scope in &scopes {
            if scope.is_empty() {
                bail!(VariableError::new_invalid_attr(
                    "scope",
                    "scopes cannot contain empty strings",
                    scope
                ));
            }

            if scope.contains(" ") {
                bail!(VariableError::new_invalid_attr(
                    "scope",
                    "scopes cannot contain spaces",
                    scope
                ));
            }
        }
        return Ok(VariableScope::Restricted(scopes));
    }

    Ok(VariableScope::Global)
}

impl Variable {
    pub fn new(name: &str) -> Self {
        Variable {
            name: name.to_string(),
            ..Variable::default()
        }
    }

    pub fn name(&self) -> String {
        self.name.to_owned().clone()
    }

    fn from_starlark(
        name: &str,
        default: Option<&str>,
        env: Option<&str>,
        cli_flag: Option<&str>,
        readers: Option<ListOf<String>>,
        writers: Option<ListOf<String>>,
    ) -> anyhow::Result<Self> {
        Ok(Variable {
            name: validate_name(name)?,
            default: default.map(|d| d.to_string()),
            env: validate_env(env)?,
            cli_flag: validate_cli_flag(cli_flag)?,
            readers: validate_scope(readers.map(|v| v.to_vec()))?,
            writers: validate_scope(writers.map(|v| v.to_vec()))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;
    use starlark::syntax::{AstModule, Dialect};

    impl VariableScope {
        fn from_str_list(values: Vec<&str>) -> Self {
            VariableScope::Restricted(values.into_iter().map(|v| v.to_string()).collect())
        }
    }

    #[test]
    fn test_variable_scope_default() {
        assert_eq!(VariableScope::default(), VariableScope::Global);
    }

    #[test]
    fn test_variable_scope_from_str_list() {
        assert_eq!(
            VariableScope::Restricted(vec!["a".to_string(), "b".to_string()]),
            VariableScope::from_str_list(vec!["a", "b"])
        );
    }

    #[test]
    fn test_variable_scope_equality() {
        assert_eq!(VariableScope::Global, VariableScope::Global);
        assert_eq!(
            VariableScope::Restricted(vec![]),
            VariableScope::Restricted(vec![])
        );
        assert_eq!(
            VariableScope::from_str_list(vec!["a", "b"]),
            VariableScope::from_str_list(vec!["a", "b"])
        );
    }

    // --- name

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

    // --- env

    #[test]
    fn validate_env_success() {
        assert_eq!(
            validate_env(Some("foo")).unwrap().unwrap(),
            "foo".to_string()
        );
        assert_eq!(validate_env(Some("1")).unwrap().unwrap(), "1".to_string());
        assert_eq!(validate_env(None).unwrap(), None);
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_empty() {
        validate_env(Some("")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_spaces() {
        validate_env(Some("a b")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_spaces_only() {
        validate_env(Some(" ")).unwrap();
    }

    // --- cli_flag

    #[test]
    fn validate_cli_flag_success() {
        assert_eq!(
            validate_cli_flag(Some("--foo")).unwrap().unwrap(),
            "--foo".to_string()
        );
        assert_eq!(
            validate_cli_flag(Some("-v")).unwrap().unwrap(),
            "-v".to_string()
        );
        assert_eq!(validate_cli_flag(None).unwrap(), None);
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_empty() {
        validate_cli_flag(Some("")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_spaces() {
        validate_cli_flag(Some("a b")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_missing_dashes() {
        validate_cli_flag(Some("foo")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_two_dashes() {
        validate_cli_flag(Some("--")).unwrap();
    }

    // -- Scopes

    #[test]
    fn validate_scope_success() {
        assert_eq!(validate_scope(None).unwrap(), VariableScope::Global);
        assert_eq!(
            validate_scope(Some(["a".to_owned(), "b".to_owned()].to_vec())).unwrap(),
            VariableScope::from_str_list(vec!["a", "b"]),
        );
    }

    #[test]
    #[should_panic]
    fn validate_scope_fail_empty() {
        validate_scope(Some(["".to_owned()].to_vec())).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_scope_fail_spaces() {
        validate_scope(Some(["a b".to_owned()].to_vec())).unwrap();
    }

    // - parsing

    #[test]
    fn test_collect_variables() {
        let starlark_code = r#"
variable(
  name = "foo",
)
variable(
  name = "bar",
)
"#;
        let globals = GlobalsBuilder::new().with(starlark_variable).build();
        let module: Module = Module::new();
        let ctx = ParseContext::default();
        let mut eval: Evaluator = Evaluator::new(&module);
        eval.extra = Some(&ctx);
        let ast: AstModule =
            AstModule::parse("test.star", starlark_code.to_owned(), &Dialect::Standard).unwrap();
        let _res = eval.eval_module(ast, &globals).unwrap();

        assert_eq!(ctx.variable_count(), 2);
    }

    #[test]
    fn test_variable_parse_all_values() {
        let starlark_code = r#"
variable(
  name = "foo",
  default = "default",
  env = "FOO",
  cli_flag = "--foo",
  readers = ["a", "b"],
  writers = ["c", "d"],
)
"#;
        let globals = GlobalsBuilder::new().with(starlark_variable).build();
        let module: Module = Module::new();
        let ctx = ParseContext::default();
        let mut eval: Evaluator = Evaluator::new(&module);
        eval.extra = Some(&ctx);
        let ast: AstModule =
            AstModule::parse("test.star", starlark_code.to_owned(), &Dialect::Standard).unwrap();
        let _res = eval.eval_module(ast, &globals).unwrap();

        let _ = ctx.with_variable("foo", |v| {
            assert_eq!(
                v.unwrap(),
                &Variable {
                    name: "foo".to_string(),
                    default: Some("default".to_string()),
                    env: Some("FOO".to_string()),
                    cli_flag: Some("--foo".to_string()),
                    readers: VariableScope::from_str_list(vec!["a", "b"]),
                    writers: VariableScope::from_str_list(vec!["c", "d"]),
                }
            );
            Ok(())
        });
    }
}
