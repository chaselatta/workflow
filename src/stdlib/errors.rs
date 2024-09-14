use thiserror::Error;

#[derive(Error, Debug)]
pub enum StdlibError {
    #[error("Invalid attribute '{attr}', {reason} got {value:?}")]
    InvalidAttribute {
        attr: String,
        value: String,
        reason: String,
    },
}

impl StdlibError {
    pub fn new_invalid_attr<T: Into<String>>(attr: &str, reason: &str, value: T) -> Self {
        StdlibError::InvalidAttribute {
            attr: attr.to_string(),
            reason: reason.to_string(),
            value: value.into(),
        }
    }
}
