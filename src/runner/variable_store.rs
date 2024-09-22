use crate::stdlib::{ValueUpdatedBy, VariableEntry};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Default, PartialEq)]
pub struct VariableStore {
    vars: RefCell<HashMap<String, VariableEntry>>,
}

impl VariableStore {
    pub fn new() -> Self {
        VariableStore {
            vars: HashMap::new().into(),
        }
    }

    pub fn register_variable(&self, identifier: &str, var: VariableEntry) {
        self.vars.borrow_mut().insert(identifier.to_string(), var);
    }

    pub fn get_variable_value<'a>(&self, identifier: &str) -> Option<String> {
        let vars = self.vars.borrow();
        vars.get(identifier).map(|v| v.value()).flatten().clone()
    }

    pub fn update_variable_value<'a>(
        &self,
        identifier: &str,
        value: String,
        updated_by: ValueUpdatedBy,
    ) {
        let mut vars = self.vars.borrow_mut();
        if let Some(var) = vars.get_mut(identifier) {
            var.update_value(value, updated_by);
        }
    }

    pub fn with_variable<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&VariableEntry),
    {
        let vars = self.vars.borrow();
        if let Some(var) = vars.get(name) {
            f(var);
        }
    }

    pub fn realize_variables(&self, workflow_args: &Vec<String>) {
        let mut vars = self.vars.borrow_mut();
        for var in vars.values_mut() {
            // First, check to see if there is a command line flag that matches
            if var.try_update_value_from_cli_flag(workflow_args).is_ok() {
                continue;
            }
            // Next,  try to set the value from the env
            if var.try_update_value_from_env().is_ok() {
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::TempEnvVar;

    #[test]
    fn test_register_variable() {
        let store = VariableStore::new();
        let var = VariableEntry::for_test(Some("foo"), None, None);
        store.register_variable("123", var);

        let var = store.get_variable_value("123");
        assert_eq!(var, Some("foo".to_string()));
    }

    #[test]
    fn test_update_variable() {
        let store = VariableStore::new();
        let var = VariableEntry::for_test(None, None, None);
        store.register_variable("123", var);
        store.update_variable_value("123", "new value".into(), ValueUpdatedBy::ForTest);
        let var = store.get_variable_value("123");
        assert_eq!(var, Some("new value".to_string()));
    }

    #[test]
    fn test_relaize_variables() {
        let env = TempEnvVar::new("ENV_VAR_FOR_test_realize_variables_env", "some_value");
        let store = VariableStore::new();
        store.register_variable("1", VariableEntry::for_test(None, Some("--foo"), None));
        store.register_variable(
            "2",
            VariableEntry::for_test(None, None, Some(&env.key.clone())),
        );
        store.register_variable(
            "3",
            VariableEntry::for_test(None, Some("--bar"), Some(&env.key.clone())),
        );

        store.realize_variables(&vec![
            "--foo".to_string(),
            "foo_value".to_string(),
            "--bar".to_string(),
            "bar_value".to_string(),
        ]);

        assert_eq!(store.get_variable_value("1"), Some("foo_value".to_string()));
        assert_eq!(
            store.get_variable_value("2"),
            Some("some_value".to_string())
        );
        assert_eq!(store.get_variable_value("3"), Some("bar_value".to_string()));
    }
}
