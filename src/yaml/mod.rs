//! YAML parsing helpers for Rust applications.
//!
//! The default feature set provides helpers for parsing YAML into either a
//! [`serde_json::Value`] or a caller-defined Serde type, loading YAML from a
//! file, and extracting YAML front matter from Markdown.
//!
//! Enable the `npm` feature to expose the parsing and front-matter functions
//! through N-API for Node.js consumers.

#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Parses YAML into a JSON-compatible dynamic value.
fn parse_yaml_value(yaml_str: &str) -> Result<Value, String> {
    noyalib::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML: {e}"))
}
/// Parses YAML into a caller-provided config type.
///
/// The target type must implement [`serde::Deserialize`]. Returns an error
/// containing the YAML parser's diagnostic when the input is invalid or does
/// not match the target type.
fn parse_yaml_config<T: DeserializeOwned + 'static>(yaml_str: &str) -> Result<T, String> {
    noyalib::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML config: {e}"))
}

/// Parses YAML into a JSON-compatible dynamic value.
///
/// YAML mappings, sequences, and scalar values are represented as a
/// [`serde_json::Value`]. Returns an error when the input cannot be parsed.
pub fn yaml_parser(yaml_str: &str) -> Result<Value, String> {
    parse_yaml_value(yaml_str).map_err(|e| format!("Failed to parse YAML: {e}"))
}

/// Reads a YAML file and deserializes it into the requested type.
///
/// Relative paths are resolved from the process's current working directory.
/// Returns an error if the file cannot be read or its contents cannot be
/// deserialized as `T`.
pub fn load_yaml<T: DeserializeOwned + 'static>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let yaml = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read YAML file {}: {e}", path.display()))?;
    parse_yaml_config(&yaml)
}

/// Parses a YAML document into a JSON-compatible value.
///
/// When consumed from Node.js, the result is exposed as a JavaScript object
/// with the TypeScript type `Record<string, any>`.
///
/// Returns an N-API error if the YAML input cannot be parsed.
#[cfg(feature = "npm")]
#[napi(
    js_name = "yamlParser",
    ts_return_type = "Record<string, any>",
    namespace = "yaml"
)]
pub fn yaml_parser_napi(yaml_str: String) -> Result<Value, napi::Error> {
    parse_yaml_value(&yaml_str).map_err(napi::Error::from_reason)
}

/// Reads and parses a YAML file relative to the process's current working
/// directory.
///
/// The parsed YAML value is returned to Node.js as a JavaScript value. File
/// read failures panic in this N-API wrapper; parse failures are returned as
/// N-API errors.
#[cfg(feature = "npm")]
#[napi(
    js_name = "loadYaml",
    ts_return_type = "Record<string, any>",
    namespace = "yaml"
)]
pub fn load_yaml_napi(file_path: String) -> Result<Value, napi::Error> {
    use std::env;
    let cwd = env::current_dir().unwrap_or(".".into());
    let path = cwd.join(file_path);
    let yaml = fs::read_to_string(path).unwrap();
    parse_yaml_value(&yaml).map_err(napi::Error::from_reason)
}

/// The Markdown body and YAML metadata parsed from a front-matter document.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterResult {
    /// Clean content after the leading YAML front matter is removed.
    pub clean_content: String,
    /// Deserialized YAML front matter, or `null` when no front matter exists.
    pub yaml_data: Option<Value>,
}
impl FrontmatterResult {
    /// Returns the deserialized YAML front matter, if one was present.
    pub fn yaml_data(&self) -> Option<&Value> {
        self.yaml_data.as_ref()
    }
    /// Returns the Markdown body with the YAML front matter removed.
    pub fn clean_content(&self) -> &str {
        &self.clean_content
    }

    /// Consume the FrontmatterResult and return the owned Markdown content and owned
    /// optional YAML front matter in one allocation. This is useful for callers
    /// that need to take ownership of the parsed frontmatter value.
    pub fn into_parts(self) -> (String, Option<Value>) {
        (self.clean_content, self.yaml_data)
    }
}
#[cfg(feature = "npm")]
#[napi(object, js_name = "FrontmatterResult", namespace = "yaml")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterResultNapi {
    /// Clean content after the leading YAML front matter is removed.
    pub clean_content: String,
    /// Deserialized YAML front matter, or `null` when no front matter exists.
    #[napi(ts_type = "Record<string, any>")]
    pub yaml_data: Option<Value>,
}
#[cfg(feature = "npm")]
impl FrontmatterResultNapi {
    /// Returns the deserialized YAML front matter, if one was present.
    pub fn yaml_data(&self) -> Option<&Value> {
        self.yaml_data.as_ref()
    }
    /// Returns the Markdown body with the YAML front matter removed.
    pub fn clean_content(&self) -> &str {
        &self.clean_content
    }

    /// Consume the FrontmatterResult and return the owned Markdown content and owned
    /// optional YAML front matter in one allocation. This is useful for callers
    /// that need to take ownership of the parsed frontmatter value.
    pub fn into_parts(self) -> (String, Option<Value>) {
        (self.clean_content, self.yaml_data)
    }
}
#[cfg(feature = "npm")]
fn parse_frontmatter(content: &str) -> Result<FrontmatterResultNapi, String> {
    let trimmed = content.trim_start();
    let has_frontmatter_start = trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n");
    // Only slice off the `---` opener when one is actually present; slicing
    // unconditionally would panic on inputs shorter than the opener itself.
    let rest = if has_frontmatter_start {
        let offset = if trimmed.starts_with("---\r\n") { 5 } else { 4 };
        &trimmed[offset..]
    } else {
        ""
    };
    let mut yaml_data: Option<Value> = None;
    let mut clean_content: String = content.to_string();
    if let Some(end_index) = rest.find("\n---") {
        let yaml_str = &rest[..end_index];
        let md_start_offset = end_index + 4;
        clean_content = rest[md_start_offset..].trim_start().to_string();

        yaml_data = Some(
            parse_yaml_value(yaml_str)
                .map_err(|e| format!("Failed to parse YAML front matter: {e}"))?,
        );
    }
    Ok(FrontmatterResultNapi {
        yaml_data,
        clean_content,
    })
}

#[cfg(feature = "npm")]
#[napi(js_name = "frontmatter", namespace = "yaml")]
/// Parses YAML front matter from Markdown content.
///
/// Returns an N-API error if the front matter is present but invalid.
pub fn frontmatter_napi(content: String) -> Result<FrontmatterResultNapi, napi::Error> {
    parse_frontmatter(&content).map_err(napi::Error::from_reason)
}

/// Parses YAML front matter from Markdown content.
///
/// If the document has no complete front-matter block, its content is
/// returned unchanged and [`FrontmatterResult::yaml_data`] is `None`.
/// Invalid YAML in a complete front-matter block returns an error.
pub fn frontmatter(content: String) -> Result<FrontmatterResult, String> {
    let trimmed = content.trim_start();
    let has_frontmatter_start = trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n");
    // Only slice off the `---` opener when one is actually present; slicing
    // unconditionally would panic on inputs shorter than the opener itself.
    let rest = if has_frontmatter_start {
        let offset = if trimmed.starts_with("---\r\n") { 5 } else { 4 };
        &trimmed[offset..]
    } else {
        ""
    };
    let mut yaml_data: Option<Value> = None;
    let mut clean_content: String = content.to_string();
    if let Some(end_index) = rest.find("\n---") {
        let yaml_str = &rest[..end_index];
        let md_start_offset = end_index + 4;
        clean_content = rest[md_start_offset..].trim_start().to_string();

        yaml_data = Some(
            parse_yaml_value(yaml_str)
                .map_err(|e| format!("Failed to parse YAML front matter: {e}"))?,
        );
    }
    Ok(FrontmatterResult {
        yaml_data,
        clean_content,
    })
}

// ------------------------------------------------------------------------------//

#[cfg(test)]

mod tests {
    use super::{FrontmatterResult, frontmatter, load_yaml, parse_yaml_config, yaml_parser};
    use serde::Deserialize;
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[derive(Debug, Deserialize, PartialEq)]
    struct AppConfig {
        host: String,
        port: u16,
        enabled: bool,
    }

    fn temp_yaml_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        env::temp_dir().join(format!("peisar-{name}-{nonce}.yaml"))
    }

    #[test]
    fn parses_dynamic_yaml_values() {
        let value = yaml_parser("title: Hello\nitems:\n  - a\n  - b\n").unwrap();
        assert_eq!(value["title"], "Hello");
        assert_eq!(value["items"].as_array().map(Vec::len), Some(2));
    }
    #[test]
    fn parses_typed_yaml_config() {
        let config: AppConfig =
            parse_yaml_config("host: localhost\nport: 3000\nenabled: true\n").unwrap();
        assert_eq!(
            config,
            AppConfig {
                host: "localhost".to_string(),
                port: 3000,
                enabled: true,
            }
        );
    }
    #[test]
    fn loads_yaml_file_as_value() {
        let path = temp_yaml_path("value");
        fs::write(&path, "name: peisar\nmode: dev\n").unwrap();

        let value: serde_json::Value = load_yaml(&path).unwrap();
        assert_eq!(value["name"], "peisar");

        fs::remove_file(path).unwrap();
    }
    #[test]
    fn loads_yaml_file_as_typed_config() {
        let path = temp_yaml_path("config");
        fs::write(&path, "host: 127.0.0.1\nport: 8080\nenabled: false\n").unwrap();

        let config: AppConfig = load_yaml(&path).unwrap();
        assert_eq!(config.port, 8080);
        assert!(!config.enabled);

        fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_yaml_returns_error() {
        let err = yaml_parser("{invalid: [").unwrap_err();
        assert!(err.contains("Failed to parse YAML"));
    }
    #[test]
    fn extracts_yaml_and_body() {
        let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Body\n".to_string()).unwrap();
        assert_eq!(parsed.clean_content(), "# Body\n");
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(yaml["title"].as_str(), Some("Hello"));
    }

    #[test]
    fn no_frontmatter_returns_whole_input() {
        let parsed = frontmatter("# Just markdown\n".to_string()).unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.clean_content(), "# Just markdown\n");
    }

    #[test]
    fn empty_input_is_valid() {
        let parsed = frontmatter("".to_string()).unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.clean_content(), "");
    }

    #[test]
    fn unterminated_frontmatter_is_treated_as_body() {
        // No closing `---`: nothing looks like front matter, so the input is
        // returned unchanged as body text.
        let parsed = frontmatter("---\ntitle: Hello\n".to_string()).unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.clean_content(), "---\ntitle: Hello\n");
    }

    #[test]
    fn crlf_line_endings_are_supported() {
        let parsed =
            frontmatter("---\r\ntitle: Hello\r\n---\r\n\r\n# Body\r\n".to_string()).unwrap();
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(yaml["title"].as_str(), Some("Hello"));
        assert!(parsed.clean_content().starts_with("# Body"));
    }

    #[test]
    fn nested_yaml_structures_deserialize() {
        let parsed =
            frontmatter("---\ntitle: T\ntags:\n  - a\n  - b\n---\nBody".to_string()).unwrap();
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(
            yaml["tags"].as_array().map(|s| s.len()).unwrap_or_default(),
            2
        );
    }

    #[test]
    fn invalid_yaml_is_an_error() {
        let err = frontmatter("---\n{invalid: [\n---\n".to_string()).unwrap_err();
        assert!(err.contains("Failed to parse YAML front matter"));
    }

    #[test]
    fn into_parts_yields_owned_values() {
        let parsed: FrontmatterResult = frontmatter("---\nx: 1\n---\n# Body".to_string()).unwrap();
        let (md, yaml) = parsed.into_parts();
        assert_eq!(md, "# Body");
        assert_eq!(yaml.unwrap()["x"], 1);
    }
}
