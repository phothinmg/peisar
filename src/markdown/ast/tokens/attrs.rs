//! Kramdown block attribute parsing and representation.
//!
//! Kramdown-style attributes use the syntax `{:#id .class key="val"}` and
//! can appear on the line following a block element.  The [`Attributes`]
//! struct stores the parsed `id`, `classes`, and arbitrary key/value pairs
//! and can render them as an HTML attribute string via
//! [`Attributes::to_html_attr_string`].
#[cfg(feature = "npm")]
use napi_derive::napi;
use serde::Serialize;

/// Parsed Kramdown block attributes (`{:#id .class key="val"}`).
///

#[cfg_attr(feature = "npm", napi(object))]
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

#[cfg(test)]
mod tests {
    use super::Attributes;

    #[test]
    fn default_is_empty_string() {
        assert_eq!(Attributes::default().to_html_attr_string(), "");
    }

    #[test]
    fn id_renders_first() {
        let attrs = Attributes {
            id: Some("intro".into()),
            classes: None,
            attributes: None,
        };
        assert_eq!(attrs.to_html_attr_string(), "id=\"intro\"");
    }

    #[test]
    fn classes_join_with_spaces() {
        let attrs = Attributes {
            id: None,
            classes: Some(vec!["a".into(), "b".into()]),
            attributes: None,
        };
        assert_eq!(attrs.to_html_attr_string(), "class=\"a b\"");
    }

    #[test]
    fn key_value_pairs_render_after_id_and_class() {
        let attrs = Attributes {
            id: Some("x".into()),
            classes: Some(vec!["y".into()]),
            attributes: Some(vec![("data-role".into(), "note".into())]),
        };
        assert_eq!(
            attrs.to_html_attr_string(),
            "id=\"x\" class=\"y\" data-role=\"note\""
        );
    }

    #[test]
    fn special_characters_are_escaped() {
        let attrs = Attributes {
            id: Some("a<b".into()),
            classes: Some(vec!["c&d".into()]),
            attributes: Some(vec![("k".into(), "v\"1".into())]),
        };
        assert_eq!(
            attrs.to_html_attr_string(),
            "id=\"a&lt;b\" class=\"c&amp;d\" k=\"v&quot;1\""
        );
    }
}
