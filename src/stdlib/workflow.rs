use crate::stdlib::{NODE_TYPE, WORKFLOW_TYPE};
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

pub(crate) fn workflow_impl<'v>(
    entrypoint: &str,
    graph: Vec<Value<'v>>,
) -> anyhow::Result<Workflow<'v>> {
    for node in &graph {
        if node.get_type() != NODE_TYPE {
            bail!("graph can only contain node values")
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
    graph: Vec<V>,
}
starlark_complex_value!(pub Workflow);

#[starlark_value(type = WORKFLOW_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for WorkflowGen<V> where
    Self: ProvidesStaticType<'v>
{
}

impl<'a> Workflow<'a> {}

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
        assert_eq!(&workflow.graph, &vec![]);
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
}
