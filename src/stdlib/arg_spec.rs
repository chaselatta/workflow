use crate::stdlib::{INT_ARG_TYPE, STRING_ARG_TYPE, STRUCT_VALUE_TYPE};
use allocative::Allocative;
use anyhow::bail;
use starlark::environment::GlobalsBuilder;
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::AllocValue;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Value;
use std::fmt;

pub fn arg_spec(globals: &mut GlobalsBuilder) {
    #[starlark_module]
    fn arg_spec_members(globals: &mut GlobalsBuilder) {
        fn string(
            #[starlark(require = named)] default: Option<String>,
            #[starlark(require = named)] required: Option<bool>,
        ) -> anyhow::Result<StringArg> {
            Ok(StringArg {
                required: required.unwrap_or(false),
                default: default.unwrap_or("".to_string()),
            })
        }

        fn int(
            #[starlark(require = named)] default: Option<i32>,
            #[starlark(require = named)] required: Option<bool>,
        ) -> anyhow::Result<IntArg> {
            Ok(IntArg {
                required: required.unwrap_or(false),
                default: default.unwrap_or(0),
            })
        }
    }
    globals.struct_("args", arg_spec_members);
}

pub trait FinalizeArg {
    fn finalize(&self, value: Option<&Value<'_>>) -> StructValue;
}

//
// -- StringArg
//
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct StringArg {
    required: bool,
    default: String,
}
starlark_simple_value!(StringArg);

#[starlark_value(type = STRING_ARG_TYPE )]
impl<'v> StarlarkValue<'v> for StringArg {}

impl fmt::Display for StringArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "string_arg: required {}, default: {}",
            self.required, &self.default
        )
    }
}

impl StringArg {
    pub fn struct_value(&self, value: Option<&Value<'_>>) -> anyhow::Result<StructValue> {
        Ok(StructValue::String(match value {
            Some(v) => {
                if v.get_type() != "string" {
                    bail!("Should be a string type")
                }
                v.to_str()
            }
            None => self.default.clone(),
        }))
    }
}

//
// -- IntArg
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct IntArg {
    required: bool,
    default: i32,
}
starlark_simple_value!(IntArg);

#[starlark_value(type = INT_ARG_TYPE )]
impl<'v> StarlarkValue<'v> for IntArg {}

impl fmt::Display for IntArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "string_arg: required {}, default: {}",
            self.required, &self.default
        )
    }
}

impl IntArg {
    pub fn struct_value(&self, value: Option<&Value<'_>>) -> anyhow::Result<StructValue> {
        Ok(StructValue::Int(match value {
            Some(v) => {
                if v.get_type() != "int" {
                    bail!("Should be an int type")
                }
                //TODO: Should fail if unpack fails
                v.unpack_i32().unwrap_or(0)
            }
            None => self.default.clone(),
        }))
    }
}

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub enum StructValue {
    String(String),
    Int(i32),
}

impl<'v> AllocValue<'v> for StructValue {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> Value<'v> {
        match self {
            StructValue::String(v) => heap.alloc(v),
            StructValue::Int(v) => heap.alloc(v),
        }
    }
}

#[starlark_value(type = STRUCT_VALUE_TYPE)]
impl<'v> StarlarkValue<'v> for StructValue {}

impl fmt::Display for StructValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "struct_value")
    }
}
