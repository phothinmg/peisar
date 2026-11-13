use peisar::ast::tokens::Block;
use peisar::plugin::{BlockParseResult, Plugin, VisitorControl};
use peisar::{Peisar, PeisarOptions};

/// Adds a thematic break after every heading.
///
/// The visitor returns no changes for other block types. Unspecified plugin
/// callbacks use their default value, so this plugin does not add parser hooks.
fn visitor_plugin() -> Plugin {
    Plugin {
        visit_block: Some(Box::new(|block| {
            if matches!(block, Block::Heading { .. }) {
                Some(VisitorControl {
                    insert_after: Some(vec![Block::ThematicBreak {
                        pos: Default::default(),
                    }]),
                    ..Default::default()
                })
            } else {
                None
            }
        })),
        ..Default::default()
    }
}

/// Parses a line containing `!!!` as a thematic break.
///
/// Returning `None` for other lines lets the built-in Markdown parser handle
/// them. The custom syntax occupies one source line, so the hook consumes one.
fn parser_plugin() -> Plugin {
    Plugin {
        parse_block: Some(Box::new(|ctx| {
            if ctx.line.trim() != "!!!" {
                return None;
            }

            Some(BlockParseResult {
                block: Some(Block::ThematicBreak {
                    pos: Default::default(),
                }),
                consumed: Some(1),
            })
        })),
        ..Default::default()
    }
}

/// Runs both examples and prints the HTML produced by each plugin.
fn main() {
    let mut visitor_example = Peisar::new("# Hello\n".to_string(), Some(PeisarOptions::default()));
    visitor_example.use_plugin(visitor_plugin());
    println!("Visitor plugin:\n{}", visitor_example.html());

    let mut parser_example = Peisar::new(
        "# Hello\n\n!!!\n".to_string(),
        Some(PeisarOptions::default()),
    );
    parser_example.use_plugin(parser_plugin());
    println!("Parser plugin:\n{}", parser_example.html());
}
