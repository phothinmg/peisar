mod config;
pub mod html;
mod parser;

pub use config::{PeisarOptions, get_options};
pub use parser::Peisar;

#[cfg(feature = "html")]
pub use config::RenderOptions;
