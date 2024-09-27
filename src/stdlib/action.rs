use crate::stdlib::variable_resolver::{string_from_value, VariableResolver};
use crate::stdlib::{Tool, ACTION_TYPE, TOOL_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::coerce::Coerce;
use starlark::starlark_complex_value;
use starlark::values::starlark_value;
use starlark::values::Freeze;
use starlark::values::Freezer;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark::StarlarkDocs;
use std::fmt;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn action_impl<'v>(tool: Value<'v>, args: Vec<Value<'v>>) -> anyhow::Result<Action<'v>> {
    if tool.get_type() != TOOL_TYPE {
        bail!("A tool must be passed as the tool in an action")
    }

    Ok(Action {
        tool: tool,
        args: args,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct ActionGen<V> {
    tool: V,
    args: Vec<V>,
}
starlark_complex_value!(pub Action);

#[starlark_value(type = ACTION_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for ActionGen<V> where Self: ProvidesStaticType<'v>
{}

impl<'a> Action<'a> {
    pub fn arg_list<T: VariableResolver>(&self, resolver: &T) -> anyhow::Result<Vec<String>> {
        let mut args_list: Vec<String> = Vec::new();
        for v in self.args.clone() {
            let r = string_from_value(v, resolver)?;
            args_list.push(r);
        }
        Ok(args_list)
    }

    pub fn command<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: &PathBuf,
    ) -> anyhow::Result<Command> {
        let tool = Tool::from_value(self.tool.clone()).unwrap();
        let program = tool.real_path(resolver, working_dir)?.into_os_string();

        let mut cmd = Command::new(program);
        for arg in self.arg_list(resolver)? {
            cmd.arg(arg);
        }

        Ok(cmd)
    }
}

impl<'v> Freeze for Action<'v> {
    type Frozen = FrozenAction;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(ActionGen {
            tool: self.tool.freeze(freezer)?,
            args: self.args.freeze(freezer)?,
        })
    }
}

impl<V> Display for ActionGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "action")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::assert_env;
    use std::ffi::OsStr;
    use which::which;

    #[test]
    fn test_can_parse_simple_action() {
        assert_env().pass("t = tool(path='foo'); action(tool=t)");
    }

    #[test]
    fn test_require_a_tool() {
        assert_env().fail(
            "action(tool='tool')",
            "A tool must be passed as the tool in an action",
        );
    }

    #[test]
    fn test_get_complex_args() {
        let mut env = assert_env();
        let module = env.module(
            "action.star",
            r#"
t = tool(path = "foo")
v = variable()
a = action(
  tool = t,
  args = [
    v,
    format("--{}", v),
    "some string",
  ]
)
"#,
        );
        let action = module.get("a").unwrap();
        let action = Action::from_value(action.value()).unwrap();

        let result = action.arg_list(&"abc").unwrap();
        let expected = vec![
            "abc".to_string(),
            "--abc".to_string(),
            "some string".to_string(),
        ];

        assert_eq!(&result, &expected);
    }

    #[test]
    fn test_get_tool_path() {
        let res = assert_env().pass(
            r#"
t = builtin_tool(name = "ls")
action(
  tool = t,
  args = [
    ".",
  ]
)
"#,
        );
        let action = Action::from_value(res.value()).unwrap();
        let command = action.command(&"", &PathBuf::new()).unwrap();

        assert_eq!(command.get_program(), which("ls").unwrap());

        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(args, &["."]);
    }
}
