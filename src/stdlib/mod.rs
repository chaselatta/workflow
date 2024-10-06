pub mod action;
pub mod errors;
pub mod format;
pub mod legacy;
pub mod next;
pub mod node;
pub mod parse_delegate;
pub mod parser;
pub mod setter;
pub mod tool;
pub mod variable;
pub mod variable_resolver;
pub mod workflow;

pub use self::parse_delegate::{ParseDelegate, ParseDelegateHolder};
pub use crate::stdlib::action::Action;
pub use crate::stdlib::next::{Next, NextStub};
pub use crate::stdlib::node::Node;
use crate::stdlib::setter::Setter;
use crate::stdlib::tool::Tool;
pub use crate::stdlib::variable::{ValueContext, ValueUpdatedBy, VariableEntry, VariableRef};
pub use crate::stdlib::workflow::Workflow;

use action::action_impl;
use format::format_impl;
use format::ValueFormatter;
use next::next_impl;
use node::{node_impl, sequence_impl};
use setter::setter_impl;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::dict::DictOf;
use starlark::values::list::{ListOf, ListRef};
use starlark::values::tuple::UnpackTuple;
use starlark::values::Value;
use tool::{builtin_tool_impl, tool_impl};
use variable::variable_impl;
use workflow::workflow_impl;

pub const ACTION_TYPE: &str = "action";
pub const WORKFLOW_TYPE: &str = "workflow";
pub const NODE_TYPE: &str = "node";
pub const VALUE_FORMATTER_TYPE: &str = "value_formatter";
pub const TOOL_TYPE: &str = "tool";
pub const VARIABLE_REF_TYPE: &str = "variable_ref";
pub const SETTER_TYPE: &str = "setter";
pub const ACTION_CTX_TYPE: &str = "action_ctx";
pub const NEXT_TYPE: &str = "next";
pub const NEXT_STUB_TYPE: &str = "next_stub";

/// A macro to downcast the delegate to an Option<T> without having
/// to deal with lifetimes.
///
/// let delegate: Option<Foo> = downcast_delegate_ref!(holder, Foo);
#[macro_export]
macro_rules! downcast_delegate_ref {
    ($y:ident, $x:tt) => {
        (&*$y.deref()).as_any().downcast_ref::<$x>()
    };
}

pub use downcast_delegate_ref;

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

    /// The tool definition
    fn tool<'v>(#[starlark(require = named)] path: Value<'v>) -> anyhow::Result<Tool<'v>> {
        tool_impl(path)
    }

    /// The builtin_tool definition
    fn builtin_tool<'v>(#[starlark(require = named)] name: &str) -> anyhow::Result<Tool<'v>> {
        builtin_tool_impl(name)
    }

    /// The action definition
    fn action<'v>(
        #[starlark(require = named)] tool: Value<'v>,
        #[starlark(require = named)] args: Option<ListOf<'v, Value<'v>>>,
        #[starlark(require = named)] setters: Option<ListOf<'v, Value<'v>>>,
    ) -> anyhow::Result<Action<'v>> {
        action_impl(
            tool,
            args.map(|v| v.to_vec()).unwrap_or_default(),
            setters.map(|v| v.to_vec()).unwrap_or_default(),
        )
    }

    /// The workflow definition
    fn workflow<'v>(
        #[starlark(require = named)] entrypoint: Option<&str>,
        #[starlark(require = named)] graph: Value<'v>,
    ) -> anyhow::Result<Workflow<'v>> {
        workflow_impl(entrypoint.unwrap_or_default(), {
            if let Some(list_ref) = ListRef::from_value(graph) {
                list_ref.to_vec()
            } else {
                vec![graph]
            }
        })
    }

    /// The node definition
    fn node<'v>(
        #[starlark(require = named)] name: Option<&str>,
        #[starlark(require = named)] action: Value<'v>,
        #[starlark(require = named)] next: Option<Value<'v>>,
    ) -> anyhow::Result<Node<'v>> {
        node_impl(name.unwrap_or_default(), action, next)
    }

    /// The sequence definition
    fn sequence<'v>(
        #[starlark(require = named)] name: Option<&str>,
        #[starlark(require = named)] actions: ListOf<'v, Value<'v>>,
        #[starlark(require = named)] next: Option<Value<'v>>,
    ) -> anyhow::Result<Node<'v>> {
        sequence_impl(name.unwrap_or_default(), actions.to_vec(), next)
    }

    /// The setter definition
    fn setter<'v>(
        #[starlark(require = named)] implementation: Value<'v>,
        #[starlark(require = named)] variable: Value<'v>,
    ) -> anyhow::Result<Setter<'v>> {
        setter_impl(implementation, variable)
    }

    /// The next definition
    fn next<'v>(
        #[starlark(require = named)] implementation: Value<'v>,
        #[starlark(require = named)] args: Option<DictOf<'v, String, Value<'v>>>,
    ) -> anyhow::Result<NextStub<'v>> {
        next_impl(
            implementation, 
            args.map(|v| v.to_dict()).unwrap_or_default(),
        )
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use starlark::assert::Assert;
    use std::any::Any;
    use std::cell::RefCell;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    #[derive(Debug, Default)]
    pub struct TestParseDelegate {
        pub on_variable_call_count: RefCell<u32>,
        pub workflow_file: RefCell<PathBuf>,
        pub completed: RefCell<bool>,
    }

    impl ParseDelegate for TestParseDelegate {
        fn on_variable(&self, _id: &str, _v: VariableEntry) {
            let v = *self.on_variable_call_count.borrow() + 1;
            self.on_variable_call_count.replace(v);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn will_parse_workflow(&self, workflow: PathBuf) {
            self.workflow_file.replace(workflow);
        }

        fn did_parse_workflow(&self) {
            self.completed.replace(true);
        }
    }

    pub struct TempWorkflowFile {
        _file: File,
        dir: tempfile::TempDir,
        file_path: PathBuf,
    }

    impl TempWorkflowFile {
        pub fn new(name: &str, content: &str) -> anyhow::Result<Self> {
            TempWorkflowFile::new_impl(name, content, false)
        }

        pub fn new_executable(name: &str, content: &str) -> anyhow::Result<Self> {
            TempWorkflowFile::new_impl(name, content, true)
        }

        fn new_impl(name: &str, content: &str, executable: bool) -> anyhow::Result<Self> {
            let dir = tempdir()?;

            let file_path = dir.path().join(name);
            let mut file = File::create(file_path.clone())?;
            writeln!(file, "{}", content)?;

            if executable {
                let mut perms = file.metadata()?.permissions();
                perms.set_mode(0o755);
                file.set_permissions(perms)?;
            }

            Ok(TempWorkflowFile {
                _file: file,
                file_path: file_path,
                dir: dir,
            })
        }

        pub fn path(&self) -> PathBuf {
            // On macos the tempfile returns the /var path which is a
            // symlink to /private/var so we need to canonicalize it.
            fs::canonicalize(self.file_path.clone()).unwrap()
        }

        pub fn dir(&self) -> PathBuf {
            // On macos the tempfile returns the /var path which is a
            // symlink to /private/var so we need to canonicalize it.
            fs::canonicalize(PathBuf::from(self.dir.path())).unwrap()
        }
    }
}
