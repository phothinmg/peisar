//! Static-site configuration for the Peisar SSG.
//!
//! The configuration lives in a [`CONFIG_FILE_NAME`] file at the root of the
//! site project. This module reads that file, validates it, and resolves it
//! into a [`PeisarSSGConfig`] in which every optional value has been replaced
//! by either the value from the file or a documented default — consumers never
//! have to handle `Option`s themselves.
//!
//! # File format
//!
//! ```toml
//! [site]
//! title = "My Site"           # required; the only mandatory key
//! description = "A blog"      # optional
//! author = "Ada Lovelace"     # optional
//!
//! [directories]               # optional table; every key is optional
//! rootDir = "."               # default
//! contentsDir = "contents"    # default
//! publicDir = "public"        # default
//! dataDir = "data"            # default
//! outDir = ".peisar"          # default
//!
//! [devServer]                 # optional table; every key is optional
//! port = 3838                 # default
//! host = "127.0.0.1"          # default
//! ```
//!
//! Table names and keys are matched case-sensitively and use camelCase
//! (`[devServer]`, `rootDir`, …). Snake-case spellings such as `[dev_server]`
//! or `root_dir` are unknown keys: they are ignored and the documented default
//! is used instead. The same goes for any other unknown key — the file may
//! carry extra keys without breaking the loader.
//!
//! # Errors
//!
//! Loading fails with a [`PeisarConfigError`] when the file cannot be read
//! ([`PeisarConfigError::Io`]), when it is not valid TOML or does not match the
//! schema ([`PeisarConfigError::Parse`]), or when `[site] title` is missing or
//! empty ([`PeisarConfigError::MissingTitle`]).
//!
//! The Rust-facing [`load_config`] returns these errors as values. The
//! JavaScript-facing [`peisar_ssg_config`] instead prints the error message to
//! stderr and terminates the host process with exit code `1`.
//!
//! See [`parse_config`] for a runnable example.

use std::fmt;
use std::path::Path;

use napi_derive::napi;
use serde::Deserialize;

/// Name of the configuration file that is looked up in the current directory.
pub const CONFIG_FILE_NAME: &str = "Peisar.toml";

/// The `[site]` table of `Peisar.toml`, as written in the file.
#[derive(Deserialize, Debug)]
struct PeisarSite {
    /// Site title — the only required key in the file.
    pub title: String,
    /// Optional site description.
    pub description: Option<String>,
    /// Optional site author.
    pub author: Option<String>,
}

/// The optional `[directories]` table of `Peisar.toml`, as written in the file.
///
/// Keys are camelCase (`rootDir`, `contentsDir`, …).
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PeisarDirectories {
    /// Project root directory.
    pub root_dir: Option<String>,
    /// Directory containing the markdown contents.
    pub contents_dir: Option<String>,
    /// Directory with static assets copied verbatim to the output.
    pub public_dir: Option<String>,
    /// Directory containing the data.
    pub data_dir: Option<String>,
    /// Build output directory.
    pub out_dir: Option<String>,
}

/// The optional `[devServer]` table of `Peisar.toml`, as written in the file.
#[derive(Deserialize, Debug)]
struct PeisarDevServer {
    /// TCP port the dev server listens on.
    pub port: Option<i32>,
    /// Address the dev server binds to.
    pub host: Option<String>,
}

/// The raw shape of `Peisar.toml` before defaults are applied.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PeisarToml {
    /// The `[site]` table; the only required table.
    pub site: PeisarSite,
    /// The optional `[directories]` table.
    pub directories: Option<PeisarDirectories>,
    /// The optional `[devServer]` table.
    pub dev_server: Option<PeisarDevServer>,
}

/// Site metadata, resolved from the `[site]` table of `Peisar.toml`.
///
/// In JavaScript the fields keep their names: `title`, `description`, `author`.
#[napi(object)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct PeisarSSGSite {
    /// Site title (required and non-empty).
    pub title: String,
    /// Site description, if the config file sets one.
    pub description: Option<String>,
    /// Site author, if the config file sets one.
    pub author: Option<String>,
}

/// Directory configuration, resolved from the optional `[directories]` table
/// of `Peisar.toml`.
///
/// In JavaScript the fields appear in camelCase: `rootDir`, `contentsDir`,
/// `publicDir`, `themeDir` and `outDir`.
#[napi(object)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeisarSSGDirectories {
    /// Project root directory. Default: `"."`.
    pub root_dir: String,
    /// Directory containing the markdown contents. Default: `"contents"`.
    pub contents_dir: String,
    /// Directory with static assets copied verbatim to the output.
    /// Default: `"public"`.
    pub public_dir: String,
    /// Directory containing the theme. Default: `"data"`.
    pub data_dir: String,
    /// Build output directory. Default: `".peisar"`.
    pub out_dir: String,
}

impl Default for PeisarSSGDirectories {
    fn default() -> Self {
        Self {
            root_dir: ".".to_string(),
            contents_dir: "contents".to_string(),
            public_dir: "public".to_string(),
            data_dir: "data".to_string(),
            out_dir: ".peisar".to_string(),
        }
    }
}

/// Development-server settings, resolved from the optional `[devServer]` table
/// of `Peisar.toml`.
#[napi(object)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct PeisarSSGDevServer {
    /// TCP port the dev server listens on. Default: `3838`.
    pub port: i32,
    /// Address the dev server binds to. Default: `"127.0.0.1"`.
    pub host: String,
}

impl Default for PeisarSSGDevServer {
    fn default() -> Self {
        Self {
            port: 3838,
            host: "127.0.0.1".to_string(),
        }
    }
}

/// Fully-resolved site configuration.
///
/// Every optional value from `Peisar.toml` has been replaced by either the
/// value from the file or its documented default, so consumers never have to
/// handle `Option`s.
#[napi(object)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeisarSSGConfig {
    /// Site metadata.
    pub site: PeisarSSGSite,
    /// Directory configuration.
    pub directories: PeisarSSGDirectories,
    /// Development-server configuration.
    pub dev_server: PeisarSSGDevServer,
}

/// Errors that can occur while loading the site configuration.
#[derive(Debug)]
pub enum PeisarConfigError {
    /// [`CONFIG_FILE_NAME`] could not be found or read; the underlying I/O
    /// error is available via [`std::error::Error::source`].
    Io(std::io::Error),
    /// The file was read, but it is not valid TOML or does not match the
    /// expected schema.
    Parse(toml::de::Error),
    /// The file parsed, but `[site] title` is empty.
    MissingTitle,
}

impl fmt::Display for PeisarConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Keep these messages stable: `peisar_ssg_config` prints them to
            // stderr right before terminating the process.
            PeisarConfigError::Io(_) => write!(f, "No config file found"),
            PeisarConfigError::Parse(err) => write!(f, "Invalid config file: {err}"),
            PeisarConfigError::MissingTitle => write!(f, "Site title required in config file."),
        }
    }
}

impl std::error::Error for PeisarConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PeisarConfigError::Io(err) => Some(err),
            PeisarConfigError::Parse(err) => Some(err),
            PeisarConfigError::MissingTitle => None,
        }
    }
}

/// Parses a `Peisar.toml` document into a fully-resolved [`PeisarSSGConfig`].
///
/// Optional tables and keys fall back to their documented defaults. This is a
/// pure function — it performs no file-system access — which makes it
/// convenient for tests and tooling. Use [`load_config`] to read and parse the
/// config file of the current directory in one step.
///
/// # Errors
///
/// Returns [`PeisarConfigError::Parse`] if `input` is not valid TOML or does
/// not match the schema (for example when the `[site]` table or its `title`
/// key is missing), and [`PeisarConfigError::MissingTitle`] if `title` is
/// present but empty.
///
/// # Example
///
/// ```
/// use peisar::ssg::ssg_config::parse_config;
///
/// let config = parse_config(r#"
/// [site]
/// title = "Peisar"
///
/// [directories]
/// outDir = "build"
/// "#)
/// .unwrap();
///
/// assert_eq!(config.site.title, "Peisar");
/// assert_eq!(config.directories.out_dir, "build");
/// // Keys left out of the file fall back to their defaults:
/// assert_eq!(config.directories.contents_dir, "contents");
/// assert_eq!(config.dev_server.port, 3838);
/// ```
pub fn parse_config(input: &str) -> Result<PeisarSSGConfig, PeisarConfigError> {
    let config: PeisarToml = toml::from_str(input).map_err(PeisarConfigError::Parse)?;

    if config.site.title.is_empty() {
        return Err(PeisarConfigError::MissingTitle);
    }

    let site = PeisarSSGSite {
        title: config.site.title,
        description: config.site.description,
        author: config.site.author,
    };

    let directory_defaults = PeisarSSGDirectories::default();
    let directories = match config.directories {
        None => directory_defaults,
        Some(dirs) => PeisarSSGDirectories {
            root_dir: dirs.root_dir.unwrap_or(directory_defaults.root_dir),
            contents_dir: dirs.contents_dir.unwrap_or(directory_defaults.contents_dir),
            public_dir: dirs.public_dir.unwrap_or(directory_defaults.public_dir),
            data_dir: dirs.data_dir.unwrap_or(directory_defaults.data_dir),
            out_dir: dirs.out_dir.unwrap_or(directory_defaults.out_dir),
        },
    };

    let server_defaults = PeisarSSGDevServer::default();
    let dev_server = match config.dev_server {
        None => server_defaults,
        Some(server) => PeisarSSGDevServer {
            port: server.port.unwrap_or(server_defaults.port),
            host: server.host.unwrap_or(server_defaults.host),
        },
    };

    Ok(PeisarSSGConfig {
        site,
        directories,
        dev_server,
    })
}

/// Reads [`CONFIG_FILE_NAME`] from `dir` and returns its contents.
///
/// Fails with [`PeisarConfigError::Io`] (of kind
/// [`std::io::ErrorKind::NotFound`]) when the file does not exist or is not a
/// regular file.
fn read_toml_config_from(dir: &Path) -> Result<String, PeisarConfigError> {
    let config_path = dir.join(CONFIG_FILE_NAME);
    if !config_path.is_file() {
        return Err(PeisarConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("`{}` not found in `{}`", CONFIG_FILE_NAME, dir.display()),
        )));
    }
    std::fs::read_to_string(config_path).map_err(PeisarConfigError::Io)
}

/// Loads the site configuration from [`CONFIG_FILE_NAME`] in the current
/// working directory.
///
/// This is the fallible, library-friendly counterpart of [`peisar_ssg_config`]:
/// instead of terminating the process on failure it returns the
/// [`PeisarConfigError`], which makes it the right choice for Rust consumers
/// such as the CLI.
///
/// # Example
///
/// ```no_run
/// use peisar::ssg::ssg_config::load_config;
///
/// match load_config() {
///     Ok(config) => println!("serving \"{}\"", config.site.title),
///     Err(err) => {
///         eprintln!("{err}");
///         std::process::exit(1);
///     }
/// }
/// ```
pub fn load_config() -> Result<PeisarSSGConfig, PeisarConfigError> {
    let cwd = std::env::current_dir().map_err(PeisarConfigError::Io)?;
    let toml_str = read_toml_config_from(&cwd)?;
    parse_config(&toml_str)
}

/// N-API entry point: loads the site configuration from `Peisar.toml` in the
/// current working directory.
///
/// Exposed to JavaScript as `peisarSsgConfig()`. On success it returns a
/// fully-resolved [`PeisarSSGConfig`] object. On failure it prints the error
/// message to stderr and terminates the host process with exit code `1` — a
/// missing file, an invalid file and an empty `[site] title` are all fatal.
///
/// Rust consumers should prefer [`load_config`], which reports errors as
/// values instead of exiting.
#[napi]
pub fn peisar_ssg_config() -> PeisarSSGConfig {
    match load_config() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests;
