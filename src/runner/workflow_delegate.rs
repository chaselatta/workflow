use super::VariableStore;
use crate::stdlib::ParseDelegate;
use crate::stdlib::VariableEntry;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Debug)]
pub struct WorkflowDelegate {
    workflow_file: RefCell<Option<PathBuf>>,
    variable_store: VariableStore,
}

impl WorkflowDelegate {
    pub fn new() -> Self {
        return WorkflowDelegate {
            workflow_file: None.into(),
            variable_store: VariableStore::new(),
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
