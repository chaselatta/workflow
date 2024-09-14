use crate::stdlib::variables::variable::VariableEntry;
use starlark::values::ProvidesStaticType;

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

#[derive(Debug, ProvidesStaticType, Default, PartialEq)]
pub struct VariableStore {}

impl VariableStore {
    pub fn new() -> Self {
        VariableStore {}
    }

    pub fn register_variable(&self, _identifier: &str, _entry: VariableEntry) {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    // Test registering
    // add ability to update all variables from environment here
}
