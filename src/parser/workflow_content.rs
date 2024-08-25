use crate::parser::var::{parse_var_entry, Var, VarScope};
use crate::parser::{Rule, WorkflowParser};
use pest::iterators::Pair;
use pest::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct WorkflowContent<'a> {
    pub vars: Vec<Var<'a>>,
}

impl<'a> WorkflowContent<'a> {
    fn new() -> WorkflowContent<'a> {
        WorkflowContent { vars: vec![] }
    }
}

fn parse_workflow_content_entry(pairs: Pair<Rule>) -> Result<WorkflowContent, String> {
    match pairs.as_rule() {
        Rule::workflow_content => (),
        _ => panic!("Attempting to parse a non-workflow entry"),
    };

    let mut content = WorkflowContent::new();

    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::var => {
                let var = parse_var_entry(pair)?;
                content.vars.push(var);
            }

            _ => unreachable!(),
        };
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_workflow(filename: &str) -> PathBuf {
        let mut workflow = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        workflow.push("src/test_data");
        workflow.push(filename);
        workflow
    }

    #[test]
    fn workflow_parses_correctly() {
        let workflow = load_test_workflow("vars_only.workflow");
        assert!(workflow.exists());
        let content = fs::read_to_string(workflow).unwrap();

        let pair = WorkflowParser::parse(Rule::workflow_file, &content);
        let result = parse_workflow_content_entry(pair.unwrap().next().unwrap()).unwrap();

        let expected = WorkflowContent {
            vars: vec![
                Var {
                    name: "some_name",
                    default: Some("some default"),
                    env: Some("ENV_VAR"),
                    cli_flag: Some("--some-name"),
                    readers: VarScope::Restricted(vec!["foo", "bar"]),
                    writers: VarScope::Restricted(vec!["foo", "bar"]),
                },
                Var {
                    name: "foo",
                    default: None,
                    env: None,
                    cli_flag: None,
                    readers: VarScope::Global,
                    writers: VarScope::Global,
                },
            ],
        };

        assert_eq!(expected, result);
    }
}
