use crate::stdlib::parser::parse_context::ParseContext;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::list::ListOf;
use starlark::values::none::NoneType;
use std::fmt;

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

    #[error("'{reader}' not in allowed scopes to read var '{name}'")]
    ReadNotAllowed { reader: String, name: String },

    #[error("'{writer}' not in allowed scopes to write var '{name}'")]
    WriteNotAllowed { writer: String, name: String },

    #[error("No value set for '{0}'")]
    NoDefaultValueSet(String),
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
#[derive(Debug, Default, PartialEq, Clone)]
pub enum VariableScope {
    /// Can be accessed by any action.
    #[default]
    Global,

    /// Scope is restried to the given names.
    Restricted(Vec<String>),
}

impl fmt::Display for VariableScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableScope::Global => write!(f, "Global"),
            VariableScope::Restricted(scopes) => write!(f, "Restricted: [{}]", scopes.join(", ")),
        }
    }
}

/// A type representing a variable in a workflow.
#[derive(Debug, PartialEq, Default)]
pub struct Variable {
    name: String,
    default: Option<String>,
    env: Option<String>,
    cli_flag: Option<String>,
    readers: VariableScope,
    writers: VariableScope,
    value: Option<String>,
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

fn access_allowed<T: Into<String>>(scope: &VariableScope, entry: T) -> bool {
    match scope {
        VariableScope::Global => true,
        VariableScope::Restricted(allowed) => allowed.contains(&entry.into()),
    }
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

    fn read_value_unchecked(&self) -> anyhow::Result<String> {
        match &self.value {
            Some(value) => Ok(value.clone()),
            None => match &self.default {
                Some(value) => Ok(value.clone()),
                None => bail!(VariableError::NoDefaultValueSet(self.name.clone())),
            },
        }
    }

    pub fn read_value(&self, reader: &str) -> anyhow::Result<String> {
        if access_allowed(&self.readers, reader) {
            return self.read_value_unchecked();
        }
        bail!(VariableError::ReadNotAllowed {
            reader: reader.to_string(),
            name: self.name().to_owned(),
        });
    }

    fn write_value_unchecked<T: Into<String>>(&mut self, value: T) {
        self.value = Some(value.into());
    }

    pub fn write_value<T: Into<String>>(&mut self, value: T, writer: &str) -> anyhow::Result<()> {
        if access_allowed(&self.writers, writer) {
            self.write_value_unchecked(value);
            Ok(())
        } else {
            bail!(VariableError::WriteNotAllowed {
                writer: writer.to_string(),
                name: self.name().to_owned(),
            })
        }
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
            value: None,
        })
    }
}

#[derive(Debug)]
pub struct FrozenVariable {
    pub name: String,
    pub default: Option<String>,
    pub env: Option<String>,
    pub cli_flag: Option<String>,
    pub readers: VariableScope,
    pub writers: VariableScope,
    pub value: Option<String>,
}

impl From<&Variable> for FrozenVariable {
    fn from(item: &Variable) -> Self {
        FrozenVariable {
            name: item.name(),
            default: item.default.clone(),
            env: item.env.clone(),
            cli_flag: item.cli_flag.clone(),
            readers: item.readers.clone(),
            writers: item.writers.clone(),
            value: match item.read_value_unchecked() {
                Ok(v) => Some(v),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;
    use starlark::syntax::{AstModule, Dialect};

    impl VariableScope {
        fn from_str_list(values: &[&str]) -> Self {
            VariableScope::Restricted(values.into_iter().map(|v| v.to_string()).collect())
        }
    }

    #[test]
    fn test_variable_scope_display() {
        assert_eq!("Global", format!("{}", VariableScope::Global));
        assert_eq!(
            "Restricted: [a, b]",
            format!("{}", VariableScope::from_str_list(&["a", "b"]))
        );
    }

    #[test]
    fn test_variable_scope_default() {
        assert_eq!(VariableScope::default(), VariableScope::Global);
    }

    #[test]
    fn test_variable_scope_from_str_list() {
        assert_eq!(
            VariableScope::Restricted(vec!["a".to_string(), "b".to_string()]),
            VariableScope::from_str_list(&["a", "b"])
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
            VariableScope::from_str_list(&["a", "b"]),
            VariableScope::from_str_list(&["a", "b"])
        );
    }

    #[test]
    fn test_variable_scope_allowed() {
        let global = VariableScope::Global;
        assert_eq!(access_allowed(&global, "foo"), true);
        assert_eq!(access_allowed(&global, "".to_string()), true);

        let restricted = VariableScope::from_str_list(&["a", "b"]);
        assert_eq!(access_allowed(&restricted, "a"), true);
        assert_eq!(access_allowed(&restricted, "b"), true);
        assert_eq!(access_allowed(&restricted, "b".to_string()), true);
        assert_eq!(access_allowed(&restricted, "c"), false);
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
            VariableScope::from_str_list(&["a", "b"]),
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

    // - Value

    #[test]
    fn empty_value_returns_default() {
        let var = Variable {
            default: Some("default".to_string()),
            ..Variable::default()
        };
        assert_eq!(var.read_value_unchecked().unwrap(), "default".to_string());
    }

    #[test]
    #[should_panic(expected = "No value set for ''")]
    fn empty_value_returns_fails_if_no_default() {
        let var = Variable::default();
        var.read_value_unchecked().unwrap();
    }

    #[test]
    fn read_value_succes_if_in_scope() {
        let var = Variable {
            default: Some("default".to_string()),
            readers: VariableScope::from_str_list(&["foo"]),
            ..Variable::default()
        };
        assert_eq!(var.read_value("foo").unwrap(), "default".to_string());
    }

    #[test]
    #[should_panic(expected = "'bar' not in allowed scopes to read var ''")]
    fn read_value_fails_if_not_in_scope() {
        let var = Variable {
            readers: VariableScope::from_str_list(&["foo"]),
            default: Some("".to_string()),
            ..Variable::default()
        };
        var.read_value("bar").unwrap();
    }

    #[test]
    fn write_value_success() {
        let mut var = Variable {
            default: Some("default".to_string()),
            ..Variable::default()
        };
        var.write_value_unchecked("new");
        assert_eq!(var.read_value_unchecked().unwrap(), "new".to_string());
    }

    #[test]
    fn write_value_success_no_default() {
        let mut var = Variable {
            ..Variable::default()
        };
        var.write_value_unchecked("new");
        assert_eq!(var.read_value_unchecked().unwrap(), "new".to_string());

        var.write_value_unchecked("next".to_string());
        assert_eq!(var.read_value_unchecked().unwrap(), "next".to_string());
    }

    #[test]
    #[should_panic(expected = "'bar' not in allowed scopes to write var ''")]
    fn write_value_fails_if_not_in_scope() {
        let mut var = Variable {
            writers: VariableScope::from_str_list(&["foo"]),
            ..Variable::default()
        };
        var.write_value("x", "bar").unwrap();
    }

    #[test]
    fn write_value_success_if_in_scope() {
        let mut var = Variable {
            writers: VariableScope::from_str_list(&["foo"]),
            ..Variable::default()
        };
        var.write_value("x", "foo").unwrap();
        assert_eq!(var.read_value_unchecked().unwrap(), "x".to_string());
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

        assert_eq!(ctx.snapshot_variables().len(), 2);
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
                    readers: VariableScope::from_str_list(&["a", "b"]),
                    writers: VariableScope::from_str_list(&["c", "d"]),
                    ..Variable::default()
                }
            );
            Ok(())
        });
    }

    // -- Frozen Variables
    #[test]
    fn test_frozen_variable() {
        let var = Variable {
            name: "foo".to_string(),
            default: Some("default".to_string()),
            env: Some("FOO".to_string()),
            cli_flag: Some("--foo".to_string()),
            readers: VariableScope::from_str_list(&["a", "b"]),
            writers: VariableScope::from_str_list(&["c", "d"]),
            ..Variable::default()
        };
        let frozen = FrozenVariable::from(&var);
        assert_eq!(frozen.name, var.name);
        assert_eq!(frozen.default, var.default);
        assert_eq!(frozen.env, var.env);
        assert_eq!(frozen.cli_flag, var.cli_flag);
        assert_eq!(frozen.readers, var.readers);
        assert_eq!(frozen.writers, var.writers);
        assert_eq!(frozen.value, var.default);
    }
}
