use crate::stdlib::VariableRef;
use crate::stdlib::{SETTER_TYPE, VARIABLE_REF_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::coerce::Coerce;
use starlark::starlark_complex_value;
use starlark::values::starlark_value;
use starlark::values::Freeze;
use starlark::values::Freezer;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark::StarlarkDocs;
use std::fmt;
use std::fmt::Display;

pub(crate) fn setter_impl<'v>(
    implementation: Value<'v>,
    variable: Value<'v>,
) -> anyhow::Result<Setter<'v>> {
    if variable.get_type() != VARIABLE_REF_TYPE {
        bail!("expected variable type in setter definition")
    }
    if implementation.get_type() != "function" {
        bail!("expected function type in setter definition")
    }
    Ok(Setter {
        implementation: implementation,
        variable: variable,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct SetterGen<V> {
    implementation: V,
    variable: V,
}
starlark_complex_value!(pub Setter);

#[starlark_value(type = SETTER_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for SetterGen<V> where Self: ProvidesStaticType<'v>
{}

impl<'v> Setter<'v> {
    pub fn implementation(&self) -> Value<'v> {
        self.implementation.clone()
    }

    pub fn variable_identifier(&self) -> &str {
        // self.variable.
        self.variable
            .downcast_ref::<VariableRef>()
            .map(|v| v.identifier())
            .unwrap_or("")
    }
}

impl<'v> Freeze for Setter<'v> {
    type Frozen = FrozenSetter;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(SetterGen {
            implementation: self.implementation.freeze(freezer)?,
            variable: self.variable.freeze(freezer)?,
        })
    }
}

impl<V> Display for SetterGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "setter")
    }
}

#[cfg(test)]
mod tests {

    use crate::stdlib::test_utils::assert_env;

    #[test]
    fn test_can_parse_simple_setter() {
        assert_env().pass(
            r#"
def _foo_impl(ctx):
  return "a"

v = variable();

v_setter = setter(
  implementation = _foo_impl,
  variable = v
)
"#,
        );
    }

    #[test]
    fn test_fail_if_not_variable() {
        assert_env().fail(
            r#"
def _foo_impl(ctx):
  return "a"

v_setter = setter(
  implementation = _foo_impl,
  variable = "v"
)
"#,
            "expected variable type in setter definition",
        );
    }

    #[test]
    fn test_fail_if_not_function() {
        assert_env().fail(
            r#"
setter(
  implementation = "_foo_impl",
  variable = variable(),
)
"#,
            "expected function type in setter definition",
        );
    }
}
