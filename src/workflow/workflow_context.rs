use std::path::PathBuf;

use super::VariableStore;

pub struct WorkflowContext {
    workflow_file: PathBuf,
    variable_store: VariableStore,
}

impl WorkflowContext {
    pub fn new(workflow_file: PathBuf) -> Self {
        return WorkflowContext {
            workflow_file: workflow_file,
            variable_store: VariableStore::default(),
        };
    }
}
