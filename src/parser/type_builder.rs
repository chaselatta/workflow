#[derive(Debug, PartialEq)]
pub enum FieldState<T> {
    Default(T),    // A Default is set and can be updated
    NeedsValue,    // No default is set and it needs a value
    Value(T),      // A value can be set
    Error(String), // An error has occurred.
}

impl<T> FieldState<T>
where
    T: std::fmt::Debug,
{
    pub fn update(&self, val: T) -> Self {
        match self {
            FieldState::NeedsValue | FieldState::Default(_) => FieldState::Value(val),
            FieldState::Value(v) => FieldState::Error(format!(
                "Cannot update value to {:?}, value already set to {:?}",
                val, v
            )),
            FieldState::Error(e) => FieldState::Error(e.to_owned()),
        }
    }

    pub fn validate(&self, ctx: &str) -> Result<&T, String> {
        match self {
            FieldState::NeedsValue => {
                let ctx_string = ctx.to_owned();
                Err(format!("{ctx_string}: No Value Set"))
            }
            FieldState::Error(e) => Err(e.to_owned()),
            FieldState::Default(v) => Ok(v),
            FieldState::Value(v) => Ok(v),
        }
    }
}

pub trait Buildable {
    type B;
    fn build(&self) -> Result<Self::B, String>;
}
