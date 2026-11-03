use peisar::ast::{AstOptions, Block, Document};
use peisar::plugin::{InlineVisitControl, PluginFactory, VisitControl, visit_document_mut};

struct DemoteHeadings;

impl PluginFactory for DemoteHeadings {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        if let Block::Heading { level, .. } = block {
            *level = (*level + 1).min(6);
        }
        VisitControl::keep_and_recurse()
    }
    fn visit_inline(&mut self, _inline: &mut peisar::ast::Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

fn main() {
    let document = Document::parse("# Hello\n", &AstOptions::default(), None);
    let mut document = document;
    visit_document_mut(&mut document, &mut DemoteHeadings);
    let doc_json = serde_json::to_string_pretty(&document).unwrap();
    println!("{}", doc_json)
}
