use crate::parser::type_builder::{Buildable, FieldState};
use crate::parser::{parse_string_entry, Rule, WorkflowParser};
use pest::iterators::Pair;
use pest::Parser;

#[derive(Debug, PartialEq)]
pub enum VarScope<'a> {
    Global,
    Restricted(Vec<&'a str>),
}

/// A type which represents a variable in the workflow
#[derive(Debug, PartialEq)]
pub struct Var<'a> {
    pub name: &'a str,
    pub default: Option<&'a str>,
    pub env: Option< &'a str>,
    pub cli_flag: Option<&'a str>,
    // pub readers: VarScope<'a>,
    // pub writers: VarScope<'a>,
}

impl<'a> Var<'a> {
    fn builder() -> VarBuilder<'a> {
        VarBuilder::new()
    }
}

#[derive(Debug)]
struct VarBuilder<'a> {
    name: FieldState<&'a str>,
    default: FieldState<Option<&'a str>>,
    env: FieldState<Option<&'a str>>,
    cli_flag: FieldState<Option<&'a str>>,
}

impl<'a> Buildable for VarBuilder<'a> {
    type B = Var<'a>;

    fn build(&self) -> Result<Self::B, String> {
        Ok(Var {
            name: self.name.validate("Var::name")?,
            default: *self.default.validate("Var::default")?,
            env: *self.env.validate("Var::env")?,
            cli_flag: *self.cli_flag.validate("Var::cli_flag")?,
        })
    }
}

impl<'a> VarBuilder<'a> {
    fn new() -> VarBuilder<'a> {
        VarBuilder {
            name: FieldState::NeedsValue,
            default: FieldState::Default(None),
            env: FieldState::Default(None),
            cli_flag: FieldState::Default(None),
        }
    }

    fn set_name(&mut self, name: &'a str) {
        if name.is_empty() {
            self.name = FieldState::Error("name cannot be an empty string.".to_string())
        }
        self.name = self.name.update(name);
    }

    fn set_cli_flag(&mut self, val: &'a str) {
        if val.starts_with("--") {
            self.cli_flag = self.cli_flag.update(Some(val));
        } else {
            self.cli_flag = FieldState::Error("Flags must start with --".to_string())
        }
    }

    fn set_default(&mut self, val: &'a str) {
        self.default = self.default.update(Some(val));
    }

    fn set_env(&mut self, val: &'a str) {
        if val.is_empty() {
            self.env = FieldState::Error("env cannot be an empty string.".to_string())
        }
        self.env = self.env.update(Some(val));
    }
}

fn parse_var(var: Pair<Rule>) -> Result<Var, String> {
    match var.as_rule() {
        Rule::var => (),
        _ => panic!("Attempting to parse a non-var")
    };

    let mut builder = Var::builder();
    for pair in var.into_inner() {
        match pair.as_rule() {
            Rule::var_name => {
                builder.set_name(parse_string_entry(pair.into_inner().next().unwrap())?);
            }
            Rule::var_cli_flag => {
                builder.set_cli_flag(parse_string_entry(pair.into_inner().next().unwrap())?);
            }
            Rule::var_default => {
                builder.set_default(parse_string_entry(pair.into_inner().next().unwrap())?);
            }
            Rule::var_env => {
                builder.set_env(parse_string_entry(pair.into_inner().next().unwrap())?);
            }
            _ => unreachable!()
        };
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::{consumes_to, parses_to};

    #[test]
    fn test_set_name() {
        let mut builder = Var::builder();
        builder.set_name("test");
        assert_eq!(builder.name, FieldState::Value("test"),);
    }

    #[test]
    #[should_panic]
    fn test_name_cannot_be_empty() {
        let mut v = Var::builder();
        v.set_name("");
        v.build().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_missing_name_fails() {
        let v = Var::builder();
        v.build().unwrap();
    }

    #[test]
    fn test_invalid_cli_flag() {
        let mut builder = Var::builder();
        builder.set_cli_flag("bar");
        assert!(match builder.cli_flag {
            FieldState::Error(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_set_cli_flag() {
        let mut builder = Var::builder();
        builder.set_cli_flag("--bar");
        assert_eq!(builder.cli_flag, FieldState::Value(Some("--bar")),);
    }

    #[test]
    fn test_set_default() {
        let mut builder = Var::builder();
        builder.set_default("foo");
        assert_eq!(builder.default, FieldState::Value(Some("foo")),);
    }

    #[test]
    fn test_empty_env_fails() {
        let mut builder = Var::builder();
        builder.set_env("");
        assert!(match builder.env {
            FieldState::Error(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_set_env() {
        let mut builder = Var::builder();
        builder.set_env("FOO");
        assert_eq!(builder.env, FieldState::Value(Some("FOO")),);
    }

    #[test]
    #[should_panic]
    fn test_fail_invalid_type_in_parse_var() {
        let pair = WorkflowParser::parse(Rule::string, "").unwrap().next().unwrap();
        parse_var(pair).unwrap();
    }

    #[test]
    fn parse_var_test() {
        parses_to! {
            parser: WorkflowParser,
            input:
r#"var(
  name:"foo"
)"#,
            rule:   Rule::var,
            tokens: [
                var(0, 19,[
                  var_name(7, 17, [
                    string(12, 17, [
                      inner(13, 16)
                    ])
                  ])
                ]),
            ]
        };
    }

    #[test]
    fn parse_var_allow_trailing_comma_test() {
        parses_to! {
            parser: WorkflowParser,
            input:
r#"var(
  name:"foo",
)"#,
            rule:   Rule::var,
            tokens: [
                var(0, 20,[
                  var_name(7, 17, [
                    string(12, 17, [
                      inner(13, 16)
                    ])
                  ])
                ]),
            ]
        };
    }

    #[test]
    fn parse_var_success() {
        let inputs = vec![
            (
                r#"var(
              name: "my_var")"#,
                {
                    let mut builder = Var::builder();
                    builder.set_name("my_var");
                    builder.build().unwrap()
                },
            ),
            (
                r#"var(
                name: "my_var",
                cli_flag: "--foo")"#,
                {
                    let mut builder = Var::builder();
                    builder.set_name("my_var");
                    builder.set_cli_flag("--foo");
                    builder.build().unwrap()
                },
            ),
            (
                r#"var(
                name: "my_var",
                default: "v",
                env: "FOO",
                cli_flag: "--foo")"#,
                {
                    let mut builder = Var::builder();
                    builder.set_name("my_var");
                    builder.set_default("v");
                    builder.set_env("FOO");
                    builder.set_cli_flag("--foo");
                    builder.build().unwrap()
                },
            ),
        ];

        for input in inputs {
            let (string, expected) = input;
            let pair = WorkflowParser::parse(Rule::var, string);
            let result = parse_var(pair.unwrap().next().unwrap());
            println!("{:?}", result);
            assert_eq!(expected, result.unwrap());
        }
    }
}
