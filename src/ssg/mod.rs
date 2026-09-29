//! Static-site-generation support built on top of the Markdown pipeline.
//!
//! - [`cache`] — [`PeisarCache`](cache::PeisarCache), an in-memory cache of
//!   markdown and asset files with recursive file watching, disk
//!   persistence, and JS change subscriptions.
//! - `files` — internal file-discovery helpers (extension lists and
//!   recursive collection).
//! - [`ssg_config`] — loading and validating the `Peisar.toml` site config
//!   ([`parse_config`](ssg_config::parse_config), [`load_config`](ssg_config::load_config)).

pub mod cache;
mod files;
pub mod ssg_config;
