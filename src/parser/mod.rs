pub mod type_builder;
pub mod var;

use {
    pest::{iterators::Pair, Parser},
    pest_derive::Parser,
};

#[derive(Parser)]
#[grammar = "grammars/workflow.pest"]
pub struct WorkflowParser;

fn parse_string_entry(pair: Pair<Rule>) -> Result<&str, String> {
    match pair.as_rule() {
        Rule::string => Ok(pair.into_inner().next().unwrap().as_str()),
        _ => Err("Could not parse entry as string".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_builtin_strings() {
        let inputs = vec![
            (r#""hello world""#, "hello world"),
            (r#"" hello ""#, " hello "),
            (r#""hello\nworld""#, "hello\\nworld"),
            (
                // Do not change the formatting on this entry
                r#""hello
world""#,
                "hello\nworld",
            ),
            (r#""abc""#, "abc"),
            (r#""123""#, "123"),
            (r#""""#, ""),
            ("\"\"", ""),
        ];

        for input in inputs {
            let (string, expected) = input;
            let pair = WorkflowParser::parse(Rule::string, string);
            let result = parse_string_entry(pair.unwrap().next().unwrap()).unwrap();
            assert_eq!(expected, result);
        }
    }
}
