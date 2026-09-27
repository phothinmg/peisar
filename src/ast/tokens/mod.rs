//! AST token (node) type definitions.
//!
//! This module groups the three sub-modules that define the shape of the
//! AST:
//!
//! - [`span`] — source position tracking ([`Span`], [`Position`]).
//! - [`token`] — the [`Block`] and [`Inline`] enums plus supporting structs.
//! - `attrs` — Kramdown block attributes ([`Attributes`]).
//!
//! [`Span`]: span::Span
//! [`Position`]: span::Position
//! [`Block`]: token::Block
//! [`Inline`]: token::Inline
//! [`Attributes`]: Attributes

mod attrs;
pub mod span;
pub mod token;

pub use attrs::Attributes;
