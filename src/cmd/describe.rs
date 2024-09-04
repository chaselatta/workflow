use crate::cmd::{GlobalArgs, RunCommand};
use crate::stdlib::parser::Parser;
use crate::stdlib::tool::FrozenTool;
use crate::stdlib::variable::FrozenVariable;
use anyhow::bail;
use clap::Args;
use std::cmp;
use std::path::PathBuf;

use ansi_term::Colour::{Cyan, Green, Red};
use starlark::environment::Module;
use starlark::eval::Evaluator;

#[derive(Args, Debug)]
pub struct DescribeArgs {
    /// The path to the workflow to describe
    pub workflow: PathBuf,

    /// The additional arguments that will be passed along to the workflow
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub workflow_args: Vec<String>,
}

#[derive(Debug)]
struct AlignedRecord {
    left: String,
    right: String,
    size: usize,
}

impl AlignedRecord {
    fn new<T: Into<String>>(left: T, right: String) -> Self {
        let left = left.into();
        let size = left.len();
        AlignedRecord {
            left: left,
            right: right,
            size: size,
        }
    }

    fn display_with_size(&self, max_size: usize) -> String {
        format!(
            "{}{} = {}",
            self.left,
            " ".repeat(max_size - self.size),
            self.right
        )
    }
}

fn print_header(header: &str, width: usize) {
    let remaining_space = width - header.len() - 2; // 2 for the '=' on either end

    let left_spaces = " ".repeat(remaining_space / 2);
    let right_spaces = " ".repeat((remaining_space / 2) + remaining_space % 2);
    let mid_line = format!("={}{}{}=", &left_spaces, Green.paint(header), &right_spaces);

    println!(
        "\n{}\n{}\n{}\n",
        "=".repeat(width),
        mid_line,
        "=".repeat(width)
    );
}

fn format_optional_string(v: Option<String>) -> String {
    format!(
        "{}",
        match v {
            Some(s) => Green.paint(s),
            None => Red.paint("None"),
        }
    )
}

fn format_bool(v: bool) -> String {
    format!(
        "{}",
        match v {
            true => Green.paint("True"),
            false => Red.paint("False"),
        }
    )
}

fn print_variable(var: &FrozenVariable) {
    println!("{}: ", Cyan.paint(var.name.clone()));

    let records = vec![
        AlignedRecord::new("env", format_optional_string(var.env.clone())),
        AlignedRecord::new("cli_flag", format_optional_string(var.cli_flag.clone())),
        AlignedRecord::new(
            "readers",
            format!("{}", Green.paint(format!("{}", var.readers))),
        ),
        AlignedRecord::new(
            "writers",
            format!("{}", Green.paint(format!("{}", var.writers))),
        ),
        AlignedRecord::new(
            "value",
            format_optional_string(var.value.clone().map(|v| v.value)),
        ),
        AlignedRecord::new(
            "context",
            match &var.value {
                Some(v) => format!("{}", v.clone().updated_by),
                None => "Value has never been set".to_string(),
            },
        ),
    ];
    let mut max = 0;
    for r in &records {
        max = cmp::max(max, r.size);
    }

    for record in &records {
        println!("  - {}", record.display_with_size(max));
    }

    println!("");
}

fn print_tool(tool: &FrozenTool) {
    println!("{}: ", Cyan.paint(tool.name.clone()));

    let records = vec![
        AlignedRecord::new("is builtin", format_bool(tool.builtin)),
        AlignedRecord::new("path", format_optional_string(tool.path.clone())),
        AlignedRecord::new(
            "cmd",
            format_optional_string(tool.cmd.clone().map(|c| format!("{}", c.display()))),
        ),
    ];
    let mut max = 0;
    for r in &records {
        max = cmp::max(max, r.size);
    }

    for record in &records {
        println!("  - {}", record.display_with_size(max));
    }

    println!("");
}

impl RunCommand for DescribeArgs {
    fn run(&self, _global_args: &GlobalArgs) -> anyhow::Result<()> {
        if self.workflow.exists() {
            let column_width = 80;
            println!("Parsing workflow at {:?}", self.workflow);

            let parser = Parser::new();
            let module: Module = Module::new();
            let mut eval: Evaluator = Evaluator::new(&module);

            parser.parse_workflow_file(self.workflow.clone(), &mut eval)?;
            parser
                .ctx
                .update_from_environment(&self.workflow_args, &self.workflow);

            let snapshot = parser.ctx.snapshot();

            print_header("Variables", column_width);
            for v in snapshot.variables {
                print_variable(&v);
            }

            print_header("Tools", column_width);
            for t in snapshot.tools {
                print_tool(&t);
            }
        } else {
            bail!("Workflow does not exist at path {:?}", self.workflow);
        }
        Ok(())
    }
}
