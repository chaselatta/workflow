use crate::stdlib::variable_resolver::{string_from_value, VariableResolver};
use allocative::Allocative;
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
use which::which;

pub(crate) fn tool_impl<'v>(path: Value<'v>) -> anyhow::Result<Tool<'v>> {
    Ok(Tool {
        path: path,
        builtin: false,
        name: "".to_string(),
    })
}

pub(crate) fn builtin_tool_impl<'v>(name: &str) -> anyhow::Result<Tool<'v>> {
    Ok(Tool {
        path: Value::new_none(),
        builtin: true,
        name: name.to_string(),
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct ToolGen<V> {
    builtin: bool,
    path: V,
    // name is only valid if builtin is true
    name: String,
}
starlark_complex_value!(pub Tool);

pub const TOOL_TYPE: &str = "tool";
#[starlark_value(type = TOOL_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for ToolGen<V> where Self: ProvidesStaticType<'v> {}

impl<'a> Tool<'a> {
    /// Returns the real path of the tool. Will return an error if the path does not
    /// resolve to an executable.
    pub fn real_path<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: &PathBuf,
    ) -> anyhow::Result<PathBuf> {
        let path = self.path(resolver, &working_dir)?;
        Ok(which(&path)?)
    }

    /// Returns the path of the tool. This tool is the raw path and is not validated.
    pub fn path<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: &PathBuf,
    ) -> anyhow::Result<PathBuf> {
        if self.builtin {
            Ok(PathBuf::from(self.name.clone()))
        } else {
            let path = PathBuf::from(string_from_value(self.path, resolver)?);

            Ok({
                if path.is_absolute() {
                    path
                } else {
                    let mut new_path = working_dir.clone();
                    new_path.push(path);
                    new_path
                }
            })
        }
    }

    pub fn is_builtin(&self) -> bool {
        self.builtin
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'v> Freeze for Tool<'v> {
    type Frozen = FrozenTool;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(ToolGen {
            path: self.path.freeze(freezer)?,
            builtin: self.builtin.freeze(freezer)?,
            name: self.name.freeze(freezer)?,
        })
    }
}

impl<V> Display for ToolGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tool")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::{assert_env, TempWorkflowFile};

    #[test]
    fn test_can_parse_simple_tool() {
        assert_env().pass("tool(path='a')");
    }

    #[test]
    fn test_can_parse_formatter_in_tool() {
        assert_env().pass("v = variable(); tool(path=format('{}', v))");
    }

    #[test]
    fn test_get_path_from_tool_with_fmt() {
        let mut env = assert_env();
        let module = env.module(
            "tool.star",
            "v = variable(); t = tool(path=format('{}', v))",
        );
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();

        assert_eq!(
            tool.path(&"foo".to_string(), &PathBuf::default()).unwrap(),
            PathBuf::from("foo".to_string())
        );
    }

    #[test]
    fn test_get_path_from_tool_with_variable() {
        let mut env = assert_env();
        let module = env.module("tool.star", "v = variable(); t = tool(path=v)");
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();

        assert_eq!(
            tool.path(&"foo".to_string(), &PathBuf::default()).unwrap(),
            PathBuf::from("foo".to_string())
        );
    }

    #[test]
    fn test_path_based_tool_real_path_absolute() {
        let exe = TempWorkflowFile::new_executable("foo.sh", "").unwrap();
        let mut env = assert_env();
        let module = env.module(
            "tool.star",
            "v = variable(); t = tool(path=format('{}/foo.sh', v))",
        );
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();
        let dir = exe.dir();
        let resolver = dir.as_os_str().to_str().unwrap();
        assert_eq!(
            tool.real_path(&resolver.to_string(), &PathBuf::default())
                .unwrap(),
            PathBuf::from(format!("{}/foo.sh", resolver))
        );
    }

    #[test]
    #[should_panic]
    fn test_path_based_tool_real_path_absolute_fail() {
        let exe = TempWorkflowFile::new_executable("foo.sh", "").unwrap();
        let mut env = assert_env();
        let module = env.module(
            "tool.star",
            "v = variable(); t = tool(path=format('{}/__no_file__.sh', v))",
        );
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();
        let dir = exe.dir();
        let resolver = dir.as_os_str().to_str().unwrap();
        tool.real_path(&resolver.to_string(), &PathBuf::default())
            .unwrap();
    }

    #[test]
    fn test_path_based_tool_real_path_relative() {
        let exe = TempWorkflowFile::new_executable("foo.sh", "").unwrap();
        let mut env = assert_env();
        let module = env.module("tool.star", "v = 'foo.sh'");

        let root = exe.dir();

        // Use this approach so we can supply our own root
        let v = module.get("v").unwrap();
        let tool = tool_impl(v.value()).unwrap();

        assert_eq!(tool.real_path(&"".to_string(), &root).unwrap(), exe.path());
    }

    #[test]
    fn test_builtin_tool_name_returns_name() {
        let mut env = assert_env();
        let module = env.module("tool.star", "t = builtin_tool(name= 'ls')");

        // Use this approach so we can supply our own root
        let t = module.get("t").unwrap();
        let tool = Tool::from_value(t.value()).unwrap();
        assert_eq!(tool.name(), "ls");
    }

    #[test]
    fn test_builtin_tool_real_path() {
        let mut env = assert_env();
        let module = env.module("tool.star", "t = builtin_tool(name= 'ls')");

        // Use this approach so we can supply our own root
        let t = module.get("t").unwrap();
        let tool = Tool::from_value(t.value()).unwrap();
        assert_eq!(
            //make sure we pass in a pathbuf to make sure the code uses the name
            tool.real_path(&"".to_string(), &PathBuf::from("foo"))
                .unwrap(),
            which("ls").unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn test_builtin_tool_real_path_fail() {
        let mut env = assert_env();
        let module = env.module("tool.star", "t = builtin_tool(name= '__INVALID_TOOL__')");

        // Use this approach so we can supply our own root
        let t = module.get("t").unwrap();
        let tool = Tool::from_value(t.value()).unwrap();
        tool.real_path(&"".to_string(), &PathBuf::default())
            .unwrap();
    }
}
