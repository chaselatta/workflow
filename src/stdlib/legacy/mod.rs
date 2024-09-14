pub mod tool;
pub mod variable;

use crate::stdlib::errors::StdlibError;
use anyhow::bail;

fn validate_name(name: &str) -> anyhow::Result<String> {
    if name.is_empty() {
        bail!(StdlibError::new_invalid_attr(
            "name",
            "cannot be empty",
            name
        ));
    }
    if name.contains(" ") {
        bail!(StdlibError::new_invalid_attr(
            "name",
            "cannot contain spaces",
            name
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn validate_name_success() {
        assert_eq!(validate_name("foo").unwrap(), "foo".to_string());
        assert_eq!(validate_name("1").unwrap(), "1".to_string());
    }

    #[test]
    #[should_panic]
    fn validate_name_fail_empty() {
        validate_name("").unwrap();
    }

    #[test]
    #[should_panic]
    fn validate_name_fail_spaces() {
        validate_name("a b").unwrap();
    }
}
