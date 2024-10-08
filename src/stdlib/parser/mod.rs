pub mod parse_context;

use crate::stdlib::legacy::tool::{starlark_builtin_tool, starlark_tool};
use crate::stdlib::legacy::variable::starlark_variable;
use crate::stdlib::parser::parse_context::ParseContext;

use starlark::environment::{Globals, GlobalsBuilder};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use std::fs;
use std::path::PathBuf;

pub struct Parser {
    globals: Globals,
    pub ctx: ParseContext,
}

fn globals() -> Globals {
    GlobalsBuilder::standard()
        .with(starlark_variable)
        .with(starlark_builtin_tool)
        .with(starlark_tool)
        .build()
}

fn try_starlark<T>(r: Result<T, starlark::Error>) -> anyhow::Result<T> {
    match r {
        Ok(v) => Ok(v),
        Err(e) => Err(e.into_anyhow()),
    }
}

impl Parser {
    pub fn new(workflow_file: PathBuf) -> anyhow::Result<Self> {
        let globals = globals();
        let ctx = ParseContext::new(fs::canonicalize(workflow_file)?);

        return Ok(Parser {
            globals: globals,
            ctx: ctx,
        });
    }

    pub fn parse_workflow<'a>(&'a self, eval: &mut Evaluator<'a, 'a>) -> anyhow::Result<()> {
        eval.extra = Some(&self.ctx);

        let ast = try_starlark(AstModule::parse_file(
            self.ctx.workflow_file(),
            &Dialect::Standard,
        ))?;
        try_starlark(eval.eval_module(ast, &self.globals))?;

        Ok(())
    }
}

pub trait StringInterpolator {
    /// Interpolate the given string for the given reader.
    fn interpolate(&self, s: &str, reader: &str) -> anyhow::Result<String>;
}

impl StringInterpolator for &str {
    fn interpolate(&self, _s: &str, _reader: &str) -> anyhow::Result<String> {
        Ok(self.to_string())
    }
}

pub struct NoStringInterp {}
impl StringInterpolator for NoStringInterp {
    fn interpolate(&self, s: &str, _reader: &str) -> anyhow::Result<String> {
        Ok(s.to_string())
    }
}
pub const NO_STRING_INTERP: &NoStringInterp = &NoStringInterp {};

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;

    #[test]
    fn test_parse_file() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/legacy_vars_only.workflow");

        let parser = Parser::new(file).unwrap();
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);

        parser.parse_workflow(&mut eval).unwrap();

        assert_eq!(parser.ctx.snapshot().variables.len(), 3);
    }

    #[test]
    #[should_panic(expected = "No such file or directory")]
    fn test_parser_create_fail() {
        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/__no_file__.workflow");

        Parser::new(file).unwrap();
    }
}
