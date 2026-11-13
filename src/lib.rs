//! Markdown parsing and rendering with an extensible abstract syntax tree.
//!

mod config;
mod html;
mod parser;
mod peisar_core;

pub use config::PeisarOptions;
pub use parser::Peisar;
pub use peisar_core::frontmatter::{Frontmatter, FrontmatterResult, FrontmatterType};

pub mod ast {
    use super::peisar_core;

    pub use peisar_core::PeisarAst;
    pub use peisar_core::options::AstOptions;
    pub use peisar_core::parsers::md_to_ast;
    pub mod tokens {
        pub use super::peisar_core::{
            parsers::PeisarDocument,
            tokens::{
                Attributes,
                token::{
                    Block, EmphasisLevel, Inline, LinkReferenceDefinition, ListItem, Table,
                    TableCell, TableCellAlignment, TableRow, TaskState,
                },
            },
        };
    }

    pub mod span {
        pub use super::peisar_core::tokens::span::{Position, Span};
    }

    pub mod parsers {
        pub use super::peisar_core::parsers::{attrs, block, inline};
    }
}

pub mod render {
    pub use super::config::RenderOptions;
    pub use super::html::{AstToHtml, ast_to_html};
}

pub mod plugin {
    pub use super::peisar_core::{
        BlockParseResult, InlineParseResult, InlineVisitorControl, Plugin, VisitorControl,
        parsers::plugin::{BlockParserContext, InlineParserContext, ParserHooks, PluginFactory},
    };
}
