use crate::stdlib::errors::StdlibError;
use crate::stdlib::{ParseDelegateHolder, VARIABLE_REF_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::eval::Evaluator;
use starlark::starlark_simple_value;
use starlark::values::list::ListOf;
use starlark::values::starlark_value;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use std::fmt;
use std::ops::Deref;
use uuid::Uuid;

pub(crate) fn variable_impl(
    default: Option<&str>,
    env: Option<&str>,
    cli_flag: Option<&str>,
    readers: Option<ListOf<String>>,
    writers: Option<ListOf<String>>,
    eval: &mut Evaluator,
) -> anyhow::Result<VariableRef> {
    let var_ref = VariableRef::new();

    if let Ok(delegate) = ParseDelegateHolder::from_evaluator(&eval) {
        delegate.deref().on_variable(
            var_ref.identifier(),
            VariableEntry::from_starlark(default, env, cli_flag, readers, writers)?,
        );
    }
    Ok(var_ref)
}

/// A value that is returned when creating a variable. The VariableRef can be
/// later used in a starlark context.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct VariableRef {
    identifier: String,
}
starlark_simple_value!(VariableRef);

#[starlark_value(type = VARIABLE_REF_TYPE )]
impl<'v> StarlarkValue<'v> for VariableRef {}

impl fmt::Display for VariableRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl VariableRef {
    fn new() -> Self {
        let id = Uuid::new_v4();
        VariableRef {
            identifier: id.to_string().to_owned(),
        }
    }
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ValueUpdatedBy {
    CLIFlag(String),
    EnvironmentVariable(String),
    Action(String),
    DefaultValue,

    #[cfg(test)]
    ForTest,
}

impl fmt::Display for ValueUpdatedBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueUpdatedBy::CLIFlag(v) => write!(f, "Updated by command line flag '{}'", v),
            ValueUpdatedBy::EnvironmentVariable(v) => {
                write!(f, "Updated by environment variable '{}'", v)
            }
            ValueUpdatedBy::Action(v) => write!(f, "Updated by action with name'{}'", v),
            ValueUpdatedBy::DefaultValue => write!(f, "Updated by default value"),

            #[cfg(test)]
            ValueUpdatedBy::ForTest => write!(f, "for testing"),
        }
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

/// A Context holding a variable
#[derive(Debug, PartialEq, Clone)]
pub struct ValueContext {
    pub value: String,
    pub updated_by: ValueUpdatedBy,
}

impl ValueContext {
    fn new<T: Into<String>>(value: T, updated_by: ValueUpdatedBy) -> Self {
        ValueContext {
            value: value.into(),
            updated_by: updated_by,
        }
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct VariableEntry {
    value_ctx: Option<ValueContext>,
    env: Option<String>,
    cli_flag: Option<String>,
    readers: VariableScope,
    writers: VariableScope,
}

impl VariableEntry {
    fn from_starlark(
        default: Option<&str>,
        env: Option<&str>,
        cli_flag: Option<&str>,
        readers: Option<ListOf<String>>,
        writers: Option<ListOf<String>>,
    ) -> anyhow::Result<Self> {
        Ok(VariableEntry {
            env: VariableEntry::validate_env(env)?,
            cli_flag: VariableEntry::validate_cli_flag(cli_flag)?,
            readers: VariableEntry::validate_scope(readers.map(|v| v.to_vec()))?,
            writers: VariableEntry::validate_scope(writers.map(|v| v.to_vec()))?,
            value_ctx: default.map(|d| ValueContext::new(d, ValueUpdatedBy::DefaultValue)),
        })
    }

    pub fn update_value<T: Into<String>>(&mut self, val: T, updated_by: ValueUpdatedBy) {
        self.value_ctx = Some(ValueContext::new(val, updated_by));
    }

    pub fn value(&self) -> Option<String> {
        self.value_ctx.clone().map(|ctx| ctx.value)
    }

    pub fn value_ctx(&self) -> Option<ValueContext> {
        self.value_ctx.clone()
    }

    pub fn env(&self) -> Option<String> {
        self.env.clone()
    }

    pub fn cli_flag(&self) -> Option<String> {
        self.cli_flag.clone()
    }

    pub fn readers(&self) -> VariableScope {
        self.readers.clone()
    }

    pub fn writers(&self) -> VariableScope {
        self.writers.clone()
    }

    #[cfg(test)]
    pub fn for_test(default: Option<&str>, cli_flag: Option<&str>, env: Option<&str>) -> Self {
        VariableEntry {
            env: env.map(|v| v.to_string()),
            cli_flag: cli_flag.map(|v| v.to_string()),
            value_ctx: default.map(|v| ValueContext::new(v, ValueUpdatedBy::ForTest)),
            ..VariableEntry::default()
        }
    }

    fn validate_env(env: Option<&str>) -> anyhow::Result<Option<String>> {
        if let Some(env) = env {
            if env.is_empty() {
                bail!(StdlibError::new_invalid_attr("env", "cannot be empty", env));
            }
            if env.contains(" ") {
                bail!(StdlibError::new_invalid_attr(
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
                bail!(StdlibError::new_invalid_attr(
                    "cli_flag",
                    "cannot be empty",
                    flag
                ));
            }

            if flag.contains(" ") {
                bail!(StdlibError::new_invalid_attr(
                    "cli_flag",
                    "cannot contain spaces",
                    flag
                ));
            }

            if flag.len() == 2 && (!flag.starts_with("-") || flag == "--") {
                bail!(StdlibError::new_invalid_attr(
                    "cli_flag",
                    "short flags must take the form -v",
                    flag
                ));
            }
            if flag.len() > 2 && !flag.starts_with("--") {
                bail!(StdlibError::new_invalid_attr(
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
                    bail!(StdlibError::new_invalid_attr(
                        "scope",
                        "scopes cannot contain empty strings",
                        scope
                    ));
                }

                if scope.contains(" ") {
                    bail!(StdlibError::new_invalid_attr(
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

    pub fn try_update_value_from_env(&mut self) -> anyhow::Result<()> {
        if let Some(key) = &self.env {
            if let Ok(val) = std::env::var(key) {
                self.update_value(val, ValueUpdatedBy::EnvironmentVariable(key.to_string()));
            } else {
                bail!("Cannot update variable from environemnt: '{}' has no associated environment variable", key);
            }
        } else {
            bail!("Cannot update from environemnt: no env set for this variable",);
        }
        Ok(())
    }

    pub fn try_update_value_from_cli_flag(&mut self, args: &Vec<String>) -> anyhow::Result<()> {
        if let Some(cli_flag) = &self.cli_flag {
            if let Some(value) = VariableEntry::find_cli_flag_value(cli_flag, args) {
                self.update_value(value, ValueUpdatedBy::CLIFlag(cli_flag.clone()));
            } else {
                bail!("Cannot update from cli_flag: '{}' is not in args", cli_flag,);
            }
        } else {
            bail!("Cannot update from cli_flag: no cli_flag set for this variable",)
        }
        Ok(())
    }

    fn find_cli_flag_value(flag: &str, workflow_args: &Vec<String>) -> Option<String> {
        let mut iter = workflow_args.into_iter();
        while let Some(val) = iter.next() {
            if val == flag {
                return iter.next().cloned();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::downcast_delegate_ref;
    use crate::stdlib::starlark_stdlib;
    use crate::stdlib::test_utils::TestParseDelegate;
    use crate::stdlib::test_utils::{assert_env, TempEnvVar};
    use starlark::environment::{GlobalsBuilder, Module};
    use starlark::syntax::AstModule;
    use starlark::syntax::Dialect;
    use std::ops::Deref;

    #[test]
    fn test_can_parse_simple_variable() {
        assert_env().pass("variable()");
    }

    #[test]
    fn test_can_parse_with_all_values() {
        assert_env().pass(
            r#"
variable(
  default = "value",
  readers =  ["foo", "bar"],
  writers =  ["foo", "bar"],
  env =  "VAR_TWO",
  cli_flag = "--foo",
)
"#,
        );
    }

    #[test]
    fn test_variable_ref_type() {
        assert_env().eq("type(variable())", "'variable_ref'");
    }

    #[test]
    fn test_unique_identifiers() {
        let mut env = assert_env();
        let module = env.module("variable.star", "a = variable(); b = variable()");
        let a_frozen = module.get("a").unwrap();
        let a = VariableRef::from_value(a_frozen.value()).unwrap();
        let b_frozen = module.get("b").unwrap();
        let b = VariableRef::from_value(b_frozen.value()).unwrap();

        assert_ne!(a.identifier(), b.identifier());
    }

    #[test]
    fn test_delegate_function_called() {
        let module: Module = Module::new();

        let delegate = TestParseDelegate::default();
        let holder = ParseDelegateHolder::new(delegate);
        let mut eval: Evaluator = Evaluator::new(&module);

        eval.extra = Some(&holder);

        let content = "variable(); variable()";

        let ast = AstModule::parse("test.star", content.to_string(), &Dialect::Standard).unwrap();
        let globals = GlobalsBuilder::standard().with(starlark_stdlib).build();
        eval.eval_module(ast, &globals).unwrap();

        assert_eq!(
            downcast_delegate_ref!(holder, TestParseDelegate)
                .unwrap()
                .on_variable_call_count,
            2.into()
        );
    }

    // --- env

    #[test]
    fn validate_env_success() {
        assert_eq!(
            VariableEntry::validate_env(Some("foo")).unwrap().unwrap(),
            "foo".to_string()
        );
        assert_eq!(
            VariableEntry::validate_env(Some("1")).unwrap().unwrap(),
            "1".to_string()
        );
        assert_eq!(VariableEntry::validate_env(None).unwrap(), None);
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_empty() {
        VariableEntry::validate_env(Some("")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_spaces() {
        VariableEntry::validate_env(Some("a b")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_env_fail_spaces_only() {
        VariableEntry::validate_env(Some(" ")).unwrap();
    }

    #[test]
    fn test_try_update_value_from_env_success() {
        let env = TempEnvVar::new(
            "ENV_VAR_FOR_test_try_update_value_from_env_success",
            "some_value",
        );
        let mut var = VariableEntry::for_test(
            /* default */ Some("default"),
            /* cli_flag */ None,
            /* env */ Some(&env.key.clone()),
        );
        assert_eq!(var.value().unwrap(), "default");
        var.try_update_value_from_env().unwrap();
        assert_eq!(var.value().unwrap(), "some_value");
    }

    #[test]
    #[should_panic(
        expected = "Cannot update variable from environemnt: 'NUL' has no associated environment variable"
    )]
    fn test_try_update_value_from_env_fail_invalid_env_set() {
        let mut var = VariableEntry::for_test(
            /* default */ None,
            /* cli_flag */ None,
            /* env */ Some("NUL"),
        );
        var.try_update_value_from_env().unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot update from environemnt: no env set for this variable")]
    fn test_try_update_value_from_env_fail_no_env_set() {
        let mut var = VariableEntry::for_test(
            /* default */ None, /* cli_flag */ None, /* env */ None,
        );
        var.try_update_value_from_env().unwrap();
    }

    // --- cli_flag

    #[test]
    fn validate_cli_flag_success() {
        assert_eq!(
            VariableEntry::validate_cli_flag(Some("--foo"))
                .unwrap()
                .unwrap(),
            "--foo".to_string()
        );
        assert_eq!(
            VariableEntry::validate_cli_flag(Some("-v"))
                .unwrap()
                .unwrap(),
            "-v".to_string()
        );
        assert_eq!(VariableEntry::validate_cli_flag(None).unwrap(), None);
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_empty() {
        VariableEntry::validate_cli_flag(Some("")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_spaces() {
        VariableEntry::validate_cli_flag(Some("a b")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_missing_dashes() {
        VariableEntry::validate_cli_flag(Some("foo")).unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_cli_flag_fail_two_dashes() {
        VariableEntry::validate_cli_flag(Some("--")).unwrap();
    }

    #[test]
    fn test_try_update_value_from_cli_flag_short_success() {
        let mut var = VariableEntry::for_test(
            /* default */ Some("default"),
            /* cli_flag */ Some("-f"),
            /* env */ None,
        );
        assert_eq!(var.value().unwrap(), "default".to_string());
        var.try_update_value_from_cli_flag(&vec![
            "--bar".to_string(),
            "a".to_string(),
            "-f".to_string(),
            "foo".to_string(),
        ])
        .unwrap();
        assert_eq!(var.value().unwrap(), "foo".to_string());
    }

    #[test]
    fn test_try_update_value_from_cli_flag_long_success() {
        let mut var = VariableEntry::for_test(
            /* default */ Some("default"),
            /* cli_flag */ Some("--foo"),
            /* env */ None,
        );
        assert_eq!(var.value().unwrap(), "default".to_string());
        var.try_update_value_from_cli_flag(&vec![
            "--bar".to_string(),
            "a".to_string(),
            "--foo".to_string(),
            "foo".to_string(),
        ])
        .unwrap();
        assert_eq!(var.value().unwrap(), "foo".to_string());
    }

    #[test]
    #[should_panic(expected = "Cannot update from cli_flag: no cli_flag set for this variable")]
    fn test_try_update_value_from_cli_flag_fail_not_set() {
        let mut var = VariableEntry::default();
        var.try_update_value_from_cli_flag(&vec![]).unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot update from cli_flag: '--foo' is not in args")]
    fn test_try_update_value_from_cli_flag_fail_not_in_args() {
        let mut var = VariableEntry::for_test(
            /* default */ Some("default"),
            /* cli_flag */ Some("--foo"),
            /* env */ None,
        );
        var.try_update_value_from_cli_flag(&vec![]).unwrap();
    }

    // - Value

    #[test]
    fn empty_value_returns_default() {
        let var = VariableEntry::for_test(
            /* default */ Some("default"),
            /* cli_flag */ None,
            /* env */ None,
        );
        assert_eq!(var.value().unwrap(), "default".to_string());
    }

    #[test]
    fn empty_value_returns_none_if_no_default() {
        let var = VariableEntry::default();
        assert_eq!(var.value(), None);
    }
}
