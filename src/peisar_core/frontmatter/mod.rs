#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "npm", napi(string_enum))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum FrontmatterType {
    /// YAML frontmatter delimited by `---` lines.
    Yaml,
    /// TOML frontmatter delimited by `+++` lines.
    Toml,
    /// JSON frontmatter represented by a top-level object.
    Json,
    /// No complete, supported frontmatter was found.
    Empty,
}

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
/// Frontmatter type and its contents, separated from the document body.
pub struct Frontmatter {
    /// Format detected in the document.
    #[cfg_attr(
        feature = "npm",
        napi(ts_type = "\"Yaml\" | \"Toml\" | \"Json\" | \"Empty\"")
    )]
    pub frontmatter_type: FrontmatterType,
    /// Frontmatter contents without delimiters, or an empty string if absent.
    pub raw: String,
}

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
/// The extracted frontmatter and the remaining document body.
pub struct FrontmatterResult {
    /// Detected frontmatter.
    pub frontmatter: Frontmatter,
    /// Document content after the frontmatter, or the original input if none was found.
    pub body: String,
}

/// Extracts YAML, TOML, or JSON frontmatter from the start of a Markdown document.
///
/// YAML and TOML use `---` and `+++` delimiter lines, respectively. JSON
/// frontmatter is recognized as a top-level object, optionally wrapped in
/// `;;;` delimiters. If no complete supported frontmatter is found, the result
/// has type [`FrontmatterType::Empty`] and contains the original input as its
/// body.
pub fn extract_frontmatter(markdown: String) -> FrontmatterResult {
    let fence = if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
        Some(("---", FrontmatterType::Yaml))
    } else if markdown.starts_with("+++\n") || markdown.starts_with("+++\r\n") {
        Some(("+++", FrontmatterType::Toml))
    } else {
        None
    };

    if let Some((delim, fm_type)) = fence {
        let after_open = markdown[3..]
            .strip_prefix("\r\n")
            .or_else(|| markdown[3..].strip_prefix('\n'))
            .unwrap_or(&markdown[3..]);

        let lf_close = format!("\n{}", delim);
        if let Some(pos) = after_open.find(&lf_close) {
            let raw = after_open[..pos]
                .strip_suffix('\r')
                .unwrap_or(&after_open[..pos])
                .to_string();
            let after_close = &after_open[pos + lf_close.len()..];

            if after_close.is_empty() {
                return FrontmatterResult {
                    frontmatter: Frontmatter {
                        frontmatter_type: fm_type,
                        raw,
                    },
                    body: "".to_string(),
                };
            } else if let Some(body) = after_close
                .strip_prefix("\r\n")
                .or_else(|| after_close.strip_prefix('\n'))
            {
                return FrontmatterResult {
                    frontmatter: Frontmatter {
                        frontmatter_type: fm_type,
                        raw,
                    },
                    body: body.to_string(),
                };
            }
        }
    }

    let (json, wrapped) = if let Some(json) = markdown.strip_prefix(";;;") {
        (
            json.strip_prefix("\r\n")
                .or_else(|| json.strip_prefix('\n'))
                .unwrap_or(json),
            true,
        )
    } else {
        (markdown.as_str(), false)
    };

    if json.starts_with('{') {
        if let Some(end_idx) = find_closing_brace(json) {
            let raw = &json[..=end_idx];
            let rest = &json[end_idx + 1..];
            let after_json = if wrapped {
                let rest = rest
                    .strip_prefix("\r\n")
                    .or_else(|| rest.strip_prefix('\n'))
                    .unwrap_or(rest);
                rest.strip_prefix(";;;")
            } else {
                Some(rest)
            };

            if let Some(rest) = after_json {
                let body = rest
                    .strip_prefix("\r\n")
                    .or_else(|| rest.strip_prefix('\n'))
                    .unwrap_or(rest)
                    .to_string();

                return FrontmatterResult {
                    frontmatter: Frontmatter {
                        frontmatter_type: FrontmatterType::Json,
                        raw: raw.to_string(),
                    },
                    body,
                };
            }
        }
    }
    FrontmatterResult {
        frontmatter: Frontmatter {
            frontmatter_type: FrontmatterType::Empty,
            raw: String::new(),
        },
        body: markdown,
    }
}

fn find_closing_brace(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{FrontmatterType, extract_frontmatter};

    #[test]
    fn extracts_yaml_frontmatter_and_body() {
        let parsed = extract_frontmatter("---\ntitle: Test\n---\n# Body".to_string());

        assert_eq!(parsed.frontmatter.frontmatter_type, FrontmatterType::Yaml);
        assert_eq!(parsed.frontmatter.raw, "title: Test");
        assert_eq!(parsed.body, "# Body");
    }

    #[test]
    fn extracts_json_with_semicolon_delimiters() {
        let parsed = extract_frontmatter(";;;{\"title\":\"Test\"};;;\n# Body".to_string());

        assert_eq!(parsed.frontmatter.frontmatter_type, FrontmatterType::Json);
        assert_eq!(parsed.frontmatter.raw, "{\"title\":\"Test\"}");
        assert_eq!(parsed.body, "# Body");
    }

    #[test]
    fn extracts_multiline_json_with_semicolon_delimiters() {
        let parsed = extract_frontmatter(";;;\n{\"title\":\"Test\"}\n;;;\n# Body".to_string());

        assert_eq!(parsed.frontmatter.frontmatter_type, FrontmatterType::Json);
        assert_eq!(parsed.frontmatter.raw, "{\"title\":\"Test\"}");
        assert_eq!(parsed.body, "# Body");
    }

    #[test]
    fn keeps_markdown_unchanged_without_complete_frontmatter() {
        let markdown = "---\ntitle: Test\n";
        let parsed = extract_frontmatter(markdown.to_string());

        assert_eq!(parsed.frontmatter.frontmatter_type, FrontmatterType::Empty);
        assert!(parsed.frontmatter.raw.is_empty());
        assert_eq!(parsed.body, markdown);
    }
}
