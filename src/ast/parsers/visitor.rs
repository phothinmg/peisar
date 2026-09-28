//! AST visitor trait and traversal engine.
//!
//! The [`AstVisitor`] trait provides a pre-order callback API for visiting
//! and mutating every block- and inline-level node in a [`Document`](super::Document).
//!
//! ## Usage
//!
//! Implement [`AstVisitor`] for a struct, register it with
//! [`PeisarAst::add_visitor`](crate::PeisarAst::add_visitor), then call
//! [`PeisarAst::visit_all`](crate::PeisarAst::visit_all).  Each visitor
//! method receives a `&mut` reference to the node and returns a control
//! struct that can request insertion, replacement, or removal.
//!
//! ## Example
//!
//! ```ignore
//! use peisar_ast::AstVisitor;
//! use peisar_ast::token::{Block, Inline};
//! use peisar_ast::parsers::visitor::{VisitControl, InlineVisitControl};
//!
//! struct LinkCounter { count: usize }
//! impl AstVisitor for LinkCounter {
//!     fn visit_inline(&mut self, inline: &mut Inline) -> InlineVisitControl {
//!         if matches!(inline, Inline::Link { .. } | Inline::LinkReference { .. }) {
//!             self.count += 1;
//!         }
//!         InlineVisitControl::default()
//!     }
//! }
//! ```

use crate::ast::tokens::token::{Block, Inline};

/// Visitor trait for AST nodes. Implement this trait to receive callbacks
/// for every block- and inline-level node. The visitor methods receive a
/// mutable reference and return a control value that can request the
/// node be replaced, removed, or have nodes inserted around it.
pub trait AstVisitor {
    /// Called for every block-level node (pre-order). Return a
    /// `VisitControl` to indicate modifications. The callback may also
    /// mutate the node in-place.
    fn visit_block(&mut self, _block: &mut Block) -> VisitControl {
        VisitControl::default()
    }

    /// Called for every inline-level node (pre-order). Return an
    /// `InlineVisitControl` to indicate modifications. The callback may
    /// also mutate the inline node in-place.
    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

/// Control returned from `visit_block` describing edits to perform.
#[derive(Debug, Default)]
pub struct VisitControl {
    /// Nodes to insert before the current node's position.
    pub insert_before: Vec<Block>,
    /// Nodes to insert after the current node's position.
    pub insert_after: Vec<Block>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Block>>,
    /// Remove the current node entirely.
    pub remove: bool,
    /// Whether to recurse into this node's child nodes (when applicable).
    /// Default `false`.
    pub recurse: bool,
}

impl VisitControl {
    /// Convenience: keep the (possibly mutated) node and recurse into children.
    pub fn keep_and_recurse() -> Self {
        VisitControl {
            recurse: true,
            ..Default::default()
        }
    }

    /// Convenience: remove the node.
    pub fn remove() -> Self {
        VisitControl {
            remove: true,
            ..Default::default()
        }
    }

    /// Convenience: replace current node with given nodes.
    pub fn replace_with(nodes: Vec<Block>) -> Self {
        VisitControl {
            replace_with: Some(nodes),
            ..Default::default()
        }
    }
}

/// Control returned from `visit_inline` describing edits to perform on inline
/// nodes. Mirrors `VisitControl` but for `Inline` nodes.
#[derive(Debug, Default)]
pub struct InlineVisitControl {
    /// Nodes to insert before the current node's position.
    pub insert_before: Vec<Inline>,
    /// Nodes to insert after the current node's position.
    pub insert_after: Vec<Inline>,
    /// Replace the current node with these nodes.
    pub replace_with: Option<Vec<Inline>>,
    /// Remove the current node entirely.
    pub remove: bool,
    /// Whether to recurse into this node's child nodes (when applicable).
    pub recurse: bool,
}

impl InlineVisitControl {
    /// Convenience: keep the (possibly mutated) node and recurse into children.
    pub fn keep_and_recurse() -> Self {
        InlineVisitControl {
            recurse: true,
            ..Default::default()
        }
    }
    /// Convenience: remove the node.
    pub fn remove() -> Self {
        InlineVisitControl {
            remove: true,
            ..Default::default()
        }
    }
    /// Convenience: replace current node with given nodes.
    pub fn replace_with(nodes: Vec<Inline>) -> Self {
        InlineVisitControl {
            replace_with: Some(nodes),
            ..Default::default()
        }
    }
}

/// Visit and possibly mutate a whole document. Traversal order is pre-order
/// (visitor is called before recursing into children). The visitor may:
/// - mutate the node in-place and return a default/keep control
/// - return insert_before / insert_after vectors
/// - return replace_with to swap the node for one or more nodes
/// - return remove to drop the node entirely
pub fn visit_document_mut<V: AstVisitor + ?Sized>(doc: &mut super::Document, visitor: &mut V) {
    visit_blocks_vec(&mut doc.children, visitor)
}

fn visit_blocks_vec<V: AstVisitor + ?Sized>(children: &mut Vec<Block>, visitor: &mut V) {
    let mut i = 0usize;
    while i < children.len() {
        // take ownership of the node so the visitor can freely replace/remove it
        let mut node = children.remove(i);

        // call visitor
        let ctrl = visitor.visit_block(&mut node);
        // destructure control so we can move fields
        let VisitControl {
            insert_before,
            insert_after,
            replace_with,
            remove,
            recurse,
        } = ctrl;

        // insert_before: put these at the original position
        if !insert_before.is_empty() {
            let before_len = insert_before.len();
            children.splice(i..i, insert_before);
            i += before_len;
        }

        // removal requested
        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                children.splice(i..i, insert_after);
                i += after_len;
            }
            // original removed, continue (don't increment i)
            continue;
        }

        // replacement requested
        if let Some(reps) = replace_with {
            let replacement_len = reps.len();
            children.splice(i..i, reps);
            i += replacement_len;
            continue;
        }

        // keep the (possibly mutated) node; recurse into children if requested
        if recurse {
            match &mut node {
                Block::Heading {
                    children: inline_children,
                    ..
                }
                | Block::Paragraph {
                    children: inline_children,
                    ..
                } => {
                    visit_inlines_vec(inline_children, visitor);
                }
                Block::BlockQuote {
                    children: block_children,
                    ..
                } => {
                    visit_blocks_vec(block_children, visitor);
                }
                Block::List { items, .. } => {
                    for item in items.iter_mut() {
                        visit_blocks_vec(&mut item.children, visitor);
                    }
                }
                Block::Table { table, .. } => {
                    visit_table(table, visitor);
                }
                // CodeBlock, ThematicBreak, HtmlBlock, LinkReferenceDefinition,
                // and Comment have no nested AST children
                _ => {}
            }
        }

        // insert the original (possibly mutated) node back into the vector
        children.insert(i, node);
        i += 1;

        // insert_after
        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            children.splice(i..i, insert_after);
            i += after_len;
        }
    }
}

fn visit_table<V: AstVisitor + ?Sized>(
    table: &mut crate::ast::tokens::token::Table,
    visitor: &mut V,
) {
    for cell in table.header.cells.iter_mut() {
        visit_inlines_vec(&mut cell.children, visitor);
    }
    for row in table.rows.iter_mut() {
        for cell in row.cells.iter_mut() {
            visit_inlines_vec(&mut cell.children, visitor);
        }
    }
}

fn visit_inlines_vec<V: AstVisitor + ?Sized>(inlines: &mut Vec<Inline>, visitor: &mut V) {
    let mut i = 0usize;
    while i < inlines.len() {
        let mut node = inlines.remove(i);
        let ctrl = visitor.visit_inline(&mut node);
        let InlineVisitControl {
            insert_before,
            insert_after,
            replace_with,
            remove,
            recurse,
        } = ctrl;

        if !insert_before.is_empty() {
            let before_len = insert_before.len();
            inlines.splice(i..i, insert_before);
            i += before_len;
        }

        if remove {
            if !insert_after.is_empty() {
                let after_len = insert_after.len();
                inlines.splice(i..i, insert_after);
                i += after_len;
            }
            continue;
        }
        #[allow(unused_mut)]
        if let Some(reps) = replace_with {
            inlines.splice(i..i, reps);
            continue;
        }

        if recurse {
            match &mut node {
                Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
                    visit_inlines_vec(children, visitor);
                }
                Inline::Link { text, .. } | Inline::LinkReference { text, .. } => {
                    visit_inlines_vec(text, visitor);
                }
                // Text, Code, Image, HtmlInline, HardBreak, SoftBreak have no nested inlines
                _ => {}
            }
        }

        inlines.insert(i, node);
        i += 1;

        if !insert_after.is_empty() {
            let after_len = insert_after.len();
            inlines.splice(i..i, insert_after);
            i += after_len;
        }
    }
}
