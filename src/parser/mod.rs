pub mod type_builder;
pub mod var;

use {pest::iterators::Pair, pest_derive::Parser};

#[derive(Parser)]
#[grammar = "grammars/workflow.pest"]
pub struct WorkflowParser;

fn parse_string_entry(pair: Pair<Rule>) -> Result<&str, String> {
    match pair.as_rule() {
        Rule::string => Ok(pair.into_inner().next().unwrap().as_str()),
        _ => Err("Could not parse entry as string".to_string()),
    }
}

fn parse_string_list_entry(pairs: Pair<Rule>) -> Result<Vec<&str>, String> {
    pairs
        .into_inner()
        .map(|pair| parse_string_entry(pair))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::{consumes_to, parses_to, Parser};

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

    #[test]
    fn parse_empty_string_list_test() {
        parses_to! {
            parser: WorkflowParser,
            input: "[]",
            rule:   Rule::string_list,
            tokens: [
                string_list(0, 2)
            ]
        };
    }

    #[test]
    fn parse_string_list_single_value_no_comma_test() {
        parses_to! {
            parser: WorkflowParser,
            input: r#"[ "abc" ]"#,
            rule:   Rule::string_list,
            tokens: [
                string_list(0, 9, [
                    string(2, 7, [
                        inner(3, 6)
                    ])
                ])
            ]
        };
    }

    #[test]
    fn parse_string_list_single_value_with_comma_test() {
        parses_to! {
            parser: WorkflowParser,
            input: r#"[ "abc", ]"#,
            rule:   Rule::string_list,
            tokens: [
                string_list(0, 10, [
                    string(2, 7, [
                        inner(3, 6)
                    ])
                ])
            ]
        };
    }

    #[test]
    fn parse_string_list_multi_value_test() {
        parses_to! {
            parser: WorkflowParser,
            input: r#"[ "a", "b" ]"#,
            rule:   Rule::string_list,
            tokens: [
                string_list(0, 12, [
                    string(2, 5, [
                        inner(3, 4)
                    ]),
                    string(7, 10, [
                        inner(8, 9)
                    ])
                ])
            ]
        };
    }

    #[test]
    fn parse_builtin_string_list() {
        let inputs = vec![
            (r#"[]"#, vec![]),
            (r#"["a"]"#, vec!["a"]),
            (r#"["a",]"#, vec!["a"]),
            (r#"["a","b"]"#, vec!["a", "b"]),
        ];

        for input in inputs {
            let (string, expected) = input;
            let pair = WorkflowParser::parse(Rule::string_list, string);
            let result = parse_string_list_entry(pair.unwrap().next().unwrap()).unwrap();
            assert_eq!(expected, result);
        }
    }
}
