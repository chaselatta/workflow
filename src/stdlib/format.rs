use crate::stdlib::variable::VariableRef;
use crate::stdlib::variable_resolver::LateBoundString;
use crate::stdlib::variable_resolver::VariableResolver;
use crate::stdlib::VALUE_FORMATTER_TYPE;
use allocative::Allocative;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::tuple::UnpackTuple;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Value;
use std::fmt;

pub(crate) fn format_impl(
    fmt_str: &str,
    args: UnpackTuple<Value>,
) -> anyhow::Result<ValueFormatter> {
    let mut values: Vec<LateBoundString> = vec![];
    for a in args {
        if let Some(formatter) = ValueFormatter::from_value(a) {
            values.push(LateBoundString::with_value_formatter(formatter.clone()));
        } else if let Some(variable) = VariableRef::from_value(a) {
            values.push(LateBoundString::with_identifier(
                variable.identifier().to_string(),
            ));
        } else {
            values.push(LateBoundString::with_value(a.to_str()));
        }
    }
    Ok(ValueFormatter {
        fmt_str: fmt_str.to_string(),
        values: values,
    })
}

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct ValueFormatter {
    fmt_str: String,
    values: Vec<LateBoundString>,
}
starlark_simple_value!(ValueFormatter);

#[starlark_value(type = VALUE_FORMATTER_TYPE)]
impl<'v> StarlarkValue<'v> for ValueFormatter {}

// fmt should take a trait like VariableResolver which takes an ID and returns the current value
// then LateBoundString can just take an ID or a value, if ID hten we resolve it later but if
// we have a value we just return that value.
impl ValueFormatter {
    pub fn new(fmt_str: &str, values: Vec<LateBoundString>) -> Self {
        ValueFormatter {
            fmt_str: fmt_str.to_string(),
            values: values,
        }
    }

    pub fn fmt<T: VariableResolver>(&self, resolver: &T) -> anyhow::Result<String> {
        // TODO: Look into using th normal write! macros here.
        // The problem is that we have a Vec<String> and we would need to expand
        // that into named parameters of sorts.
        let mut fmt = self.fmt_str.clone();
        for v in &self.values {
            fmt = {
                let t = fmt.replacen("{}", &v.get_value(resolver)?, 1);
                if t == fmt {
                    panic!("more args than placeholders");
                }
                t
            };
        }
        Ok(fmt)
    }
}

impl fmt::Display for ValueFormatter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.fmt_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::assert_env;
    use std::collections::HashMap;

    struct TestResolver {}
    const NO_RESOLVE: TestResolver = TestResolver {};
    impl VariableResolver for TestResolver {
        fn resolve(&self, _identifier: &str) -> anyhow::Result<String> {
            Ok("".to_string())
        }
    }

    #[test]
    fn test_can_parse_empty_string() {
        assert_env().pass("format('')");
    }

    #[test]
    fn test_can_parse_no_args_string() {
        assert_env().pass("format('hello, world')");
    }

    #[test]
    fn test_can_parse_n_args_string() {
        assert_env().pass("format('a', 1); format('a', 1, 'x', None)");
    }

    #[test]
    fn test_simple_format() {
        let mut env = assert_env();
        let module = env.module("format.star", "a = format('a')");
        let a = module.get("a").unwrap();
        let formatter = ValueFormatter::from_value(a.value()).unwrap();
        assert_eq!(formatter.fmt(&NO_RESOLVE).unwrap(), "a");
    }

    #[test]
    fn test_complex_format() {
        let mut env = assert_env();
        let module = env.module("format.star", "a = format('{}, {}, {}', 'z', 1, None)");
        let a = module.get("a").unwrap();
        let formatter = ValueFormatter::from_value(a.value()).unwrap();
        assert_eq!(formatter.fmt(&NO_RESOLVE).unwrap(), "z, 1, None");
    }

    #[test]
    fn test_complex_recurse() {
        let mut env = assert_env();
        let module = env.module("format.star", "a = format('{}', 'a'); b = format('{}', a)");
        let a = module.get("b").unwrap();
        let formatter = ValueFormatter::from_value(a.value()).unwrap();
        assert_eq!(formatter.fmt(&NO_RESOLVE).unwrap(), "a");
    }

    #[test]
    fn test_format_with_variable() {
        let mut resolver: HashMap<&str, &str> = HashMap::new();

        let mut env = assert_env();
        let module = env.module(
            "format.star",
            "v = variable(default = 'default'); a = format('{}', v)",
        );
        let v = module.get("v").unwrap();
        let v = v.value();
        let var_ref = VariableRef::from_value(v).unwrap();
        resolver.insert(var_ref.identifier(), "default");

        let a = module.get("a").unwrap();
        let formatter = ValueFormatter::from_value(a.value()).unwrap();
        assert_eq!(formatter.fmt(&resolver).unwrap(), "default");
    }
}
