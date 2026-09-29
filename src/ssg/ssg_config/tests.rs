//! Unit tests for the site configuration module.
//!
//! These cover the pure [`parse_config`] logic (defaults, camelCase keys,
//! validation) plus the file-reading helpers, which are exercised through
//! temporary directories.

use std::io;
use std::path::{Path, PathBuf};

use super::*;

/// A unique temporary directory that is removed recursively when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "peisar_utils-site_config-{}-{tag}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A config that exercises every table and every key.
const FULL_CONFIG: &str = r#"
[site]
title = "Peisar"
description = "A static site"
author = "Ada"

[directories]
rootDir = "/srv/site"
contentsDir = "docs"
publicDir = "static"
themeDir = "skins"
outDir = "build"

[devServer]
port = 8080
host = "0.0.0.0"
"#;

/// Parses `input`, panicking if it is rejected.
fn parse(input: &str) -> PeisarSSGConfig {
    parse_config(input).unwrap()
}

/// Parses `input`, panicking if it is accepted.
fn parse_err(input: &str) -> PeisarConfigError {
    parse_config(input).expect_err("expected the config to be rejected")
}

/// Writes `contents` to `<dir>/Peisar.toml`.
fn write_config(dir: &Path, contents: &str) {
    std::fs::write(dir.join(CONFIG_FILE_NAME), contents).unwrap();
}

#[test]
fn defaults_match_the_documented_values() {
    let dirs = PeisarSSGDirectories::default();
    assert_eq!(dirs.root_dir, ".");
    assert_eq!(dirs.contents_dir, "contents");
    assert_eq!(dirs.public_dir, "public");
    assert_eq!(dirs.theme_dir, "themes");
    assert_eq!(dirs.out_dir, ".peisar");

    let server = PeisarSSGDevServer::default();
    assert_eq!(server.port, 3838);
    assert_eq!(server.host, "127.0.0.1");
}

#[test]
fn parses_every_field_from_a_full_config() {
    let config = parse(FULL_CONFIG);

    assert_eq!(config.site.title, "Peisar");
    assert_eq!(config.site.description.as_deref(), Some("A static site"));
    assert_eq!(config.site.author.as_deref(), Some("Ada"));

    assert_eq!(config.directories.root_dir, "/srv/site");
    assert_eq!(config.directories.contents_dir, "docs");
    assert_eq!(config.directories.public_dir, "static");
    assert_eq!(config.directories.theme_dir, "skins");
    assert_eq!(config.directories.out_dir, "build");

    assert_eq!(config.dev_server.port, 8080);
    assert_eq!(config.dev_server.host, "0.0.0.0");
}

#[test]
fn minimal_config_falls_back_to_all_defaults() {
    let config = parse(
        r#"
[site]
title = "Peisar"
"#,
    );

    assert_eq!(config.site.title, "Peisar");
    assert_eq!(config.site.description, None);
    assert_eq!(config.site.author, None);
    assert_eq!(config.directories, PeisarSSGDirectories::default());
    assert_eq!(config.dev_server, PeisarSSGDevServer::default());
}

#[test]
fn empty_optional_tables_fall_back_to_defaults() {
    let config = parse(
        r#"
[site]
title = "Peisar"

[directories]

[devServer]
"#,
    );
    assert_eq!(config.directories, PeisarSSGDirectories::default());
    assert_eq!(config.dev_server, PeisarSSGDevServer::default());
}

#[test]
fn directory_defaults_apply_per_missing_key() {
    let config = parse(
        r#"
[site]
title = "Peisar"

[directories]
rootDir = "site-root"
"#,
    );
    let dirs = config.directories;
    assert_eq!(dirs.root_dir, "site-root");
    assert_eq!(dirs.contents_dir, "contents");
    assert_eq!(dirs.public_dir, "public");
    assert_eq!(dirs.theme_dir, "themes");
    assert_eq!(dirs.out_dir, ".peisar");
}

#[test]
fn dev_server_defaults_apply_per_missing_key() {
    let port_only = parse(
        r#"
[site]
title = "x"

[devServer]
port = 9999
"#,
    );
    assert_eq!(port_only.dev_server.port, 9999);
    assert_eq!(port_only.dev_server.host, "127.0.0.1");

    let host_only = parse(
        r#"
[site]
title = "x"

[devServer]
host = "localhost"
"#,
    );
    assert_eq!(host_only.dev_server.port, 3838);
    assert_eq!(host_only.dev_server.host, "localhost");
}

#[test]
fn optional_site_fields_stay_optional() {
    let config = parse(
        r#"
[site]
title = "x"
description = "A site"
"#,
    );
    assert_eq!(config.site.description.as_deref(), Some("A site"));
    assert_eq!(config.site.author, None);
}

#[test]
fn empty_site_title_is_rejected() {
    assert!(matches!(
        parse_err(
            r#"
[site]
title = ""
"#,
        ),
        PeisarConfigError::MissingTitle
    ));
}

#[test]
fn missing_site_table_is_rejected() {
    assert!(matches!(
        parse_err(
            r#"
[directories]
rootDir = "."
"#,
        ),
        PeisarConfigError::Parse(_)
    ));
}

#[test]
fn missing_title_key_is_rejected() {
    assert!(matches!(
        parse_err(
            r#"
[site]
description = "no title"
"#,
        ),
        PeisarConfigError::Parse(_)
    ));
}

#[test]
fn invalid_toml_is_rejected() {
    assert!(matches!(parse_err("[site\n"), PeisarConfigError::Parse(_)));
}

#[test]
fn wrong_value_type_is_rejected() {
    assert!(matches!(
        parse_err(
            r#"
[site]
title = "x"

[devServer]
port = "not a number"
"#,
        ),
        PeisarConfigError::Parse(_)
    ));
}

#[test]
fn unknown_keys_are_ignored() {
    let config = parse(
        r#"
[site]
title = "x"
lang = "en"

[extra]
whatever = true
"#,
    );
    assert_eq!(config.site.title, "x");
    assert_eq!(config.directories, PeisarSSGDirectories::default());
    assert_eq!(config.dev_server, PeisarSSGDevServer::default());
}

#[test]
fn snake_case_keys_fall_back_to_defaults() {
    // Keys are matched case-sensitively in camelCase: `root_dir` and the
    // `[dev_server]` table below are unknown and therefore ignored.
    let config = parse(
        r#"
[site]
title = "x"

[directories]
root_dir = "/ignored"

[dev_server]
port = 7000
"#,
    );
    assert_eq!(config.directories.root_dir, ".");
    assert_eq!(config.dev_server.port, 3838);
}

#[test]
fn reads_config_file_from_disk() {
    let dir = TempDir::new("reads_config_file_from_disk");
    write_config(dir.path(), FULL_CONFIG);

    let content = read_toml_config_from(dir.path()).unwrap();
    assert_eq!(content, FULL_CONFIG);

    let config = parse_config(&content).unwrap();
    assert_eq!(config.site.title, "Peisar");
    assert_eq!(config.directories.out_dir, "build");
}

#[test]
fn missing_config_file_is_an_io_error() {
    let dir = TempDir::new("missing_config_file_is_an_io_error");

    let err = read_toml_config_from(dir.path()).unwrap_err();
    assert!(matches!(
        &err,
        PeisarConfigError::Io(io_err) if io_err.kind() == io::ErrorKind::NotFound
    ));
}

#[test]
fn config_path_that_is_a_directory_is_an_io_error() {
    let dir = TempDir::new("config_path_that_is_a_directory_is_an_io_error");
    std::fs::create_dir_all(dir.path().join(CONFIG_FILE_NAME)).unwrap();

    let err = read_toml_config_from(dir.path()).unwrap_err();
    assert!(matches!(
        &err,
        PeisarConfigError::Io(io_err) if io_err.kind() == io::ErrorKind::NotFound
    ));
}

#[test]
fn error_messages_match_the_cli_output() {
    assert_eq!(
        PeisarConfigError::MissingTitle.to_string(),
        "Site title required in config file."
    );
    assert_eq!(
        PeisarConfigError::Io(io::Error::new(io::ErrorKind::NotFound, "gone")).to_string(),
        "No config file found"
    );

    let parse_error = toml::from_str::<PeisarToml>("= nope").unwrap_err();
    let err = PeisarConfigError::Parse(parse_error);
    assert!(err.to_string().starts_with("Invalid config file:"));
}

#[test]
fn config_error_is_a_std_error_with_source() {
    let io_err = PeisarConfigError::Io(io::Error::new(io::ErrorKind::NotFound, "gone"));
    assert!(std::error::Error::source(&io_err).is_some());

    let parse_err = PeisarConfigError::Parse(toml::from_str::<PeisarToml>("= nope").unwrap_err());
    assert!(std::error::Error::source(&parse_err).is_some());

    assert!(std::error::Error::source(&PeisarConfigError::MissingTitle).is_none());
}
