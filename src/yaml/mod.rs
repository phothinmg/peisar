//! YAML parsing helpers for config files and dynamic values.
//!
//! The functions in this module use `noyalib`, a maintained YAML 1.2 parser
//! with Serde integration, so the same parser can back both front matter and
//! standalone YAML configuration files.

#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::Path};

/// Parses YAML into a JSON-compatible dynamic value.
pub fn parse_yaml_value(yaml_str: &str) -> Result<Value, String> {
    noyalib::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML: {e}"))
}

#[cfg(feature = "npm")]
/// Parses a YAML document into a JSON-compatible value.
///
/// When consumed from Node.js, the result is exposed as a JavaScript object
/// with the TypeScript type `Record<string, any>`.
#[napi(ts_return_type = "Record<string, any>")]
pub fn yaml_parser(yaml_str: String) -> Result<Value, napi::Error> {
    parse_yaml_value(&yaml_str).map_err(napi::Error::from_reason)
}

/// Loads a YAML file and parses it into a JSON-compatible dynamic value.
pub fn load_yaml_value_file(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let yaml = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read YAML file {}: {e}", path.display()))?;
    parse_yaml_value(&yaml)
}

/// Parses YAML into a caller-provided config type.
pub fn parse_yaml_config<T: DeserializeOwned + 'static>(yaml_str: &str) -> Result<T, String> {
    noyalib::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML config: {e}"))
}

/// Loads a YAML config file and deserializes it into the requested type.
pub fn load_yaml_config_file<T: DeserializeOwned + 'static>(
    path: impl AsRef<Path>,
) -> Result<T, String> {
    let path = path.as_ref();
    let yaml = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read YAML file {}: {e}", path.display()))?;
    parse_yaml_config(&yaml)
}

#[cfg(feature = "npm")]
#[napi(ts_return_type = "Record<string, any>")]
pub fn load_yaml(file_path: String) -> Result<Value, napi::Error> {
    use std::env;
    let cwd = env::current_dir().unwrap_or(".".into());
    let path = cwd.join(file_path);
    let yaml = fs::read_to_string(path).unwrap();
    parse_yaml_value(&yaml).map_err(napi::Error::from_reason)
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde::Deserialize;

    use super::{load_yaml_config_file, load_yaml_value_file, parse_yaml_config, parse_yaml_value};

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
        let value = parse_yaml_value("title: Hello\nitems:\n  - a\n  - b\n").unwrap();
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

        let value = load_yaml_value_file(&path).unwrap();
        assert_eq!(value["name"], "peisar");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn loads_yaml_file_as_typed_config() {
        let path = temp_yaml_path("config");
        fs::write(&path, "host: 127.0.0.1\nport: 8080\nenabled: false\n").unwrap();

        let config: AppConfig = load_yaml_config_file(&path).unwrap();
        assert_eq!(config.port, 8080);
        assert!(!config.enabled);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let err = parse_yaml_value("{invalid: [").unwrap_err();
        assert!(err.contains("Failed to parse YAML"));
    }
}
