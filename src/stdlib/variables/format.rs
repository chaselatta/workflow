use crate::stdlib::variables::LateBoundString;
use crate::stdlib::variables::VariableResolver;
use allocative::Allocative;
use starlark::environment::GlobalsBuilder;
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::tuple::UnpackTuple;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Value;
use std::fmt;

#[starlark_module]
pub fn starlark_format(builder: &mut GlobalsBuilder) {
    fn format(
        #[starlark(require = pos)] fmt_str: &str,
        #[starlark(args)] args: UnpackTuple<Value>,
    ) -> anyhow::Result<ValueFormatter> {
        let mut values: Vec<LateBoundString> = vec![];
        for a in args {
            if let Some(formatter) = ValueFormatter::from_value(a) {
                values.push(LateBoundString::with_value_formatter(formatter.clone()));
            } else {
                values.push(LateBoundString::with_value(a.to_str()));
            }
        }
        Ok(ValueFormatter {
            fmt_str: fmt_str.to_string(),
            values: values,
        })
    }
}

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct ValueFormatter {
    fmt_str: String,
    values: Vec<LateBoundString>,
}
starlark_simple_value!(ValueFormatter);

#[starlark_value(type = "value_formatter")]
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
    use starlark::assert::Assert;

    struct TestResolver {}
    const NO_RESOLVE: TestResolver = TestResolver {};
    impl VariableResolver for TestResolver {
        fn resolve(&self, _identifier: &str) -> anyhow::Result<String> {
            Ok("".to_string())
        }
    }

    fn assert_env<'a>() -> Assert<'a> {
        let mut env = Assert::new();
        env.globals_add(starlark_format);
        env
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
}
