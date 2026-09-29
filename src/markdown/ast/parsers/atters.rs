use crate::markdown::ast::tokens::Attributes;

/// Parse a Kramdown attribute block starting at the opening `{`.
///
/// Recognises the Kramdown block-attribute syntax `{:#id .class key="val"}`
/// (the `:` after `{` is optional).  Nested braces and quoted values
/// containing `}` are handled.
///
/// Returns `(attrs, chars_consumed)` counting from the opening `{`, or `None`
/// when the input does not start with a valid attribute block.
///
/// # Example
///
/// ```ignore
/// // The attribute block in `{: #hero .card data-role="note"}` parses to:
/// assert_eq!(attrs.to_html_attr_string(),
///            "id=\"hero\" class=\"card\" data-role=\"note\"");
/// ```
pub fn parse_attrs(s: &str) -> Option<(Attributes, usize)> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }

    // Find the matching closing `}`.
    let chars: Vec<char> = s.chars().collect();
    let mut depth = 0i32;
    let mut end = 0usize;
    let mut in_string: Option<char> = None;
    for (i, &c) in chars.iter().enumerate() {
        match in_string {
            Some(q) => {
                if c == q {
                    in_string = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    in_string = Some(c);
                } else if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
            }
        }
    }

    if end == 0 {
        return None;
    }

    // Kramdown block attribute syntax is `{:...}` — skip the optional `:`
    // immediately after the opening `{`.
    let inner_start = if chars.get(1) == Some(&':') { 2 } else { 1 };
    let inner: String = chars[inner_start..end - 1].iter().collect();
    let attrs = parse_attr_tokens(&inner);
    Some((attrs, end))
}

/// Parse the tokens inside `{...}`.
fn parse_attr_tokens(inner: &str) -> Attributes {
    let mut attrs = Attributes::default();
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        match chars[i] {
            // ID: #id
            '#' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                if i > start {
                    attrs.id = Some(chars[start..i].iter().collect());
                }
            }
            // Class: .class
            '.' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                if i > start {
                    let class: String = chars[start..i].iter().collect();
                    match &mut attrs.classes {
                        Some(v) => v.push(class),
                        None => attrs.classes = Some(vec![class]),
                    }
                }
            }
            // key="value" or key='value'
            c if is_ident_start(c) => {
                let key_start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                let key: String = chars[key_start..i].iter().collect();

                // skip whitespace
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }

                if i < chars.len() && chars[i] == '=' {
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                        let q = chars[i];
                        i += 1;
                        let val_start = i;
                        while i < chars.len() && chars[i] != q {
                            i += 1;
                        }
                        let val: String = chars[val_start..i].iter().collect();
                        if i < chars.len() {
                            i += 1; // skip closing quote
                        }
                        push_attr(&mut attrs, key, val);
                    } else {
                        // unquoted value
                        let val_start = i;
                        while i < chars.len() && !chars[i].is_whitespace() {
                            i += 1;
                        }
                        let val: String = chars[val_start..i].iter().collect();
                        if !val.is_empty() {
                            push_attr(&mut attrs, key, val);
                        }
                    }
                } else {
                    // bare key → boolean attribute
                    push_attr(&mut attrs, key, String::new());
                }
            }
            _ => {
                i += 1; // skip unknown char
            }
        }
    }

    attrs
}

/// Push a key-value pair into `attrs.attributes`, initialising the `Vec` if
/// it is still `None`.
fn push_attr(attrs: &mut Attributes, key: String, val: String) {
    match &mut attrs.attributes {
        Some(v) => v.push((key, val)),
        None => attrs.attributes = Some(vec![(key, val)]),
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '-'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

#[cfg(test)]
mod tests {
    use super::parse_attrs;

    #[test]
    fn rejects_non_attribute_input() {
        assert!(parse_attrs("plain text").is_none());
        assert!(parse_attrs("").is_none());
        // Unclosed block.
        assert!(parse_attrs("{#id").is_none());
    }

    #[test]
    fn parses_id_class_and_key_values() {
        let (attrs, consumed) = parse_attrs("{:#hero .card data-role=\"note\"}").unwrap();
        assert_eq!(
            consumed,
            "{:#hero .card data-role=\"note\"}".chars().count()
        );
        assert_eq!(attrs.id.as_deref(), Some("hero"));
        assert_eq!(attrs.classes, Some(vec!["card".to_string()]));
        assert_eq!(
            attrs.attributes,
            Some(vec![("data-role".to_string(), "note".to_string())])
        );
    }

    #[test]
    fn colon_after_open_brace_is_optional() {
        // Kramdown writes `{: ...}` but `{ ...}` is accepted too.
        let (attrs, _) = parse_attrs("{#id}").unwrap();
        assert_eq!(attrs.id.as_deref(), Some("id"));
    }

    #[test]
    fn multiple_classes_accumulate() {
        let (attrs, _) = parse_attrs("{:.a .b .c}").unwrap();
        assert_eq!(
            attrs.classes,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn quoted_values_may_contain_braces() {
        let (attrs, _) = parse_attrs("{:data-json=\"{a:1}\"}").unwrap();
        assert_eq!(
            attrs.attributes,
            Some(vec![("data-json".to_string(), "{a:1}".to_string())])
        );
    }

    #[test]
    fn bare_keys_become_boolean_attributes() {
        let (attrs, _) = parse_attrs("{:disabled}").unwrap();
        assert_eq!(
            attrs.attributes,
            Some(vec![("disabled".to_string(), String::new())])
        );
    }

    #[test]
    fn unquoted_values_read_until_whitespace() {
        let (attrs, _) = parse_attrs("{:data-x=1 data-y=2}").unwrap();
        assert_eq!(
            attrs.attributes,
            Some(vec![
                ("data-x".to_string(), "1".to_string()),
                ("data-y".to_string(), "2".to_string()),
            ])
        );
    }
}
