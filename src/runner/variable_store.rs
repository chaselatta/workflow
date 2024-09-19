use crate::stdlib::{ValueUpdatedBy, VariableEntry};
use std::cell::RefCell;
use std::collections::HashMap;

// variable() -> VariableRef which holds the identifier
// variable() emits values to the parse context
// parse_context holds a VariableStore which maps ids -> variables
// VariableEntry is stored in VariableStore and holds the variable info
// which can be printed by describe or queried by the id

// For checking readers/writers of variables do something like the following.
// when running an action we push a value on to the valuestore which references
// the current running action. Then this value is used when we try to read any
// variable to see if the action that is running can access the variable.
// for things like tools and such we can also lock the variable store or we can
// just say that variables are globally available to tools.

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_iter_variables() {
        let store = VariableStore::new();
        store.register_variable("1", VariableEntry::for_test(None, None, None));
        store.register_variable("2", VariableEntry::for_test(None, None, None));

        let mut iter_count = 0;
        store.iter_variables(|_| iter_count += 1);
        assert_eq!(iter_count, 2);
    }
}
