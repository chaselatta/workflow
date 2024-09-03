pub mod parser;
pub mod variable;

#[cfg(test)]
pub mod test_utils {
    pub struct TempEnvVar {
        pub key: String,
        pub original: Option<String>,
    }

    impl TempEnvVar {
        pub fn new(key: &str, val: &str) -> Self {
            let original = std::env::var(&key).ok();
            std::env::set_var(key, val.to_string());
            TempEnvVar {
                key: key.to_string(),
                original: original,
            }
        }
    }

    impl Drop for TempEnvVar {
        fn drop(&mut self) {
            if let Some(original) = &self.original {
                std::env::set_var(&self.key, original.clone());
            } else {
                std::env::remove_var(&self.key);
            }
        }
    }
}
