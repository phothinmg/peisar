//! # peisar
//!
//! A practical Markdown parser supporting CommonMark, GitHub Flavored
//! Markdown (GFM), Kramdown-style block attributes, a plugin system, and
//! configurable HTML output (full document or fragment).

pub mod ast;
pub mod config;
pub mod html;
pub mod parser;
pub mod plugin_factory;
pub mod plugins;
pub mod visitor;

#[cfg(test)]
mod tests;
