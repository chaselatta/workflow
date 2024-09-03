pub mod parse_context;

use crate::stdlib::parser::parse_context::ParseContext;
use crate::stdlib::variable::starlark_variable;

use starlark::environment::Globals;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use std::path::PathBuf;

pub struct Parser {
    globals: Globals,
    pub ctx: ParseContext,
}

fn globals() -> Globals {
    GlobalsBuilder::new().with(starlark_variable).build()
}

impl Parser {
    pub fn new() -> Self {
        let globals = globals();
        let ctx = ParseContext::default();

        return Parser {
            globals: globals,
            ctx: ctx,
        };
    }

    pub fn parse_content<'a>(
        &'a self,
        filename: &str,
        content: String,
        eval: &mut Evaluator<'a, 'a>,
    ) -> anyhow::Result<()> {
        eval.extra = Some(&self.ctx);
        // TODO: fix unwrap
        let ast: AstModule = AstModule::parse(filename, content, &Dialect::Standard).unwrap();

        // TODO: fix unwrap
        let _ = eval.eval_module(ast, &self.globals).unwrap();
        Ok(())
    }

    pub fn parse_workflow_file<'a>(
        &'a self,
        file: PathBuf,
        eval: &mut Evaluator<'a, 'a>,
    ) -> anyhow::Result<()> {
        eval.extra = Some(&self.ctx);
        //TODO: check filename ends with .workflow
        // TODO: fix unwrap
        let ast: AstModule = AstModule::parse_file(&file, &Dialect::Standard).unwrap();

        // TODO: fix unwrap
        let _ = eval.eval_module(ast, &self.globals).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;

    #[test]
    fn test_parser_new() {
        let _ = Parser::new();
    }

    #[test]
    fn test_parse_content() {
        let content = r#"
variable(
  name = "foo"
)
def foo():
  pass
"#;
        let parser = Parser::new();
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);

        parser
            .parse_content("foo.star", content.to_owned(), &mut eval)
            .unwrap();

        assert_eq!(parser.ctx.snapshot_variables().len(), 1);
    }

    #[test]
    fn test_parse_file() {
        let parser = Parser::new();
        let module: Module = Module::new();
        let mut eval: Evaluator = Evaluator::new(&module);

        let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file.push("src/test_data/vars_only.workflow");

        parser.parse_workflow_file(file, &mut eval).unwrap();

        assert_eq!(parser.ctx.snapshot_variables().len(), 3);
    }
}
