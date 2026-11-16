//! Markdown parsing and rendering with an extensible abstract syntax tree.
//!
//! Parsing logic inspired by https://github.com/progsource/maddy
//! (C++ Markdown to HTML header-only parser library)
//!
//! *********************************************************************************
//! Copyright 2026 Pho Thin Maung
//! Licensed under the Apache License, Version 2.0 (the "License"); you may not use
//! this file except in compliance with the License. You may obtain a copy of the
//! License at http://www.apache.org/licenses/LICENSE-2.0
//!
//! THIS CODE IS PROVIDED ON AN *AS IS* BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
//! KIND, EITHER EXPRESS OR IMPLIED, INCLUDING WITHOUT LIMITATION ANY IMPLIED
//! WARRANTIES OR CONDITIONS OF TITLE, FITNESS FOR A PARTICULAR PURPOSE,
//! MERCHANTABILITY OR NON-INFRINGEMENT.
//!
//! See the Apache Version 2.0 License for specific language governing permissions
//! and limitations under the License.
//! ***********************************************************************************

mod parser;
mod peisar_core;

use peisar_core::config;
use peisar_core::html;
// ---------------------------------------------------------------------------------------------
pub mod toml;
pub mod yaml;
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
        BlockParseResult, InlineParseResult, InlineVisitorControl, PeisarPlugin, VisitorControl,
        parsers::plugin::{BlockParserContext, InlineParserContext, ParserHooks, PluginFactory},
    };
}
