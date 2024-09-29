use crate::stdlib::variable_resolver::VariableResolver;
use crate::stdlib::Node;
use crate::stdlib::{NODE_TYPE, WORKFLOW_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::coerce::Coerce;
use starlark::collections::SmallMap;
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

pub(crate) fn workflow_impl<'v>(
    entrypoint: &str,
    nodes: Vec<Value<'v>>,
) -> anyhow::Result<Workflow<'v>> {
    let mut graph: SmallMap<String, Value<'_>> = SmallMap::new();
    for node in &nodes {
        if node.get_type() != NODE_TYPE {
            bail!("graph can only contain node values")
        }
        let name = Node::from_value(*node)
            .expect("Should be a node")
            .name()
            .to_string();
        if let Some(_) = graph.insert(name, *node) {
            bail!("nodes must have unique names")
        }
    }

    Ok(Workflow {
        entrypoint: entrypoint.to_string(),
        graph: graph,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct WorkflowGen<V> {
    entrypoint: String,
    graph: SmallMap<String, V>,
}
starlark_complex_value!(pub Workflow);

#[starlark_value(type = WORKFLOW_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for WorkflowGen<V> where
    Self: ProvidesStaticType<'v>
{
}

impl<'a> Workflow<'a> {
    pub fn first_node(&self) -> anyhow::Result<&Node<'a>> {
        match self.graph.len() {
            0 => bail!("Graph contains no nodes"),
            1 => self.first_node_from_single_node_graph(),
            _ => self.node_with_name(&self.entrypoint),
        }
    }

    fn first_node_from_single_node_graph(&self) -> anyhow::Result<&Node<'a>> {
        let value = self.graph.first().unwrap().1;
        Ok(Node::from_value(*value).unwrap())
    }

    fn node_with_name(&self, name: &str) -> anyhow::Result<&Node<'a>> {
        if let Some(value) = self.graph.get(name) {
            Ok(Node::from_value(*value).unwrap())
        } else {
            bail!("No node with name: '{}'", name)
        }
    }

    pub fn run<T: VariableResolver>(
        &self,
        resolver: &T,
        working_dir: &PathBuf,
    ) -> anyhow::Result<()> {
        let node = self.first_node()?;
        node.run(resolver, working_dir)?;
        Ok(())
    }
}

impl<'v> Freeze for Workflow<'v> {
    type Frozen = FrozenWorkflow;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(WorkflowGen {
            entrypoint: self.entrypoint.freeze(freezer)?,
            graph: self.graph.freeze(freezer)?,
        })
    }
}

impl<V> Display for WorkflowGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "workflow")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::assert_env;

    #[test]
    fn test_required_values() {
        assert_env().pass("workflow(graph=[])");
    }

    #[test]
    fn test_parse_sets_values() {
        let module: starlark::environment::FrozenModule =
            assert_env().pass_module("w = workflow(entrypoint = 'e', graph=[])");
        let workflow = module.get("w").unwrap();
        let workflow = Workflow::from_value(workflow.value()).unwrap();
        assert_eq!(workflow.entrypoint, "e".to_string());
        assert_eq!(&workflow.graph, &SmallMap::new());
    }

    #[test]
    fn test_parse_graph_many_values() {
        assert_env().pass(
            r#"
workflow(
    entrypoint = "a",
    graph = [
        node(name = "a", action = action(tool = tool(path = ""))),
        node(name = "b", action = action(tool = tool(path = ""))),
        sequence(name = "c", actions = []),
    ]
)"#,
        );
    }

    #[test]
    fn test_graph_must_contain_unique_names() {
        assert_env().fail(
            r#"
workflow(
    entrypoint = "a",
    graph = [
        node(name = "a", action = action(tool = tool(path = ""))),
        sequence(name = "a", actions = []),
    ]
)"#,
            "nodes must have unique names",
        );
    }

    #[test]
    fn test_graph_must_contain_nodes_only() {
        assert_env().fail(
            r#"
workflow(
    entrypoint = "a",
    graph = [
        node(name = "a", action = action(tool = tool(path = ""))),
        1,
        sequence(name = "c", actions = []),
    ]
)"#,
            "graph can only contain node values",
        );
    }

    #[test]
    fn test_graph_can_take_single_node() {
        assert_env().pass(
            r#"
workflow(
    entrypoint = "a",
    graph = node(name = "a", action = action(tool = tool(path = ""))),
)"#,
        );
    }

    #[test]
    fn test_entry_point_single_node() {
        let res = assert_env().pass(
            r#"
workflow(
    graph = node(name = "a", action = action(tool = tool(path = ""))),
)"#,
        );
        let workflow = Workflow::from_value(res.value()).unwrap();
        let first_node = workflow.first_node().unwrap();
        assert_eq!(first_node.name(), "a");
    }

    #[test]
    fn test_entry_point_multi_node() {
        let res = assert_env().pass(
            r#"
workflow(
    entrypoint = "b",
    graph = [
      node(name = "a", action = action(tool = tool(path = ""))),
      node(name = "b", action = action(tool = tool(path = ""))),
    ],
)"#,
        );
        let workflow = Workflow::from_value(res.value()).unwrap();
        let first_node = workflow.first_node().unwrap();
        assert_eq!(first_node.name(), "b");
    }
}
