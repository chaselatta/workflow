use crate::stdlib::{ValueFormatter, VariableRef};
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

use super::variable_resolver::VariableResolver;

pub(crate) fn tool_impl<'v>(path: Value<'v>) -> anyhow::Result<Tool<'v>> {
    Ok(Tool {
        path: path,
        builtin: false,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct ToolGen<V> {
    builtin: bool,
    path: V,
}
starlark_complex_value!(pub Tool);

#[starlark_value(type = "tool")]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for ToolGen<V> where Self: ProvidesStaticType<'v> {}

impl<'a> Tool<'a> {
    /// Returns the real path of the tool. Will return an error if the path does not
    /// resolve to an executable.
    pub fn real_path<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: PathBuf,
    ) -> anyhow::Result<PathBuf> {
        // if self.builtin {
        // bail!("not implemented")
        // } else {
        let path = self.path(resolver, working_dir)?;
        Ok(which(&path)?)
        // }
    }

    /// Returns the path of the tool. This tool is the raw path and is not validated.
    pub fn path<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: PathBuf,
    ) -> anyhow::Result<PathBuf> {
        let path = PathBuf::from({
            if let Some(formatter) = ValueFormatter::from_value(self.path) {
                formatter.fmt(resolver)?
            } else if let Some(var_ref) = VariableRef::from_value(self.path) {
                resolver.resolve(var_ref.identifier())?
            } else {
                self.path.to_str()
            }
        });

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

impl<'v> Freeze for Tool<'v> {
    type Frozen = FrozenTool;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(ToolGen {
            path: self.path.freeze(freezer)?,
            builtin: self.builtin.freeze(freezer)?,
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
    use crate::stdlib::test_utils::assert_env;

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
            tool.path(&"foo".to_string(), PathBuf::default()).unwrap(),
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
            tool.path(&"foo".to_string(), PathBuf::default()).unwrap(),
            PathBuf::from("foo".to_string())
        );
    }

    #[test]
    fn test_validate_path_based_tool_path_absolute() {
        let mut env = assert_env();
        let module = env.module(
            "tool.star",
            "v = variable(); t = tool(path=format('{}/src/test_data/foo.sh', v))",
        );
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();
        let resolver = env!("CARGO_MANIFEST_DIR");
        assert_eq!(
            tool.real_path(&resolver.to_string(), PathBuf::default())
                .unwrap(),
            PathBuf::from(format!("{}/src/test_data/foo.sh", resolver))
        );
    }

    #[test]
    #[should_panic]
    fn test_validate_path_based_tool_path_absolute_fail() {
        let mut env = assert_env();
        let module = env.module(
            "tool.star",
            "v = variable(); t = tool(path=format('{}/src/test_data/__no_file__.sh', v))",
        );
        let tool = module.get("t").unwrap();
        let tool = Tool::from_value(tool.value()).unwrap();
        let resolver = env!("CARGO_MANIFEST_DIR");
        tool.real_path(&resolver.to_string(), PathBuf::default())
            .unwrap();
    }

    #[test]
    fn test_validate_path_based_tool_path_relative() {
        let mut env = assert_env();
        let module = env.module("tool.star", "v = 'test_data/foo.sh'");

        // Note, we cannot use the CARGO_MANIFEST_DIR here since it is the working
        // directory for the tests so we just nest one level deeper.
        let root = env!("CARGO_MANIFEST_DIR").to_owned() + "/src";
        // Use this approach so we can supply our own root
        let v = module.get("v").unwrap();
        let tool = tool_impl(v.value()).unwrap();

        assert_eq!(
            tool.real_path(&"".to_string(), PathBuf::from(&root))
                .unwrap(),
            PathBuf::from(format!("{}/test_data/foo.sh", root))
        );
    }
}
