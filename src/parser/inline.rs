//! Inline-level Markdown parser.
//!
//! Parses inline constructs: emphasis, code spans, links, images,
//! strikethrough (GFM), autolinks (GFM), and raw HTML.

use crate::ast::{EmphasisLevel, Inline, Span};

/// Precomputed mapping from char-index → source position within an inline
/// input string.  Positions are relative to that string (typically the
/// joined paragraph text), not the original document.
pub struct InlineCtx {
    /// `line_starts_char[l]` = char index at which line `l` begins.
    line_starts_char: Vec<usize>,
    /// `char_byte_offsets[i]` = byte offset of `chars[i]`.
    char_byte_offsets: Vec<usize>,
}

impl InlineCtx {
    fn new(input: &str, chars: &[char]) -> Self {
        let mut line_starts_char = vec![0usize];
        let mut char_byte_offsets = Vec::with_capacity(chars.len());
        let mut byte = 0usize;
        for c in chars {
            char_byte_offsets.push(byte);
            if *c == '\n' {
                line_starts_char.push(char_byte_offsets.len());
            }
            byte += c.len_utf8();
        }
        let _ = input;
        Self {
            line_starts_char,
            char_byte_offsets,
        }
    }

    /// Position at the given char index.
    pub fn position(&self, i: usize, chars_len: usize) -> crate::ast::Position {
        let i = i.min(chars_len);
        let line = match self.line_starts_char.binary_search(&i) {
            Ok(l) => l,
            Err(l) => l.saturating_sub(1),
        };
        let line_start = self.line_starts_char[line];
        let column = i - line_start;
        let offset = if i < self.char_byte_offsets.len() {
            self.char_byte_offsets[i]
        } else {
            self.char_byte_offsets.last().copied().unwrap_or(0)
                + if chars_len > 0 && self.char_byte_offsets.len() > 1 {
                    self.char_byte_offsets[self.char_byte_offsets.len() - 1]
                        - self.char_byte_offsets[self.char_byte_offsets.len() - 2]
                } else {
                    0
                }
        };
        crate::ast::Position {
            line,
            column,
            offset,
        }
    }
}

/// Parse inline markdown into a list of [`Inline`] nodes.
pub fn parse_inline(input: &str) -> Vec<Inline> {
    parse_inline_with(input, &crate::config::ParseOptions::default())
}

/// Parse inline markdown with the given options (controls GFM features).
pub fn parse_inline_with(input: &str, opts: &crate::config::ParseOptions) -> Vec<Inline> {
    let mut tokens: Vec<Inline> = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let ctx = InlineCtx::new(input, &chars);
    let mut i = 0;
    let mut text = String::new();
    let mut text_start = 0usize;

    while i < chars.len() {
        let c = chars[i];

        // Inline code: `code` or ``code``
        if c == '`' {
            let tick_count = chars[i..].iter().take_while(|&&ch| ch == '`').count();
            if let Some((code, end)) = match_inline_code(&chars, i, tick_count) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::Code {
                    code,
                    position: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(end, chars.len()),
                    ),
                });
                i = end;
                text_start = i;
                continue;
            }
        }

        // GFM strikethrough: ~~text~~
        if opts.gfm && c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
            if let Some((node, end)) = super::gfm::match_strikethrough(&chars, i, &ctx) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(node);
                i = end;
                text_start = i;
                continue;
            }
        }

        // Image: ![alt](url)
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some((img, end)) = match_image(&chars, i, &ctx) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(img);
                i = end;
                text_start = i;
                continue;
            }
        }

        // Link: [text](url)
        if c == '[' {
            if let Some((link, end)) = match_link(&chars, i, &ctx) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(link);
                i = end;
                text_start = i;
                continue;
            }
        }

        // GFM autolink: bare URLs
        if opts.gfm && (c == 'h' || c == 'w') {
            if let Some((url, end)) = super::gfm::match_autolink(&chars, i) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::Link {
                    text: vec![Inline::Text {
                        value: url.clone(),
                        position: Span::new(
                            ctx.position(i, chars.len()),
                            ctx.position(end, chars.len()),
                        ),
                    }],
                    url,
                    title: None,
                    autolink: true,
                    position: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(end, chars.len()),
                    ),
                });
                i = end;
                text_start = i;
                continue;
            }
        }

        // Emphasis: **bold** / *italic* / __bold__ / _italic_
        if c == '*' || c == '_' {
            if let Some((node, end)) = match_emphasis(&chars, i, c, &ctx) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(node);
                i = end;
                text_start = i;
                continue;
            }
        }

        // Hard break: two trailing spaces + newline, or backslash + newline
        if c == '\n' {
            if text.ends_with("  ") {
                text.truncate(text.len() - 2);
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                tokens.push(Inline::HardBreak {
                    position: Span::new(
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
                    position: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(i + 1, chars.len()),
                    ),
                });
                i += 1;
                text_start = i;
                continue;
            }
            flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
            tokens.push(Inline::SoftBreak {
                position: Span::new(
                    ctx.position(i, chars.len()),
                    ctx.position(i + 1, chars.len()),
                ),
            });
            i += 1;
            text_start = i;
            continue;
        }

        // Inline HTML (very naive: <tag ...> or </tag>)
        if c == '<' {
            if let Some(end) = match_inline_html(&chars, i) {
                flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
                let html: String = chars[i..end].iter().collect();
                tokens.push(Inline::HtmlInline {
                    html,
                    position: Span::new(
                        ctx.position(i, chars.len()),
                        ctx.position(end, chars.len()),
                    ),
                });
                i = end;
                text_start = i;
                continue;
            }
        }

        text.push(c);
        i += 1;
    }

    flush_text(&mut tokens, &mut text, &mut text_start, &ctx, &chars);
    tokens
}

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
        let span = Span::new(
            ctx.position(start, chars.len()),
            ctx.position(end, chars.len()),
        );
        tokens.push(Inline::Text {
            value: std::mem::take(text),
            position: span,
        });
    }
}

fn match_inline_code(chars: &[char], start: usize, tick_count: usize) -> Option<(String, usize)> {
    let open: String = std::iter::repeat('`').take(tick_count).collect();
    let rest = &chars[start + tick_count..];
    let rest_str: String = rest.iter().collect();
    let close_idx = rest_str.find(&open)?;
    let code = rest_str[..close_idx].to_string();
    let end = start + tick_count + close_idx + tick_count;
    Some((code, end))
}

fn match_link(chars: &[char], start: usize, ctx: &InlineCtx) -> Option<(Inline, usize)> {
    let (text, after_text) = match_bracket(chars, start, '[')?;
    let (url, title, after_url) = match_paren(chars, after_text)?;
    let children = parse_inline(&text);
    Some((
        Inline::Link {
            text: children,
            url,
            title,
            autolink: false,
            position: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(after_url, chars.len()),
            ),
        },
        after_url,
    ))
}

fn match_image(chars: &[char], start: usize, ctx: &InlineCtx) -> Option<(Inline, usize)> {
    let (alt, after_text) = match_bracket(chars, start + 1, '[')?;
    let (url, title, after_url) = match_paren(chars, after_text)?;
    Some((
        Inline::Image {
            alt,
            url,
            title,
            position: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(after_url, chars.len()),
            ),
        },
        after_url,
    ))
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

/// Match `(...)` starting at `chars[start]`.
fn match_paren(chars: &[char], start: usize) -> Option<(String, Option<String>, usize)> {
    if chars.get(start)? != &'(' {
        return None;
    }
    let mut depth = 1;
    let mut i = start + 1;
    let mut raw = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '(' {
            depth += 1;
            raw.push(c);
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                let (url, title) = parse_link_dest(&raw);
                return Some((url, title, i + 1));
            }
            raw.push(c);
        } else {
            raw.push(c);
        }
        i += 1;
    }
    None
}

/// Split `url  "title"` into components.
fn parse_link_dest(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_suffix('"') {
        if let Some(qidx) = rest.rfind('"') {
            let url = rest[..qidx].trim().to_string();
            let title = rest[qidx + 1..].trim().to_string();
            return (url, Some(title));
        }
    }
    if let Some(rest) = trimmed.strip_suffix('\'') {
        if let Some(qidx) = rest.rfind('\'') {
            let url = rest[..qidx].trim().to_string();
            let title = rest[qidx + 1..].trim().to_string();
            return (url, Some(title));
        }
    }
    (trimmed.to_string(), None)
}

fn match_emphasis(
    chars: &[char],
    start: usize,
    marker: char,
    ctx: &InlineCtx,
) -> Option<(Inline, usize)> {
    let run = chars[start..].iter().take_while(|&c| *c == marker).count();
    // Try bold first (double marker)
    if run >= 2 {
        let close = find_emphasis_close(chars, start + 2, marker, 2);
        if let Some(end) = close {
            let inner: String = chars[start + 2..end].iter().collect();
            let children = parse_inline(&inner);
            let after = end + 2;
            return Some((
                Inline::Emphasis {
                    level: EmphasisLevel::Bold,
                    children,
                    position: Span::new(
                        ctx.position(start, chars.len()),
                        ctx.position(after, chars.len()),
                    ),
                },
                after,
            ));
        }
    }
    // Single — italic
    let end = find_emphasis_close(chars, start + 1, marker, 1)?;
    let inner: String = chars[start + 1..end].iter().collect();
    let children = parse_inline(&inner);
    let after = end + 1;
    Some((
        Inline::Emphasis {
            level: EmphasisLevel::Italic,
            children,
            position: Span::new(
                ctx.position(start, chars.len()),
                ctx.position(after, chars.len()),
            ),
        },
        after,
    ))
}

fn find_emphasis_close(chars: &[char], from: usize, marker: char, count: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == marker {
            let run = chars[i..].iter().take_while(|&&c| c == marker).count();
            if run >= count {
                return Some(i);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

fn match_inline_html(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start)? != &'<' {
        return None;
    }
    let mut i = start + 1;
    // comment
    if chars.get(i)? == &'!' {
        let comment_end = chars[start..]
            .windows(3)
            .position(|w| w == ['-', '-', '>'])?;
        return Some(start + comment_end + 3);
    }
    // closing tag
    if chars.get(i)? == &'/' {
        i += 1;
    }
    // tag name
    if chars.get(i)?.is_ascii_alphabetic() {
        while i < chars.len() && chars[i] != '>' {
            i += 1;
        }
        if i < chars.len() && chars[i] == '>' {
            return Some(i + 1);
        }
    }
    None
}
