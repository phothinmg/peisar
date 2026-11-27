//! Inline-level Markdown parser.
//!
//! This module parses inline Markdown constructs within a paragraph, heading,
//! or table cell into a list of [`Inline`] nodes.  Supported constructs:
//!
//! - Plain text
//! - Emphasis (`*italic*`, `**bold**`, `_italic_`, `__bold__`)
//! - Inline code (`` `code` ``)
//! - Inline HTML
//! - GFM strikethrough (`~~text~~`)
//! - Images (`![alt](url)`)
//! - Inline links (`[text](url)`)
//! - Reference-style links (`[text][label]`, `[label][]`, `[label]`)
//! - GFM autolink (bare URLs)
//! - Hard and soft line breaks
//!
//! ## Link reference resolution
//!
//! Reference-style links are resolved against a [`LinkRefMap`] that is
//! collected in a pre-pass by [`md_to_ast`](super::md_to_ast).  Use
//! [`parse_inline_with_refs`] when you have a ref map, or [`parse_inline`]
//! when you do not need reference resolution.

use super::plugin::{InlineParserContext, ParserHooks, finalize_hook_inline};
use crate::ast::{
    AstOptions,
    span::{Position, Span},
    tokens::{EmphasisLevel, Inline},
};

use std::collections::HashMap;

/// A map of normalise link reference labels to `(url, title)`.
pub type LinkRefMap = HashMap<String, (String, Option<String>)>;

/// Parse inline markdown into a list of inline nodes, with the given
/// options (controls GFM features).
pub fn parse_inline(input: &str, options: Option<&AstOptions>) -> Vec<Inline> {
    parse_inline_with_refs(input, options, None)
}

/// Parse inline markdown with an optional link reference map for resolving
/// reference-style links (`[text][label]`, `[label][]`, `[label]`).
pub fn parse_inline_with_refs(
    input: &str,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
) -> Vec<Inline> {
    parse_inline_with_hooks(input, options, refs, &ParserHooks::empty())
}

/// Parse inline markdown with custom parser hooks (see
/// [`ParserHooks`]).
///
/// Hooks run before the built-in inline matchers at every character
/// position; the first hook to claim a position wins.
pub fn parse_inline_with_hooks(
    input: &str,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
    hooks: &ParserHooks,
) -> Vec<Inline> {
    let binding = AstOptions::default();
    let opts = options.unwrap_or(&binding);
    let mut tokens: Vec<Inline> = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let ctx = InlineCtx::new(input, &chars);
    let mut i = 0;
    let mut text = String::new();
    let mut text_start = 0usize;

    while i < chars.len() {
        let c = chars[i];

        // Custom parser hooks — tried before every built-in matcher so an
        // extension can both introduce new syntax and override built-ins.
        if !hooks.is_empty() {
            let rest: String = chars[i..].iter().collect();
            let hctx = InlineParserContext {
                rest,
                index: i as u32,
            };
            if let Some((node, consumed)) = hooks.try_parse_inline(&hctx) {
                if consumed > 0 {
                    flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                    let end = (i + consumed).min(chars.len());
                    let pos =
                        Span::new(ctx.position(i, chars.len()), ctx.position(end, chars.len()));
                    tokens.push(finalize_hook_inline(node, pos));
                    i = end;
                    text_start = i;
                    continue;
                }
            }
        }

        // CommonMark backslash escapes for ASCII punctuation.
        if c == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
            flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
            tokens.push(Inline::Text {
                value: chars[i + 1].to_string(),
                pos: Span::new(
                    ctx.position(i, chars.len()),
                    ctx.position(i + 2, chars.len()),
                ),
            });
            i += 2;
            text_start = i;
            continue;
        }

        if c == '&'
            && let Some((decoded, end)) = match_char_reference(&chars, i)
        {
            text.push_str(&decoded);
            i = end;
            continue;
        }

        // Inline code: `code` or ``code``
        if c == '`' {
            let tick_count = chars[i..].iter().take_while(|&&ch| ch == '`').count();
            if let Some((code, end)) = match_inline_code(&chars, i, tick_count) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::Code {
                    code,
                    pos: Span::new(ctx.position(i, chars.len()), ctx.position(end, chars.len())),
                });
                i = end;
                text_start = i;
                continue;
            }
            text.extend(chars[i..i + tick_count].iter());
            i += tick_count;
            continue;
        }

        // GFM strikethrough: ~~text~~
        if opts.gfm && c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
            if let Some((node, end)) = match_strikethrough(&chars, i, &ctx, options, hooks) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(node);
                i = end;
                text_start = i;
                continue;
            }
        }

        // Image: ![alt](url)
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some((img, end)) = match_image(&chars, i, &ctx, options, refs, hooks) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(img);
                i = end;
                text_start = i;
                continue;
            }
        }

        // Link: [text](url) or reference link [text][label], [label][], [label]
        if c == '[' {
            // First try inline link `[text](url)`
            if let Some((link, end)) = match_link(&chars, i, &ctx, options, refs, hooks) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(link);
                i = end;
                text_start = i;
                continue;
            }
            // Then try reference link `[text][label]`, `[label][]`, `[label]`
            if let Some((link, end)) = match_reference_link(&chars, i, &ctx, options, refs, hooks) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(link);
                i = end;
                text_start = i;
                continue;
            }
        }

        // GFM autolink: bare URLs
        if opts.gfm && (c == 'h' || c == 'w') {
            if let Some((url, end)) = match_autolink(&chars, i) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::Link {
                    text: vec![Inline::Text {
                        value: url.clone(),
                        pos: Span::new(
                            ctx.position(i, chars.len()),
                            ctx.position(end, chars.len()),
                        ),
                    }],
                    url,
                    title: None,
                    autolink: true,
                    pos: Span::new(ctx.position(i, chars.len()), ctx.position(end, chars.len())),
                });
                i = end;
                text_start = i;
                continue;
            }
        }

        // Emphasis: **bold** / *italic* / __bold__ / _italic_
        if c == '*' || c == '_' {
            if let Some((node, end)) = match_emphasis(&chars, i, c, &ctx, options, refs, hooks) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(node);
                i = end;
                text_start = i;
                continue;
            }
            let run = chars[i..].iter().take_while(|&&ch| ch == c).count();
            if run > 1 {
                if c == '_' && !delimiter_can_open(&chars, i, run, c) {
                    text.extend(chars[i..i + run].iter());
                    i += run;
                    continue;
                }
                text.push(c);
                i += 1;
                continue;
            }
        }

        // Hard break: two trailing spaces + newline, or backslash + newline
        if c == '\n' {
            let trailing_spaces = text.chars().rev().take_while(|&ch| ch == ' ').count();
            if trailing_spaces >= 2 {
                text.truncate(text.len() - trailing_spaces);
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::HardBreak {
                    pos: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(i + 1, chars.len()),
                    ),
                });
                i += 1;
                text_start = i;
                continue;
            }
            if text.ends_with('\\') {
                text.truncate(text.len() - 1);
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::HardBreak {
                    pos: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(i + 1, chars.len()),
                    ),
                });
                i += 1;
                text_start = i;
                continue;
            }
            if trailing_spaces > 0 {
                text.truncate(text.len() - trailing_spaces);
            }
            flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
            tokens.push(Inline::SoftBreak {
                pos: Span::new(
                    ctx.position(i, chars.len()),
                    ctx.position(i + 1, chars.len()),
                ),
            });
            i += 1;
            text_start = i;
            continue;
        }

        // Autolinks must be recognized before the raw HTML matcher.
        if c == '<' {
            if let Some((url, text_value, end)) = match_angle_autolink(&chars, i) {
                let trailing_spaces = text.chars().rev().take_while(|&ch| ch == ' ').count();
                text.truncate(text.len() - trailing_spaces);
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                let pos = Span::new(ctx.position(i, chars.len()), ctx.position(end, chars.len()));
                tokens.push(Inline::Link {
                    text: vec![Inline::Text {
                        value: text_value,
                        pos,
                    }],
                    url,
                    title: None,
                    autolink: true,
                    pos,
                });
                i = end;
                text_start = i;
                continue;
            }

            if let Some(end) = match_inline_html(&chars, i) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                let html: String = chars[i..end].iter().collect();
                tokens.push(Inline::HtmlInline {
                    html,
                    pos: Span::new(ctx.position(i, chars.len()), ctx.position(end, chars.len())),
                });
                i = end;
                text_start = i;
                continue;
            }
        }

        text.push(c);
        i += 1;
    }

    let trailing_spaces = text.chars().rev().take_while(|&ch| ch == ' ').count();
    text.truncate(text.len() - trailing_spaces);
    flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
    tokens
}

/// Internal context for mapping character indices to source [`Position`]s.
///
/// Maintains precomputed line-start char indices and char-to-byte offset
/// tables so that [`InlineCtx::position`] runs in O(log n) time.
pub struct InlineCtx {
    /// `line_starts_char[l]` = char index at which line `l` begins.
    line_starts_char: Vec<usize>,
    /// `char_byte_offsets[i]` = byte offset at character index `i`,
    /// including the end-of-input position.
    char_byte_offsets: Vec<usize>,
}

impl InlineCtx {
    fn new(_input: &str, chars: &[char]) -> Self {
        let mut line_starts_char = vec![0usize];
        let mut char_byte_offsets = Vec::with_capacity(chars.len() + 1);
        let mut byte = 0usize;
        for c in chars {
            char_byte_offsets.push(byte);
            if *c == '\n' {
                line_starts_char.push(char_byte_offsets.len());
            }
            byte += c.len_utf8();
        }
        char_byte_offsets.push(byte);
        Self {
            line_starts_char,
            char_byte_offsets,
        }
    }

    /// Position at the given char index.
    pub fn position(&self, i: usize, chars_len: usize) -> Position {
        let i = i.min(chars_len);
        let line = match self.line_starts_char.binary_search(&i) {
            Ok(l) => l,
            Err(l) => l.saturating_sub(1),
        };
        let line_start = self.line_starts_char[line];
        let column = i - line_start;
        let offset = self.char_byte_offsets[i];
        Position {
            line: line as u32,
            column: column as u32,
            offset: offset as u32,
        }
    }
}

fn is_ascii_punctuation(ch: char) -> bool {
    ch.is_ascii_punctuation()
}

fn match_char_reference(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start)? != &'&' {
        return None;
    }

    let end = (start + 1..chars.len()).find(|&i| chars[i] == ';')?;
    let body = &chars[start + 1..end];
    if body.is_empty() || !body.iter().all(|c| c.is_ascii_alphanumeric() || *c == '#') {
        return None;
    }

    let numeric = if body.first() == Some(&'#') {
        let value = &body[1..];
        let (radix, digits) = if value.first().is_some_and(|c| *c == 'x' || *c == 'X') {
            (16, &value[1..])
        } else {
            (10, value)
        };
        let max_digits = if radix == 16 { 6 } else { 7 };
        if digits.is_empty()
            || digits.len() > max_digits
            || !digits.iter().all(|c| {
                if radix == 16 {
                    c.is_ascii_hexdigit()
                } else {
                    c.is_ascii_digit()
                }
            })
        {
            return None;
        }
        Some((radix, digits))
    } else {
        None
    };

    let reference: String = chars[start..=end].iter().collect();
    let decoded = html_escape::decode_html_entities(&reference).into_owned();
    if decoded != reference {
        return Some((decoded, end + 1));
    }

    let (radix, digits) = numeric?;
    let digits: String = digits.iter().collect();
    let value = u32::from_str_radix(&digits, radix).unwrap_or(0x11_0000);
    let scalar = if value == 0 {
        '\u{FFFD}'
    } else {
        char::from_u32(value).unwrap_or('\u{FFFD}')
    };
    Some((scalar.to_string(), end + 1))
}

pub(super) fn decode_char_references(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut decoded = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&'
            && let Some((value, end)) = match_char_reference(&chars, i)
        {
            decoded.push_str(&value);
            i = end;
        } else {
            decoded.push(chars[i]);
            i += 1;
        }
    }
    decoded
}

pub(super) fn unescape_backslash_punctuation(input: &str) -> String {
    let mut chars = input.chars();
    let mut output = String::new();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(next) = chars.next()
        {
            if is_ascii_punctuation(next) {
                output.push(next);
            } else {
                output.push(c);
                output.push(next);
            }
        } else {
            output.push(c);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::parse_inline_with_refs;
    use crate::peisar_core::tokens::token::Inline;

    #[test]
    fn inline_end_offsets_count_utf8_bytes() {
        for (input, expected_offset, expected_column) in [("é", 2, 1), ("aé", 3, 2)] {
            let nodes = parse_inline_with_refs(input, None, None);
            let Inline::Text { pos, .. } = nodes.last().unwrap() else {
                panic!("expected a text node");
            };
            assert_eq!(pos.end.offset, expected_offset);
            assert_eq!(pos.end.column, expected_column);
        }
    }

    #[test]
    fn backslash_escaped_punctuation_is_text() {
        let nodes = parse_inline_with_refs(r"\|", None, None);
        assert!(matches!(
            nodes.as_slice(),
            [Inline::Text { value, pos }]
                if value == "|" && pos.start.offset == 0 && pos.end.offset == 2
        ));
    }
}
// ---------------------------------------------------------------------------
// GFM - Inline Parsers
// ---------------------------------------------------------------------------
/// Match GFM strikethrough `~~text~~`. Returns `(children, end_index)`.
fn match_strikethrough(
    chars: &[char],
    start: usize,
    ctx: &InlineCtx,
    options: Option<&AstOptions>,
    hooks: &ParserHooks,
) -> Option<(Inline, usize)> {
    if chars[start] != '~' || start + 1 >= chars.len() || chars[start + 1] != '~' {
        return None;
    }

    let after = &chars[start + 2..];
    let after_str: String = after.iter().collect();
    let close = after_str.find("~~")?;
    let inner: String = after_str[..close].to_string();
    let children = parse_inline_with_hooks(&inner, options, None, hooks);
    let end = start + 2 + close + 2;

    Some((
        Inline::Strikethrough {
            children,
            pos: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(end, chars.len()),
            ),
        },
        end,
    ))
}
fn find_url_end(chars: &[char], from: usize) -> usize {
    let mut i = from;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == '<' || c == '>' {
            break;
        }
        // Trailing punctuation: . , ; : ! ? ) — but not if part of URL path
        if (c == '.' || c == ',' || c == ';' || c == ':' || c == '!' || c == '?' || c == ')')
            && (i + 1 >= chars.len() || chars[i + 1].is_whitespace())
        {
            break;
        }
        i += 1;
    }
    i
}
/// Detect a bare URL for GFM autolink (www.example.com or https://example.com).
/// Returns `(url, end_index)` relative to `chars`.
fn match_autolink(chars: &[char], start: usize) -> Option<(String, usize)> {
    // Check for https:// or http://
    let protocols: [&[char]; 2] = [
        &['h', 't', 't', 'p', 's', ':', '/', '/'],
        &['h', 't', 't', 'p', ':', '/', '/'],
    ];

    for proto in protocols {
        if start + proto.len() <= chars.len() && chars[start..start + proto.len()] == *proto {
            let end = find_url_end(chars, start + proto.len());
            let url: String = chars[start..end].iter().collect();
            return Some((url, end));
        }
    }

    // Check for www.
    let www: [char; 4] = ['w', 'w', 'w', '.'];
    if start + www.len() <= chars.len() && &chars[start..start + www.len()] == &www {
        let end = find_url_end(chars, start + www.len());
        let url: String = chars[start..end].iter().collect();
        // Prepend https:// for www.
        return Some((format!("https://{url}"), end));
    }

    None
}

/// get text node
fn flush_text(
    tokens: &mut Vec<Inline>,
    text: &mut String,
    text_start: &mut usize,
    ctx: &InlineCtx,
    chars: &[char],
) {
    if !text.is_empty() {
        let start = *text_start;
        let end = start + text.chars().count();
        let pos = Span::new(
            ctx.position(start, chars.len()),
            ctx.position(end, chars.len()),
        );
        let value = std::mem::take(text);
        tokens.push(Inline::Text { value, pos })
    }
}

fn match_inline_code(chars: &[char], start: usize, tick_count: usize) -> Option<(String, usize)> {
    let mut i = start + tick_count;
    let mut close = None;
    while i < chars.len() {
        if chars[i] != '`' {
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|&&ch| ch == '`').count();
        if run == tick_count {
            close = Some((i, i + run));
            break;
        }
        i += run;
    }
    let (close_start, end) = close?;
    let mut code: String = chars[start + tick_count..close_start].iter().collect();
    code = code.replace('\n', " ");
    if code.starts_with(' ') && code.ends_with(' ') && code.chars().any(|c| c != ' ') {
        code.remove(0);
        code.pop();
    }
    Some((code, end))
}

fn match_angle_autolink(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close = (start + 1..chars.len()).find(|&i| chars[i] == '>')?;
    let inner: String = chars[start + 1..close].iter().collect();
    if inner.is_empty()
        || inner
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '<' || c == '>')
    {
        return None;
    }

    if let Some(colon) = inner.find(':') {
        let scheme = &inner[..colon];
        if scheme.len() < 2
            || scheme.len() > 32
            || !scheme.as_bytes()[0].is_ascii_alphabetic()
            || !scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'))
            || inner[colon + 1..]
                .bytes()
                .any(|b| !(0x21..=0x7e).contains(&b) || matches!(b, b'<' | b'>'))
        {
            return None;
        }
        return Some((inner.clone(), inner, close + 1));
    }

    if valid_email_autolink(&inner) {
        return Some((format!("mailto:{inner}"), inner, close + 1));
    }
    None
}

fn valid_email_autolink(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !local.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'.' | b'!'
                        | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'/'
                        | b'='
                        | b'?'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'{'
                        | b'|'
                        | b'}'
                        | b'~'
                        | b'-'
                )
        })
    {
        return false;
    }
    domain.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && label
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric())
            && label
                .bytes()
                .next_back()
                .is_some_and(|b| b.is_ascii_alphanumeric())
    })
}

/// Match a reference-style link starting at `chars[start]` (which must be `[`).
/// Supports full: `[text][label]`, collapsed: `[label][]`, and shortcut: `[label]`.
fn match_reference_link(
    chars: &[char],
    start: usize,
    ctx: &InlineCtx,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
    hooks: &ParserHooks,
) -> Option<(Inline, usize)> {
    let (text, after_first) = match_bracket(chars, start, '[')?;
    let (normalized, url, title, end) = match_reference_target(chars, after_first, &text, refs)?;
    let children = parse_inline_with_hooks(&text, options, refs, hooks);
    if contains_link(&children) {
        return None;
    }
    Some((
        Inline::LinkReference {
            text: children,
            label: normalized,
            url: url.clone(),
            title: title.clone(),
            pos: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(end, chars.len()),
            ),
        },
        end,
    ))
}

fn match_reference_target<'a>(
    chars: &[char],
    after_first: usize,
    text: &str,
    refs: Option<&'a LinkRefMap>,
) -> Option<(String, &'a String, &'a Option<String>, usize)> {
    let refs = refs?;
    let (label, end) = if chars.get(after_first) == Some(&'[') {
        let (label_text, after_label) = match_bracket(chars, after_first, '[')?;
        let label = if label_text.trim().is_empty() {
            text.to_string()
        } else {
            label_text
        };
        (label, after_label)
    } else {
        (text.to_string(), after_first)
    };
    let normalized = super::block::normalize_label(&label);
    let (url, title) = refs.get(&normalized)?;
    Some((normalized, url, title, end))
}

fn match_link(
    chars: &[char],
    start: usize,
    ctx: &InlineCtx,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
    hooks: &ParserHooks,
) -> Option<(Inline, usize)> {
    let (text, after_text) = match_bracket(chars, start, '[')?;
    let (url, title, after_url) = match_paren(chars, after_text)?;
    let children = parse_inline_with_hooks(&text, options, refs, hooks);
    if contains_link(&children) {
        return None;
    }
    Some((
        Inline::Link {
            text: children,
            url,
            title,
            autolink: false,
            pos: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(after_url, chars.len()),
            ),
        },
        after_url,
    ))
}

fn match_image(
    chars: &[char],
    start: usize,
    ctx: &InlineCtx,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
    hooks: &ParserHooks,
) -> Option<(Inline, usize)> {
    let (alt, after_text) = match_bracket(chars, start + 1, '[')?;
    let (url, title, end) = if chars.get(after_text) == Some(&'(') {
        let (url, title, end) = match_paren(chars, after_text)?;
        (url, title, end)
    } else {
        let (_, url, title, end) = match_reference_target(chars, after_text, &alt, refs)?;
        (url.clone(), title.clone(), end)
    };
    let parsed_alt = parse_inline_with_hooks(&alt, options, refs, hooks);
    let alt = inline_plain_text(&parsed_alt);
    Some((
        Inline::Image {
            alt,
            url,
            title,
            pos: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(end, chars.len()),
            ),
        },
        end,
    ))
}

fn inline_plain_text(nodes: &[Inline]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            Inline::Text { value, .. } => result.push_str(value),
            Inline::Code { code, .. } => result.push_str(code),
            Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
                result.push_str(&inline_plain_text(children));
            }
            Inline::Link { text, .. } | Inline::LinkReference { text, .. } => {
                result.push_str(&inline_plain_text(text));
            }
            Inline::Image { alt, .. } => result.push_str(alt),
            Inline::HardBreak { .. } | Inline::SoftBreak { .. } => result.push('\n'),
            Inline::HtmlInline { html, .. } => result.push_str(html),
        }
    }
    result
}

fn contains_link(nodes: &[Inline]) -> bool {
    nodes.iter().any(|node| match node {
        Inline::Link { .. } | Inline::LinkReference { .. } => true,
        Inline::Emphasis { children, .. } | Inline::Strikethrough { children, .. } => {
            contains_link(children)
        }
        Inline::Image { .. }
        | Inline::Text { .. }
        | Inline::Code { .. }
        | Inline::HtmlInline { .. }
        | Inline::HardBreak { .. }
        | Inline::SoftBreak { .. } => false,
    })
}

/// Match `[...content...]` starting at `chars[start]`.
fn match_bracket(chars: &[char], start: usize, open: char) -> Option<(String, usize)> {
    if chars.get(start)? != &open {
        return None;
    }
    let close = if open == '[' { ']' } else { ')' };
    let mut depth = 1;
    let mut i = start + 1;
    let mut content = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
            content.push(c);
            content.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '`' {
            let run = chars[i..].iter().take_while(|&&ch| ch == '`').count();
            if let Some((_, end)) = match_inline_code(chars, i, run) {
                content.extend(chars[i..end].iter().copied());
                i = end;
                continue;
            }
        }
        if c == '<'
            && let Some(end) = skip_angle_sequence(chars, i)
        {
            content.extend(chars[i..end].iter().copied());
            i = end;
            continue;
        }
        if c == open {
            depth += 1;
            content.push(c);
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some((content, i + 1));
            }
            content.push(c);
        } else {
            content.push(c);
        }
        i += 1;
    }
    None
}

fn skip_angle_sequence(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'<') {
        return None;
    }
    let mut quote = None;
    for (index, &c) in chars.iter().enumerate().skip(start + 1) {
        if let Some(active) = quote {
            if c == active {
                quote = None;
            }
        } else {
            match c {
                '"' | '\'' => quote = Some(c),
                '>' => return Some(index + 1),
                '\n' => return None,
                _ => {}
            }
        }
    }
    None
}

/// Match `(...)` starting at `chars[start]`.
fn match_paren(chars: &[char], start: usize) -> Option<(String, Option<String>, usize)> {
    if chars.get(start)? != &'(' {
        return None;
    }
    let mut i = start + 1;
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }

    let mut destination = String::new();
    if chars.get(i) == Some(&'<') {
        i += 1;
        let mut closed = false;
        while i < chars.len() {
            match chars[i] {
                '\n' | '<' => return None,
                '\\' if i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) => {
                    destination.push(chars[i]);
                    destination.push(chars[i + 1]);
                    i += 2;
                }
                '>' => {
                    i += 1;
                    closed = true;
                    break;
                }
                c => {
                    destination.push(c);
                    i += 1;
                }
            }
        }
        if !closed {
            return None;
        }
    } else {
        let mut depth = 0usize;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
                destination.push(c);
                destination.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c.is_whitespace() {
                break;
            }
            if c == '<' || c == '>' {
                return None;
            }
            if c == '(' {
                depth += 1;
                if depth > 32 {
                    return None;
                }
            } else if c == ')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            destination.push(c);
            i += 1;
        }
        if depth != 0 {
            return None;
        }
    }

    let url = decode_char_references(&unescape_backslash_punctuation(&destination));
    let whitespace_start = i;
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    let had_whitespace = i > whitespace_start;

    let title = if had_whitespace && matches!(chars.get(i), Some('"') | Some('\'') | Some('(')) {
        let opening = chars[i];
        let closing = if opening == '(' { ')' } else { opening };
        i += 1;
        let mut raw_title = String::new();
        let mut closed = false;
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
                raw_title.push(chars[i]);
                raw_title.push(chars[i + 1]);
                i += 2;
            } else if chars[i] == closing {
                i += 1;
                closed = true;
                break;
            } else {
                raw_title.push(chars[i]);
                i += 1;
            }
        }
        if !closed {
            return None;
        }
        Some(decode_char_references(&unescape_backslash_punctuation(
            &raw_title,
        )))
    } else {
        None
    };

    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    if chars.get(i) != Some(&')') {
        return None;
    }
    Some((url, title, i + 1))
}

fn match_emphasis(
    chars: &[char],
    start: usize,
    marker: char,
    ctx: &InlineCtx,
    options: Option<&AstOptions>,
    refs: Option<&LinkRefMap>,
    hooks: &ParserHooks,
) -> Option<(Inline, usize)> {
    let run = chars[start..].iter().take_while(|&c| *c == marker).count();
    if !delimiter_can_open(chars, start, run, marker) {
        return None;
    }
    let primary_width = if run % 2 == 1 { 1 } else { 2 };
    let mut widths = vec![primary_width];
    if run >= 2 {
        widths.push(if primary_width == 1 { 2 } else { 1 });
    }
    let (width, close_start, after) = widths.into_iter().find_map(|width| {
        find_emphasis_close(chars, start, marker, run, width, refs)
            .map(|(close_start, after)| (width, close_start, after))
    })?;
    let inner: String = chars[start + width..close_start].iter().collect();
    let children = parse_inline_with_hooks(&inner, options, refs, hooks);
    Some((
        Inline::Emphasis {
            level: if width == 2 {
                EmphasisLevel::Bold
            } else {
                EmphasisLevel::Italic
            },
            children,
            pos: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(after, chars.len()),
            ),
        },
        after,
    ))
}

fn find_emphasis_close(
    chars: &[char],
    start: usize,
    marker: char,
    opening_run: usize,
    width: usize,
    refs: Option<&LinkRefMap>,
) -> Option<(usize, usize)> {
    let opening_can_close = delimiter_can_close(chars, start, opening_run, marker);
    let mut openers = vec![(start, width, opening_run, opening_can_close, false)];
    if opening_run > width {
        openers.push((
            start + width,
            opening_run - width,
            opening_run,
            opening_can_close,
            true,
        ));
    }
    let mut i = start + opening_run;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
            i += 2;
            continue;
        }
        if chars[i] == '`' {
            let ticks = chars[i..].iter().take_while(|&&ch| ch == '`').count();
            if let Some((_, end)) = match_inline_code(chars, i, ticks) {
                i = end;
                continue;
            }
        }
        if chars[i] == '<'
            && let Some((_, _, end)) = match_angle_autolink(chars, i)
        {
            i = end;
            continue;
        }
        if chars[i] == '<'
            && let Some(end) = match_inline_html(chars, i)
        {
            i = end;
            continue;
        }
        if chars[i] == '['
            && let Some((_, end)) = match_bracket(chars, i, '[')
        {
            let (label, after_label) = match_bracket(chars, i, '[')?;
            let end = if chars.get(after_label) == Some(&'(') {
                match_paren(chars, after_label).map(|(_, _, end)| end)
            } else if chars.get(after_label) == Some(&'[') {
                match_bracket(chars, after_label, '[').map(|(_, end)| end)
            } else {
                refs.and_then(|refs| refs.get(&super::block::normalize_label(&label)))
                    .map(|_| end)
            };
            if let Some(end) = end {
                i = end;
                continue;
            }
        }
        let other_marker = if marker == '*' { '_' } else { '*' };
        if chars[i] == other_marker {
            let run = chars[i..]
                .iter()
                .take_while(|&&ch| ch == other_marker)
                .count();
            if delimiter_can_open(chars, i, run, other_marker)
                && let Some(end) = find_simple_emphasis_close(chars, i, other_marker, run)
                && !has_delimiter_close(chars, i + run, end - if run >= 2 { 2 } else { 1 }, marker)
            {
                i = end;
                continue;
            }
        }

        if chars[i] == marker {
            let run = chars[i..].iter().take_while(|&&c| c == marker).count();
            let can_open = delimiter_can_open(chars, i, run, marker);
            let can_close = delimiter_can_close(chars, i, run, marker);
            let mut remaining = run;
            let mut consumed = 0;
            if can_close {
                while remaining > 0 {
                    if let Some((_, opener_remaining, _, _, true)) = openers.last()
                        && !((*opener_remaining >= 2 && remaining >= 2)
                            || (*opener_remaining == 1 && remaining >= 1))
                    {
                        break;
                    }
                    let opener_index = openers.iter().rposition(
                        |(_, opener_remaining, opener_total, can_close, _)| {
                            let can_pair = if *opener_remaining >= 2 {
                                remaining >= 2
                            } else {
                                remaining >= 1
                            };
                            can_pair
                                && !((can_open || *can_close)
                                    && (*opener_total + run).is_multiple_of(3)
                                    && (!opener_total.is_multiple_of(3) || !run.is_multiple_of(3)))
                        },
                    );
                    let Some(opener_index) = opener_index else {
                        break;
                    };
                    let (opener_start, opener_remaining, _, _, _) = openers[opener_index];
                    let used = if opener_remaining >= 2 && remaining >= 2 {
                        2
                    } else {
                        1
                    };
                    remaining -= used;
                    consumed += used;
                    if opener_remaining == used {
                        openers.remove(opener_index);
                    } else {
                        openers[opener_index].1 -= used;
                    }
                    if opener_start == start && opener_remaining == used {
                        let outer_close = i + consumed - used;
                        return Some((outer_close, outer_close + used));
                    }
                }
            }
            if can_open && remaining > 0 {
                openers.push((i, remaining, run, can_close, false));
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

fn find_simple_emphasis_close(
    chars: &[char],
    start: usize,
    marker: char,
    opening_run: usize,
) -> Option<usize> {
    let width = if opening_run >= 2 { 2 } else { 1 };
    let mut i = start + opening_run;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && is_ascii_punctuation(chars[i + 1]) {
            i += 2;
            continue;
        }
        if chars[i] == '`' {
            let ticks = chars[i..].iter().take_while(|&&ch| ch == '`').count();
            if let Some((_, end)) = match_inline_code(chars, i, ticks) {
                i = end;
                continue;
            }
        }
        if chars[i] == marker {
            let run = chars[i..].iter().take_while(|&&ch| ch == marker).count();
            if run >= width && delimiter_can_close(chars, i, run, marker) {
                return Some(i + width);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

fn has_delimiter_close(chars: &[char], start: usize, end: usize, marker: char) -> bool {
    let mut i = start;
    while i < end {
        if chars[i] == '\\' && i + 1 < end && is_ascii_punctuation(chars[i + 1]) {
            i += 2;
        } else if chars[i] == marker {
            let run = chars[i..end].iter().take_while(|&&ch| ch == marker).count();
            if delimiter_can_close(chars, i, run, marker) {
                return true;
            }
            i += run;
        } else {
            i += 1;
        }
    }
    false
}

fn delimiter_can_open(chars: &[char], start: usize, run: usize, marker: char) -> bool {
    let before = start.checked_sub(1).and_then(|i| chars.get(i)).copied();
    let after = chars.get(start + run).copied();
    let before_whitespace = before.is_none_or(char::is_whitespace);
    let after_whitespace = after.is_none_or(char::is_whitespace);
    let before_punctuation = before.is_some_and(is_unicode_punctuation_or_symbol);
    let after_punctuation = after.is_some_and(is_unicode_punctuation_or_symbol);
    let left_flanking =
        !after_whitespace && (!after_punctuation || before_whitespace || before_punctuation);
    let right_flanking =
        !before_whitespace && (!before_punctuation || after_whitespace || after_punctuation);

    if marker == '_' {
        left_flanking && (!right_flanking || before_punctuation)
    } else {
        left_flanking
    }
}

fn delimiter_can_close(chars: &[char], start: usize, run: usize, marker: char) -> bool {
    let before = start.checked_sub(1).and_then(|i| chars.get(i)).copied();
    let after = chars.get(start + run).copied();
    let before_whitespace = before.is_none_or(char::is_whitespace);
    let after_whitespace = after.is_none_or(char::is_whitespace);
    let before_punctuation = before.is_some_and(is_unicode_punctuation_or_symbol);
    let after_punctuation = after.is_some_and(is_unicode_punctuation_or_symbol);
    let left_flanking =
        !after_whitespace && (!after_punctuation || before_whitespace || before_punctuation);
    let right_flanking =
        !before_whitespace && (!before_punctuation || after_whitespace || after_punctuation);

    if marker == '_' {
        right_flanking && (!left_flanking || after_punctuation)
    } else {
        right_flanking
    }
}

fn is_unicode_punctuation_or_symbol(c: char) -> bool {
    c.is_ascii_punctuation() || (!c.is_alphanumeric() && !c.is_whitespace() && !c.is_control())
}

pub(super) fn match_inline_html(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start)? != &'<' {
        return None;
    }
    let tail: String = chars[start..].iter().collect();
    for (prefix, terminator) in [("<!--", "-->"), ("<?", "?>"), ("<![CDATA[", "]]>")] {
        if tail.starts_with(prefix) {
            let end = tail.find(terminator)? + terminator.len();
            return Some(start + tail[..end].chars().count());
        }
    }
    if tail.starts_with("<!") && chars.get(start + 2).is_some_and(char::is_ascii_uppercase) {
        let close = (start + 2..chars.len()).find(|&i| chars[i] == '>')?;
        return Some(close + 1);
    }

    let mut i = start + 1;
    let closing = chars.get(i) == Some(&'/');
    if closing {
        i += 1;
    }
    if !chars.get(i).is_some_and(char::is_ascii_alphabetic) {
        return None;
    }
    i += 1;
    while chars
        .get(i)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-')
    {
        i += 1;
    }

    if closing {
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        return (chars.get(i) == Some(&'>')).then_some(i + 1);
    }

    loop {
        let before_space = i;
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        let had_space = i > before_space;
        match chars.get(i) {
            Some('>') => return Some(i + 1),
            Some('/') if chars.get(i + 1) == Some(&'>') => return Some(i + 2),
            None => return None,
            _ if !had_space => return None,
            _ => {}
        }

        if !chars
            .get(i)
            .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '_' | ':'))
        {
            return None;
        }
        i += 1;
        while chars
            .get(i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.' | '-'))
        {
            i += 1;
        }
        let name_end_with_space = i;
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        if chars.get(i) != Some(&'=') {
            i = name_end_with_space;
            continue;
        }
        i += 1;
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        match chars.get(i)? {
            '"' | '\'' => {
                let quote = chars[i];
                i += 1;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if chars.get(i) != Some(&quote) {
                    return None;
                }
                i += 1;
            }
            _ => {
                let value_start = i;
                while chars.get(i).is_some_and(|c| {
                    !c.is_whitespace() && !matches!(c, '"' | '\'' | '=' | '<' | '>' | '`')
                }) {
                    i += 1;
                }
                if i == value_start {
                    return None;
                }
            }
        }
    }
}
