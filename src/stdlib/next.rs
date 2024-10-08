use crate::stdlib::arg_spec::StructValue;
use crate::stdlib::{NEXT_STUB_TYPE, NEXT_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::coerce::Coerce;
use starlark::collections::SmallMap;
use starlark::eval::Arguments;
use starlark::eval::Evaluator;
use starlark::starlark_complex_value;
use starlark::values::starlark_value;
use starlark::values::structs::AllocStruct;
use starlark::values::AllocValue;
use starlark::values::Freeze;
use starlark::values::Freezer;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark::StarlarkDocs;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;

use super::arg_spec::StringArg;

pub(crate) fn next_impl<'v>(
    implementation: Value<'v>,
    arg_spec: SmallMap<String, Value<'v>>,
) -> anyhow::Result<NextStub<'v>> {
    if implementation.get_type() != "function" {
        // TODO: look at using ValueError
        bail!("expected function type in next definition")
    }
    Ok(NextStub {
        implementation: implementation,
        arg_spec: arg_spec,
    })
}

#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct NextStubGen<V> {
    implementation: V,
    arg_spec: SmallMap<String, V>,
}
starlark_complex_value!(pub NextStub);

#[starlark_value(type = NEXT_STUB_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for NextStubGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn invoke(
        &self,
        me: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut Evaluator<'v, '_>,
    ) -> starlark::Result<Value<'v>> {
        let me = NextStub::from_value(me).unwrap();

        // get arg_spec and match it up against what args should be
        let mut ctx_args: HashMap<&str, StructValue> = HashMap::new();
        let args_map = args.names_map()?;
        let heap = eval.heap();

        for (spec_name, spec_value) in &me.arg_spec {
            let key = heap.alloc_str(spec_name);
            let arg_value = args_map.get(&key);
            let value = {
                if let Some(spec) = StringArg::from_value(spec_value.clone()) {
                    spec.struct_value(arg_value).expect("TODO")
                } else {
                    panic!("FIX ME");
                }
            };
            ctx_args.insert(spec_name, value);
        }

        // TOOD: Fix this, we need to check that we are not passing along
        // too many args.
        if args.names()?.len() != ctx_args.len() {
            panic!("too many args");
        }

        let next_args = eval.heap().alloc(AllocStruct(ctx_args));

        let next = Next {
            implementation: me.implementation.clone(),
            args: next_args,
        };

        Ok(next.alloc_value(eval.heap()))
    }
}

impl<'v> NextStub<'v> {
    // pub fn implementation(&self) -> Value<'v> {
    //     self.implementation.clone()
    // }
}

impl<'v> Freeze for NextStub<'v> {
    type Frozen = FrozenNextStub;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(NextStubGen {
            implementation: self.implementation.freeze(freezer)?,
            arg_spec: self.arg_spec.freeze(freezer)?,
        })
    }
}

impl<V> Display for NextStubGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "next_stub")
    }
}

//
// - Next Impl
//
#[derive(
    Coerce, Clone, Default, Trace, Debug, ProvidesStaticType, StarlarkDocs, NoSerialize, Allocative,
)]
#[repr(C)]
pub struct NextGen<V> {
    implementation: V,
    args: V,
}
starlark_complex_value!(pub Next);

#[starlark_value(type = NEXT_TYPE)]
impl<'v, V: ValueLike<'v> + 'v> StarlarkValue<'v> for NextGen<V> where Self: ProvidesStaticType<'v> {}

impl<'v> Next<'v> {
    pub fn implementation(&self) -> Value<'v> {
        self.implementation.clone()
    }

    pub fn args(&self) -> Value<'v> {
        self.args.clone()
    }
}

impl<'v> Freeze for Next<'v> {
    type Frozen = FrozenNext;
    fn freeze(self, freezer: &Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(NextGen {
            implementation: self.implementation.freeze(freezer)?,
            args: self.args.freeze(freezer)?,
            // arg_spec: self.arg_spec.freeze(freezer)?,
        })
    }
}

impl<V> Display for NextGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "next")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::test_utils::assert_env;

    #[test]
    fn test_next_returns_stub_type() {
        let res = assert_env().pass(
            r#"
def _foo_impl(ctx):
  return "a"

next(
  implementation = _foo_impl,
)
"#,
        );
        assert_eq!(res.value().get_type(), NEXT_STUB_TYPE);
    }

    #[test]
    fn test_fail_if_not_function() {
        assert_env().fail(
            r#"
next(
  implementation = "_foo_impl",
)
"#,
            "expected function type in next definition",
        );
    }

    #[test]
    fn test_invoke_next_stub() {
        let res = assert_env().pass(
            r#"
def _foo_impl(ctx):
  return "a"

foo = next(
  implementation = _foo_impl,
)

foo()
"#,
        );
        assert_eq!(res.value().get_type(), NEXT_TYPE);
    }
}
