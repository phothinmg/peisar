use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[napi(object)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub pure_markdown_content: String,
    #[napi(ts_type = "Record<string, any>")]
    pub yaml_data: Option<Value>,
}
#[napi]
impl ParseResult {
    /// Returns the deserialized YAML front matter, if one was present.
    pub fn yaml_data(&self) -> Option<&Value> {
        self.yaml_data.as_ref()
    }
    /// Returns the Markdown body with the YAML front matter removed.
    pub fn pure_markdown_content(&self) -> &str {
        &self.pure_markdown_content
    }

    /// Consume the ParseResult and return the owned Markdown content and owned
    /// optional YAML front matter in one allocation. This is useful for callers
    /// that need to take ownership of the parsed frontmatter value.
    pub fn into_parts(self) -> (String, Option<Value>) {
        (self.pure_markdown_content, self.yaml_data)
    }
}

pub fn parse_markdown_frontmatter(content: &str) -> Result<ParseResult, String> {
    let trimmed = content.trim_start();
    let has_frontmatter_start = trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n");
    let offset = if trimmed.starts_with("---\r\n") { 5 } else { 4 };
    let rest = &trimmed[offset..];
    let mut yaml_data: Option<Value> = None;
    let mut pure_markdown_content: String = content.to_string();
    if has_frontmatter_start {
        if let Some(end_index) = rest.find("\n---") {
            let yaml_str = &rest[..end_index];
            let md_start_offset = end_index + 4;
            pure_markdown_content = rest[md_start_offset..].trim_start().to_string();

            // Parse into the generic type T
            yaml_data = serde_yaml::from_str(yaml_str)
                .map_err(|e| format!("Failed to parse YAML front matter: {}", e))?;
        }
    }
    Ok(ParseResult {
        yaml_data,
        pure_markdown_content,
    })
}
