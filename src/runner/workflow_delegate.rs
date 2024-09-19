use super::VariableStore;
use crate::stdlib::ParseDelegate;
use std::cell::RefCell;
use std::path::PathBuf;

pub struct WorkflowDelegate {
    workflow_file: RefCell<Option<PathBuf>>,
    _variable_store: VariableStore,
}

impl WorkflowDelegate {
    pub fn new() -> Self {
        return WorkflowDelegate {
            workflow_file: None.into(),
            _variable_store: VariableStore::default(),
        };
    }
}

impl ParseDelegate for WorkflowDelegate {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn on_variable(&self, _i: u32) {}

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
