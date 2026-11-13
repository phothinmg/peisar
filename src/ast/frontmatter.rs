#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "npm", napi(skip_typescript))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum FrontmatterType {
    Yaml,
    Toml,
    Json,
    Empty,
}

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]

pub struct Frontmatter {
    #[cfg_attr(
        feature = "npm",
        napi(ts_type = "\"yaml\" | \"toml\" | \"json\" | \"empty\"")
    )]
    pub frontmatter_type: FrontmatterType,
    pub raw: String,
}

#[cfg_attr(feature = "npm", napi(object))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FrontmatterResult {
    pub frontmatter: Frontmatter,
    pub body: String,
}

#[cfg_attr(feature = "npm", napi)]
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

    if markdown.starts_with('{') {
        if let Some(end_idx) = find_closing_brace(&markdown) {
            let raw = &markdown[..=end_idx];
            let rest = &markdown[end_idx + 1..];
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
    fn keeps_markdown_unchanged_without_complete_frontmatter() {
        let markdown = "---\ntitle: Test\n";
        let parsed = extract_frontmatter(markdown.to_string());

        assert_eq!(parsed.frontmatter.frontmatter_type, FrontmatterType::Empty);
        assert!(parsed.frontmatter.raw.is_empty());
        assert_eq!(parsed.body, markdown);
    }
}
