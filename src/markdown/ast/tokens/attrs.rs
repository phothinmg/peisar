//! Kramdown block attribute parsing and representation.
//!
//! Kramdown-style attributes use the syntax `{:#id .class key="val"}` and
//! can appear on the line following a block element.  The [`Attributes`]
//! struct stores the parsed `id`, `classes`, and arbitrary key/value pairs
//! and can render them as an HTML attribute string via
//! [`Attributes::to_html_attr_string`].

use napi_derive::napi;
use serde::Serialize;

/// Parsed Kramdown block attributes (`{:#id .class key="val"}`).
///

#[napi(object)]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Attributes {
    /// HTML `id` attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// CSS class list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// Arbitrary key/value attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<(String, String)>>,
}

impl Attributes {
    /// Render the attributes as a space-delimited HTML attribute string.
    ///
    /// Special characters (`&`, `"`, `<`, `>`) are escaped.
    pub fn to_html_attr_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref id) = self.id {
            parts.push(format!("id=\"{}\"", escape_attr(id)));
        }
        if let Some(ref classes) = self.classes {
            parts.push(format!("class=\"{}\"", escape_attr(&classes.join(" "))));
        }
        if let Some(ref attrs) = self.attributes {
            for (k, v) in attrs {
                parts.push(format!("{}=\"{}\"", escape_attr(k), escape_attr(v)));
            }
        }
        parts.join(" ")
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
