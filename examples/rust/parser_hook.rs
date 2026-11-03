use peisar::ast::{AstOptions, Document, md_to_ast_with_hooks};
use peisar::plugin::{BlockParserContext, ParserHooks, PluginFactory};

struct BangBreak;

impl PluginFactory for BangBreak {
    fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(peisar::ast::Block, usize)> {
        if ctx.line.trim() != "!" {
            return None; // decline
        }
        let block = peisar::ast::Block::ThematicBreak {
            pos: Default::default(),
        };
        Some((block, 1))
    }
}

fn main() {
    let hooks = ParserHooks::empty().with(&BangBreak);
    let document: Document = md_to_ast_with_hooks("!\n", &AstOptions::default(), None, &hooks);
    let doc_json = serde_json::to_string_pretty(&document).unwrap();
    println!("{}", doc_json)
}
