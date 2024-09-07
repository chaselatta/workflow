use crate::stdlib::parser::parse_context::ParseContext;
use crate::stdlib::parser::StringInterpolator;
use crate::stdlib::{validate_name, StdlibError};
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::none::NoneType;
use std::path::PathBuf;
use which::which;

use anyhow::bail;

#[starlark_module]
pub fn starlark_tool(builder: &mut GlobalsBuilder) {
    fn tool(
        #[starlark(require = named)] name: &str,
        #[starlark(require = named)] path: &str,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let tool = Tool::path_based(name, path)?;
        let ctx = ParseContext::from_evaluator(eval)?;
        ctx.add_tool(tool)?;
        Ok(NoneType)
    }
}

#[starlark_module]
pub fn starlark_builtin_tool(builder: &mut GlobalsBuilder) {
    fn builtin_tool(
        #[starlark(require = named)] name: &str,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let tool = Tool::builtin(name)?;
        let ctx = ParseContext::from_evaluator(eval)?;
        ctx.add_tool(tool)?;
        Ok(NoneType)
    }
}

/// A type representing a tool used in a workflow
#[derive(Debug, PartialEq, Default)]
pub struct Tool {
    name: String,
    builtin: bool,
    path: Option<String>,
}

impl Tool {
    fn builtin(name: &str) -> anyhow::Result<Self> {
        Ok(Tool {
            name: validate_name(name)?,
            builtin: true,
            path: None,
        })
    }

    fn path_based(name: &str, path: &str) -> anyhow::Result<Self> {
        Ok(Tool {
            name: validate_name(name)?,
            path: validate_path(path)?,
            builtin: false,
        })
    }

    pub fn name(&self) -> String {
        self.name.to_owned().clone()
    }

    pub fn cmd<T: StringInterpolator>(
        &self,
        workflow_path: &PathBuf,
        interpolator: &T,
    ) -> Option<PathBuf> {
        if self.builtin {
            return which(&self.name).ok();
        } else if let Some(path) = self
            .path
            .as_ref()
            .map(|s| interpolator.interpolate(s, &self.name))
            .map(|v| v.ok())
            .flatten()
            .map(PathBuf::from)
        {
            // path based so find out the full path
            let full_path = {
                if path.is_absolute() {
                    path.clone()
                } else {
                    let mut new_path = workflow_path.clone();
                    new_path.push(path);
                    new_path
                }
            };
            return which(&full_path).ok();
        } else {
            None
        }
    }

    pub fn freeze<T: StringInterpolator>(
        &self,
        workflow_path: &PathBuf,
        interpolator: &T,
    ) -> FrozenTool {
        FrozenTool {
            name: self.name.clone(),
            builtin: self.builtin,
            path: self.path.clone(),
            cmd: self.cmd(workflow_path, interpolator),
        }
    }

    #[cfg(test)]
    pub fn for_test(name: &str) -> Self {
        Tool {
            name: name.to_string(),
            path: None,
            builtin: true,
        }
    }

    #[cfg(test)]
    pub fn for_test_path_based(name: &str, path: &str) -> Self {
        Tool {
            name: name.to_string(),
            path: Some(path.to_string()),
            builtin: false,
        }
    }
}

fn validate_path(path: &str) -> anyhow::Result<Option<String>> {
    if path.is_empty() {
        bail!(StdlibError::new_invalid_attr(
            "path",
            "cannot be empty",
            path
        ));
    }
    if path.contains(" ") {
        bail!(StdlibError::new_invalid_attr(
            "path",
            "cannot contain spaces",
            path
        ));
    }
    return Ok(Some(path.to_string()));
}

#[derive(Debug)]
pub struct FrozenTool {
    pub name: String,
    pub builtin: bool,
    pub path: Option<String>,
    pub cmd: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::parser::NO_STRING_INTERP;
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempdir::TempDir;

    #[test]
    fn test_builtin_pass() {
        assert_eq!(
            Tool::builtin("foo").unwrap(),
            Tool {
                name: "foo".to_string(),
                path: None,
                builtin: true,
            }
        );
    }

    #[test]
    #[should_panic(expected = "Invalid attribute 'name', cannot be empty got \"\"")]
    fn test_builtin_fail() {
        Tool::builtin("").unwrap();
    }

    #[test]
    fn test_path_based_pass() {
        assert_eq!(
            Tool::path_based("foo", "my/path").unwrap(),
            Tool {
                name: "foo".to_string(),
                path: Some("my/path".to_string()),
                builtin: false,
            }
        );
    }

    #[test]
    #[should_panic(expected = "Invalid attribute 'name', cannot be empty got \"\"")]
    fn test_path_based_fail_empty_name() {
        Tool::path_based("", "path").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid attribute 'path', cannot be empty got \"\"")]
    fn test_path_based_fail_empty_path() {
        Tool::path_based("foo", "").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid attribute 'path', cannot contain spaces got")]
    fn test_path_based_fail_spaces_in_path() {
        Tool::path_based("foo", "my path").unwrap();
    }

    #[test]
    fn test_frozen_tool() {
        let tool = Tool {
            name: "ls".to_string(),
            builtin: true,
            path: Some("foo".to_string()),
            // cmd: Some(PathBuf::from("foo")),
        };
        let frozen = tool.freeze(&PathBuf::default(), NO_STRING_INTERP);
        assert_eq!(frozen.name, tool.name);
        assert_eq!(frozen.builtin, tool.builtin);
        assert_eq!(frozen.path, tool.path);
        assert_eq!(frozen.cmd, which("ls").ok());
    }

    // - validation
    #[test]
    fn test_validate_builtin_tool_path() {
        let tools = ["ls", "echo", "["];
        for tool in tools {
            let t = Tool::builtin(tool).unwrap();
            assert_eq!(
                which(tool),
                Ok(t.cmd(&PathBuf::from(""), NO_STRING_INTERP)
                    .unwrap_or_default()),
                "testing which for tool '{}'",
                &tool
            );
        }
    }

    #[test]
    fn test_validate_path_based_tool_path_absolute() -> anyhow::Result<()> {
        // Create a temporary directory
        let tmp_dir = TempDir::new("")?;

        // Create a file in the temp dir that is executable
        let tmp_file_path = tmp_dir.path().join("foo.sh");
        let mut tmp_file = File::create(tmp_file_path.clone())?;
        let mut perms = tmp_file.metadata()?.permissions();
        perms.set_mode(0o755);
        tmp_file.set_permissions(perms)?;
        writeln!(tmp_file, "")?;

        // This mimics a user passing in an absolute path
        let absolute_tool_path = PathBuf::from(&tmp_file_path);
        let tool = Tool::path_based(
            "foo",
            &absolute_tool_path.into_os_string().into_string().unwrap(),
        )?;

        // the workflow path does not matter here since it is an absolute path
        assert_eq!(
            Some(PathBuf::from(&tmp_file_path)),
            tool.cmd(&PathBuf::default(), NO_STRING_INTERP)
        );

        // Delete all the files
        drop(tmp_file);
        tmp_dir.close()?;
        Ok(())
    }

    #[test]
    fn test_validate_path_based_tool_path_relative() -> anyhow::Result<()> {
        // Create a temporary directory
        let tmp_dir = TempDir::new("")?;
        let mut tool_absolute_path = PathBuf::from(tmp_dir.path());

        // create a directory in the temporary dir called foo
        tool_absolute_path.push("foo");
        fs::create_dir(&tool_absolute_path)?;

        // Create a file in the nested temp dir that is executable
        tool_absolute_path.push("foo.sh");
        let mut tmp_file = File::create(&tool_absolute_path)?;
        let mut perms = tmp_file.metadata()?.permissions();
        perms.set_mode(0o755);
        tmp_file.set_permissions(perms)?;
        writeln!(tmp_file, "")?;

        // This mimics a user writing a path relative to the workflow file
        let workflow_path = PathBuf::from(tmp_dir.path());
        let tool = Tool::path_based("foo", "foo/foo.sh")?;

        // Test
        assert_eq!(
            Some(tool_absolute_path),
            tool.cmd(&workflow_path, NO_STRING_INTERP)
        );

        // Delete all the files
        drop(tmp_file);
        tmp_dir.close()?;
        Ok(())
    }

    #[test]
    fn test_validate_path_does_nothing_if_unknown_file() -> anyhow::Result<()> {
        let tmp_dir = TempDir::new("")?;

        // This mimics a user writing a path relative to the workflow file
        let workflow_path = PathBuf::from(tmp_dir.path());
        let tool = Tool::path_based("foo", "foo.sh")?;

        // Test
        assert_eq!(tool.cmd(&workflow_path, NO_STRING_INTERP), None);

        // Delete the temp dir
        tmp_dir.close()?;
        Ok(())
    }

    #[test]
    fn test_validate_path_based_with_string_interpolation() -> anyhow::Result<()> {
        // Create a temporary directory
        let tmp_dir = TempDir::new("")?;
        let mut tool_absolute_path = PathBuf::from(tmp_dir.path());

        // create a directory in the temporary dir called foo
        tool_absolute_path.push("foo");
        fs::create_dir(&tool_absolute_path)?;

        // Create a file in the nested temp dir that is executable
        tool_absolute_path.push("foo.sh");
        let mut tmp_file = File::create(&tool_absolute_path)?;
        let mut perms = tmp_file.metadata()?.permissions();
        perms.set_mode(0o755);
        tmp_file.set_permissions(perms)?;
        writeln!(tmp_file, "")?;

        // This mimics a user writing a path relative to the workflow file
        let workflow_path = PathBuf::from(tmp_dir.path());
        let tool = Tool::path_based("foo", "{variable(p)}/foo.sh")?;

        assert_eq!(
            tool.cmd(&workflow_path, &"foo/foo.sh"),
            Some(tool_absolute_path)
        );

        // Delete all the files
        drop(tmp_file);
        tmp_dir.close()?;
        Ok(())
    }
}
