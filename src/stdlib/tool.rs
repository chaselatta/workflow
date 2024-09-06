use crate::stdlib::parser::parse_context::ParseContext;
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
    cmd: Option<PathBuf>,
}

impl Tool {
    fn builtin(name: &str) -> anyhow::Result<Self> {
        Ok(Tool {
            name: validate_name(name)?,
            builtin: true,
            path: None,
            cmd: None,
        })
    }

    fn path_based(name: &str, path: &str) -> anyhow::Result<Self> {
        Ok(Tool {
            name: validate_name(name)?,
            path: validate_path(path)?,
            builtin: false,
            cmd: None,
        })
    }

    pub fn name(&self) -> String {
        self.name.to_owned().clone()
    }

    // TODO: For string interpolation, do something like "my string {func("key")}" where
    // func is either a builting like get_variable("foo") or get_tool_path("foo") or
    // a user can define their own.

    // TODO: this should be called on demand and not stored since variables can change over time
    pub fn update_command_for_tool(&mut self, workflow_path: &PathBuf) {
        if self.builtin {
            self.cmd = which(&self.name).ok();
        } else if let Some(path) = self.path.as_ref().map(PathBuf::from) {
            // path based so find out the full path
            dbg!(&path);
            let full_path = {
                if path.is_absolute() {
                    path.clone()
                } else {
                    let mut new_path = workflow_path.clone();
                    new_path.push(path);
                    new_path
                }
            };
            self.cmd = which(&full_path).ok();
        } else {
            panic!("Expecting to find a path or builtin tool");
        }
    }

    #[cfg(test)]
    pub fn for_test(name: &str) -> Self {
        Tool {
            name: name.to_string(),
            path: None,
            builtin: true,
            cmd: None,
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
    //TODO: validating commands can use string interpolation
    pub cmd: Option<PathBuf>,
}

impl From<&Tool> for FrozenTool {
    fn from(item: &Tool) -> Self {
        FrozenTool {
            name: item.name.clone(),
            builtin: item.builtin,
            path: item.path.clone(),
            cmd: item.cmd.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                cmd: None,
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
                cmd: None,
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
            name: "foo".to_string(),
            builtin: true,
            path: Some("foo".to_string()),
            cmd: Some(PathBuf::from("foo")),
        };
        let frozen = FrozenTool::from(&tool);
        assert_eq!(frozen.name, tool.name);
        assert_eq!(frozen.builtin, tool.builtin);
        assert_eq!(frozen.path, tool.path);
        assert_eq!(frozen.cmd, tool.cmd);
    }

    // - validation
    #[test]
    fn test_validate_builtin_tool_path() {
        let tools = ["ls", "echo", "cd", "["];
        for tool in tools {
            let mut t = Tool::builtin(tool).unwrap();
            t.update_command_for_tool(&PathBuf::from(""));
            assert_eq!(which(tool).unwrap(), t.cmd.unwrap());
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
        let mut tool = Tool::path_based(
            "foo",
            &absolute_tool_path.into_os_string().into_string().unwrap(),
        )?;

        // Update the tool, the workflow path does not matter here
        tool.update_command_for_tool(&PathBuf::from("some-path"));
        assert_eq!(PathBuf::from(&tmp_file_path), tool.cmd.unwrap());

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
        let mut tool = Tool::path_based("foo", "foo/foo.sh")?;

        // Update
        tool.update_command_for_tool(&workflow_path);

        // Test
        assert_eq!(tool_absolute_path, tool.cmd.unwrap());

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
        let mut tool = Tool::path_based("foo", "foo.sh")?;

        // Update
        tool.update_command_for_tool(&workflow_path);

        // Test
        assert_eq!(tool.cmd, None);

        // Delete the temp dir
        tmp_dir.close()?;
        Ok(())
    }

    //TODO: add test to work with an string interpolated path
}
