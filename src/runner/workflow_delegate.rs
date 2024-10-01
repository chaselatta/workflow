use super::VariableStore;
use crate::stdlib::variable_resolver::VariableResolver;
use crate::stdlib::variable_resolver::VariableUpdater;
use crate::stdlib::ParseDelegate;
use crate::stdlib::ValueUpdatedBy;
use crate::stdlib::VariableEntry;
use anyhow::bail;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Debug)]
pub struct WorkflowDelegate {
    workflow_file: RefCell<Option<PathBuf>>,
    variable_store: VariableStore,
    workflow_args: Vec<String>,
}

impl WorkflowDelegate {
    pub fn new() -> Self {
        WorkflowDelegate::with_args(vec![])
    }

    pub fn with_args(args: Vec<String>) -> Self {
        return WorkflowDelegate {
            workflow_file: None.into(),
            variable_store: VariableStore::new(),
            workflow_args: args,
        };
    }

    pub fn variable_store(&self) -> &VariableStore {
        &self.variable_store
    }
}

impl ParseDelegate for WorkflowDelegate {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn on_variable(&self, identifier: &str, variable: VariableEntry) {
        self.variable_store.register_variable(identifier, variable);
    }

    fn will_parse_workflow(&self, workflow: PathBuf) {
        self.workflow_file.replace(Some(workflow));
    }

    fn did_parse_workflow(&self) {
        self.variable_store.realize_variables(&self.workflow_args);
    }
}

impl VariableResolver for WorkflowDelegate {
    fn resolve(&self, identifier: &str) -> anyhow::Result<String> {
        match self.variable_store.get_variable_value(identifier) {
            Some(v) => Ok(v),
            None => bail!("No value for variable"),
        }
    }
}

impl VariableUpdater for WorkflowDelegate {
    fn update(&self, identifier: &str, value: String) -> anyhow::Result<()> {
        self.variable_store.update_variable_value(
            identifier,
            value,
            ValueUpdatedBy::Action("".to_string()),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_will_parse_workflow() {
        let delegate = WorkflowDelegate::new();
        delegate.will_parse_workflow(PathBuf::from("foo"));
        assert_eq!(delegate.workflow_file, Some(PathBuf::from("foo")).into());
    }
}
