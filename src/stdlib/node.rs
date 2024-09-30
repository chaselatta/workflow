use crate::stdlib::variable_resolver::VariableResolver;
use crate::stdlib::{Action, ACTION_TYPE, NODE_TYPE};
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

pub(crate) fn node_impl<'v>(name: &str, action: Value<'v>) -> anyhow::Result<Node<'v>> {
    if action.get_type() != ACTION_TYPE {
        bail!("An action must be passed as the ation in a node")
    }

    Ok(Node {
        name: name.to_string(),
        actions: vec![action],
    })
}

pub(crate) fn sequence_impl<'v>(name: &str, actions: Vec<Value<'v>>) -> anyhow::Result<Node<'v>> {
    for action in &actions {
        if action.get_type() != ACTION_TYPE {
            bail!("All actions in a sequence must be action types")
        }
    }

    Ok(Node {
        name: name.to_string(),
        actions: actions,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct NodeGen<V> {
    name: String,
    actions: Vec<V>,
}
starlark_complex_value!(pub Node);

#[starlark_value(type = NODE_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for NodeGen<V> where Self: ProvidesStaticType<'v> {}

impl<'a> Node<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: &PathBuf,
    ) -> anyhow::Result<()> {
        for value in self.actions.clone() {
            let action = Action::from_value(value).unwrap();
            action.run(resolver, working_dir)?;
        }
        Ok(())
    }
}

impl<'v> Freeze for Node<'v> {
    type Frozen = FrozenNode;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(NodeGen {
            name: self.name.freeze(freezer)?,
            actions: self.actions.freeze(freezer)?,
        })
    }
}

impl<V> Display for NodeGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "node")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::assert_env;

    #[test]
    fn test_can_parse_simple_node() {
        assert_env().pass("node(action = action(tool = tool(path='')))");
    }

    #[test]
    fn test_require_an_action_type() {
        assert_env().fail(
            "node(action = 1)",
            "An action must be passed as the ation in a node",
        );
    }

    #[test]
    fn test_set_name() {
        let res = assert_env().pass("node(name = 'foo', action = action(tool = tool(path='')))");
        let node = Node::from_value(res.value()).unwrap();
        assert_eq!(node.name(), "foo");
    }

    #[test]
    fn test_can_parse_simple_sequence() {
        assert_env().pass(
            r#"sequence(
  actions = 
    [
      action(tool = tool(path = '')),
      action(tool = tool(path = '')),
    ]
)"#,
        );
    }

    #[test]
    fn test_sequence_fails_if_any_non_action() {
        assert_env().fail(
            r#"sequence(
  actions = 
    [
      action(tool = tool(path = '')),
      1,
      action(tool = tool(path = '')),
    ]
)"#,
            "All actions in a sequence must be action types",
        );
    }
}
