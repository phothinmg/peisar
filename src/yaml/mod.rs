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

// ------------------------------------------------------------------------------//

#[cfg(test)]

mod tests {
    use super::{load_yaml, parse_yaml_config, yaml_parser};
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
}
