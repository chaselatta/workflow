pub mod format;
use crate::stdlib::variables::format::ValueFormatter;
use allocative::Allocative;
use anyhow::bail;
use starlark::values::ProvidesStaticType;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VariableResolverError {
    #[error("Unknown variable with id '{0}'")]
    UnknownVariable(String),
    #[error("Variable with id '{0}' has no value")]
    NoValueSet(String),
}

/// A trait which is used to resolve a variable's value based on
/// an identifier.
pub trait VariableResolver {
    /// Return the value for the identifier. If the value is not
    /// known return VariableResolverError::UnknownVariable and if there
    /// is no value set for the variable return VariableResolverError::NoValueSet
    fn resolve(&self, identifier: &str) -> anyhow::Result<String>;
}

impl VariableResolver for HashMap<&str, &str> {
    fn resolve(&self, identifier: &str) -> anyhow::Result<String> {
        if let Some(val) = self.get(identifier) {
            Ok(val.to_string())
        } else {
            bail!(VariableResolverError::UnknownVariable(
                identifier.to_string()
            ))
        }
    }
}

#[derive(Debug, ProvidesStaticType, Allocative, Clone)]
enum OneOf {
    Value(String),
    Identifier(String),
    ValueFormatter(ValueFormatter),
}

/// A string that can be used to format a string on demand.
#[derive(Debug, ProvidesStaticType, Allocative, Clone)]
pub struct LateBoundString(OneOf);

impl LateBoundString {
    pub fn with_value(string: String) -> Self {
        LateBoundString(OneOf::Value(string))
    }

    pub fn with_identifier(identifier: String) -> Self {
        LateBoundString(OneOf::Identifier(identifier))
    }

    pub fn with_value_formatter(formatter: ValueFormatter) -> Self {
        LateBoundString(OneOf::ValueFormatter(formatter))
    }

    pub fn get_value<V: VariableResolver>(&self, resolver: &V) -> anyhow::Result<String> {
        match &self.0 {
            OneOf::Value(s) => Ok(s.clone()),
            OneOf::Identifier(id) => resolver.resolve(&id),
            OneOf::ValueFormatter(vf) => vf.fmt(resolver),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_from_value() {
        let r: HashMap<&str, &str> = HashMap::new();
        let v = LateBoundString::with_value("foo".to_string());
        assert_eq!(v.get_value(&r).unwrap(), "foo".to_string());
    }

    #[test]
    fn test_resolve_from_identifier() {
        let mut r: HashMap<&str, &str> = HashMap::new();
        r.insert("123", "foo");
        let v = LateBoundString::with_identifier("123".to_string());
        assert_eq!(v.get_value(&r).unwrap(), "foo".to_string());
    }

    #[test]
    fn test_resolve_from_value_formatter() {
        let r: HashMap<&str, &str> = HashMap::new();
        let formatter =
            ValueFormatter::new("-{}-", vec![LateBoundString::with_value("foo".to_string())]);
        let v = LateBoundString::with_value_formatter(formatter);
        assert_eq!(v.get_value(&r).unwrap(), "-foo-".to_string());
    }

    #[test]
    #[should_panic(expected = "Unknown variable with id '123'")]
    fn test_resolve_from_identifier_fail() {
        let r: HashMap<&str, &str> = HashMap::new();
        let v = LateBoundString::with_identifier("123".to_string());
        v.get_value(&r).unwrap();
    }
}
