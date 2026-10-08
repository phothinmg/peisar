//! YAML front-matter extraction for Markdown documents.
//!
//! Front matter is an optional YAML block delimited by `---` markers at the
//! very top of the document:
//!
//! ```markdown
//! ---
//! title: Hello
//! tags:
//!   - a
//!   - b
//! ---
//! # The Markdown body follows
//! ```
//!
//! Use [`frontmatter`] to split a document into its Markdown
//! body and the deserialized YAML metadata (a `serde_json::Value`, so the
//! result is directly usable from both Rust and JavaScript).
//!
//! # Example
//!
//! ```
//! use peisar::frontmatter::frontmatter;
//!
//! let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Body".to_string()).unwrap();
//! assert_eq!(parsed.pure_markdown_content(), "# Body");
//! assert_eq!(parsed.yaml_data().unwrap()["title"], "Hello");
//! ```
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The Markdown body and YAML metadata parsed from a front-matter document.
#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterResult {
    /// Markdown source after the leading YAML front matter is removed.
    pub pure_markdown_content: String,
    /// Deserialized YAML front matter, or `null` when no front matter exists.
    #[cfg_attr(feature = "npm", napi(ts_type = "Record<string, any>"))]
    pub yaml_data: Option<Value>,
}
#[cfg_attr(feature = "npm", napi)]
impl FrontmatterResult {
    /// Returns the deserialized YAML front matter, if one was present.
    pub fn yaml_data(&self) -> Option<&Value> {
        self.yaml_data.as_ref()
    }
    /// Returns the Markdown body with the YAML front matter removed.
    pub fn pure_markdown_content(&self) -> &str {
        &self.pure_markdown_content
    }

    /// Consume the FrontmatterResult and return the owned Markdown content and owned
    /// optional YAML front matter in one allocation. This is useful for callers
    /// that need to take ownership of the parsed frontmatter value.
    pub fn into_parts(self) -> (String, Option<Value>) {
        (self.pure_markdown_content, self.yaml_data)
    }
}

#[cfg(feature = "npm")]
/// Parses a YAML document into a JSON-compatible value.
///
/// When consumed from Node.js, the result is exposed as a JavaScript object
/// with the TypeScript type `Record<string, any>`.
///
/// # Panics
///
/// Panics when `yaml_str` is not valid YAML.
#[napi(ts_return_type = "Record<string, any>")]
pub fn yaml_parser(yaml_str: String) -> Value {
    serde_yaml::from_str(&yaml_str).unwrap()
}

/// Parses leading YAML front matter from Markdown source.
///
/// The returned [`FrontmatterResult`] preserves the Markdown body and, when present,
/// deserializes the YAML metadata into a JSON-compatible value.
///
/// # Behavior
///
/// - A front-matter block must start on the first non-whitespace line with
///   `---` (a `\r\n` line ending is supported) and be closed by a line
///   starting with `---`.
/// - When no front matter is present the full input is returned as the
///   Markdown body and `yaml_data` is `None`.
/// - Invalid YAML produces `Err` with a human-readable message.
///
fn parse_markdown_frontmatter(content: &str) -> Result<FrontmatterResult, String> {
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
    let mut pure_markdown_content: String = content.to_string();
    if let Some(end_index) = rest.find("\n---") {
        let yaml_str = &rest[..end_index];
        let md_start_offset = end_index + 4;
        pure_markdown_content = rest[md_start_offset..].trim_start().to_string();

        // Parse into the generic type T
        yaml_data = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse YAML front matter: {}", e))?;
    }
    Ok(FrontmatterResult {
        yaml_data,
        pure_markdown_content,
    })
}

#[cfg(feature = "npm")]
#[napi]
pub fn frontmatter(content: String) -> Result<FrontmatterResult, napi::Error> {
    parse_markdown_frontmatter(&content).map_err(napi::Error::from_reason)
}

#[cfg(not(feature = "npm"))]
pub fn frontmatter(content: String) -> Result<FrontmatterResult, String> {
    parse_markdown_frontmatter(&content)
}

#[cfg(test)]
mod tests {
    use super::{FrontmatterResult, parse_markdown_frontmatter};

    #[test]
    fn extracts_yaml_and_body() {
        let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Body\n").unwrap();
        assert_eq!(parsed.pure_markdown_content(), "# Body\n");
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(yaml["title"].as_str(), Some("Hello"));
    }

    #[test]
    fn no_frontmatter_returns_whole_input() {
        let parsed = parse_markdown_frontmatter("# Just markdown\n").unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.pure_markdown_content(), "# Just markdown\n");
    }

    #[test]
    fn empty_input_is_valid() {
        let parsed = parse_markdown_frontmatter("").unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.pure_markdown_content(), "");
    }

    #[test]
    fn unterminated_frontmatter_is_treated_as_body() {
        // No closing `---`: nothing looks like front matter, so the input is
        // returned unchanged as body text.
        let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n").unwrap();
        assert!(parsed.yaml_data().is_none());
        assert_eq!(parsed.pure_markdown_content(), "---\ntitle: Hello\n");
    }

    #[test]
    fn crlf_line_endings_are_supported() {
        let parsed =
            parse_markdown_frontmatter("---\r\ntitle: Hello\r\n---\r\n\r\n# Body\r\n").unwrap();
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(yaml["title"].as_str(), Some("Hello"));
        assert!(parsed.pure_markdown_content().starts_with("# Body"));
    }

    #[test]
    fn nested_yaml_structures_deserialize() {
        let parsed =
            parse_markdown_frontmatter("---\ntitle: T\ntags:\n  - a\n  - b\n---\nBody").unwrap();
        let yaml = parsed.yaml_data().unwrap();
        assert_eq!(
            yaml["tags"].as_array().map(|s| s.len()).unwrap_or_default(),
            2
        );
    }

    #[test]
    fn invalid_yaml_is_an_error() {
        let err = parse_markdown_frontmatter("---\n{invalid: [\n---\n").unwrap_err();
        assert!(err.contains("Failed to parse YAML front matter"));
    }

    #[test]
    fn into_parts_yields_owned_values() {
        let parsed: FrontmatterResult =
            parse_markdown_frontmatter("---\nx: 1\n---\n# Body").unwrap();
        let (md, yaml) = parsed.into_parts();
        assert_eq!(md, "# Body");
        assert_eq!(yaml.unwrap()["x"], 1);
    }
}
