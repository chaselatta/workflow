use crate::cmd::{GlobalArgs, RunCommand};
use crate::downcast_delegate_ref;
use crate::runner::{Runner, WorkflowDelegate};
use crate::stdlib::tool::Tool;
use crate::stdlib::Action;
use crate::stdlib::{VariableEntry, VariableRef};
use ansi_term::Colour::{Cyan, Green, Red};
use anyhow::bail;
use clap::Args;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::FrozenStringValue;
use std::cmp;
use std::ops::Deref;
use std::path::PathBuf;

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

fn format_result<T: std::fmt::Display>(v: anyhow::Result<T>) -> String {
    format!(
        "{}",
        match v {
            Ok(s) => Green.paint(s.to_string()),
            // TODO: return the actual error
            Err(_) => Red.paint("Error getting value"),
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

fn print_variable_entry(name: &str, var: &VariableEntry) {
    println!("{}: ", Cyan.paint(name.to_string()));
    let value_ctx = var.value_ctx();

    let records = vec![
        AlignedRecord::new("env", format_optional_string(var.env())),
        AlignedRecord::new("cli_flag", format_optional_string(var.cli_flag())),
        AlignedRecord::new(
            "readers",
            format!("{}", Green.paint(format!("{}", var.readers()))),
        ),
        AlignedRecord::new(
            "writers",
            format!("{}", Green.paint(format!("{}", var.writers()))),
        ),
        AlignedRecord::new(
            "value",
            format_optional_string(value_ctx.clone().map(|v| v.value)),
        ),
        AlignedRecord::new(
            "context",
            match &value_ctx {
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

fn print_tool(name: &str, tool: &Tool, delegate: &WorkflowDelegate, working_dir: &PathBuf) {
    println!("{}: ", Cyan.paint(name.to_string()));

    let records = vec![
        AlignedRecord::new("is builtin", format_bool(tool.is_builtin())),
        AlignedRecord::new(
            "path",
            format_result(
                tool.path(delegate, working_dir)
                    .map(|p| format!("{}", p.display())),
            ),
        ),
        AlignedRecord::new(
            "real_path",
            format_result(
                tool.real_path(delegate, working_dir)
                    .map(|p| format!("{}", p.display())),
            ),
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

            let runner = Runner::new(
                self.workflow.clone(),
                WorkflowDelegate::with_args(self.workflow_args.clone()),
            )?;
            let module: Module = Module::new();
            let mut eval: Evaluator = Evaluator::new(&module);

            let _result = runner.parse_workflow(&mut eval).unwrap();

            let holder = runner.delegate();
            let delegate = downcast_delegate_ref!(holder, WorkflowDelegate).unwrap();
            let working_dir = runner.working_dir();

            let mut vars: Vec<(FrozenStringValue, &VariableRef)> = Vec::new();
            let mut tools: Vec<(FrozenStringValue, &Tool)> = Vec::new();
            let mut actions: Vec<(FrozenStringValue, &Action)> = Vec::new();

            let names = module.names();
            for name in names {
                if let Some(value) = module.get(&name) {
                    if let Some(entry) = VariableRef::from_value(value) {
                        vars.push((name, entry));
                    } else if let Some(entry) = Tool::from_value(value) {
                        tools.push((name, entry));
                    } else if let Some(entry) = Action::from_value(value) {
                        actions.push((name, entry));
                    }
                }
            }

            print_header("Variables", column_width);
            for (name, var) in vars {
                delegate
                    .variable_store()
                    .with_variable(var.identifier(), |v| {
                        print_variable_entry(&name, v);
                    });
            }

            print_header("Tools", column_width);
            for (name, tool) in tools {
                print_tool(&name, &tool, &delegate, &working_dir);
            }

            print_header("Actions", column_width);
            for (name, action) in actions {
                dbg!(&name);
                dbg!(action);
            }
        } else {
            bail!("Workflow does not exist at path {:?}", self.workflow);
        }
        Ok(())
    }
}
