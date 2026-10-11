//! Block-level Markdown parser.
//!
//! This module implements the block-level parser that converts lines of
//! Markdown source text into [`Block`] nodes.  It supports:
//!
//! - ATX headings (`#` … `######`)
//! - Fenced and indented code blocks
//! - Block quotes
//! - Ordered and unordered lists (with GFM task-list markers)
//! - Thematic breaks
//! - HTML blocks and HTML comments (`<!-- ... -->`)
//! - GFM tables
//! - Link reference definitions (`[label]: url "title"`)
//! - Kramdown block attributes (`{: #id .class}`)
//!
//! The [`ParserState`] struct holds the parser's cursor position and shared
//! context.  The [`md_to_ast`](super::md_to_ast) function creates a
//! `ParserState` and drives it to completion.

use super::atters::parse_attrs;
use super::inline::{LinkRefMap, parse_inline_with_hooks};
use super::plugin::{BlockParserContext, ParserHooks, finalize_hook_block};
use super::table::{build_table, is_table_start, parse_delimiter_alignments};
use crate::markdown::ast::options::AstOptions;
use crate::markdown::ast::tokens::token::TaskState;
use crate::markdown::ast::tokens::{
    Attributes,
    span::{Position, Span},
    token::{Block, LinkReferenceDefinition, ListItem},
};

/// Compute the byte offset of the start of each line.
///
/// Line 0 always starts at offset 0.  For each `\n` in `input`, a new
/// entry is added pointing to the byte immediately after the newline.
///
pub fn compute_line_starts(input: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in input.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Internal parser state for block-level parsing.
///
/// Holds a reference to the source text, the line slice, line-start offsets,
/// parser options, the link reference map (for resolving reference-style
/// links during inline parsing), and the custom parser hooks.
pub struct ParserState<'a> {
    /// The full source text.
    pub input: &'a str,
    /// Lines of the source text (split on `\n`).
    pub lines: &'a [&'a str],
    /// Byte offset of the start of each line.
    pub line_starts: &'a [usize],
    /// Current line index (0-based).
    pub pos: usize,
    /// Parser options.
    pub opts: &'a AstOptions,
    /// Optional file name.
    pub file_name: Option<String>,
    /// Link reference definitions collected in the pre-pass.
    pub refs: &'a LinkRefMap,
    /// Custom parser hooks (empty when none registered).
    pub hooks: &'a ParserHooks<'a>,
}
impl<'a> ParserState<'a> {
    /// Create a new `ParserState` from the given source and options.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: &'a str,
        lines: &'a [&'a str],
        line_starts: &'a [usize],
        opts: &'a AstOptions,
        file_name: Option<String>,
        refs: &'a LinkRefMap,
        hooks: &'a ParserHooks<'a>,
    ) -> Self {
        Self {
            input,
            lines,
            line_starts,
            pos: 0,
            opts,
            file_name,
            refs,
            hooks,
        }
    }

    /// Returns `true` if the parser has consumed all lines.
    pub fn is_done(&self) -> bool {
        self.pos >= self.lines.len()
    }

    /// Returns the current line, or `None` if at end of input.
    pub fn current(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    /// Advance to the next line.
    pub fn advance(&mut self) {
        self.pos += 1;
    }

    /// The [`Position`] at the start of logical line `line` (0-based).
    pub fn position_at(&self, line: usize) -> Position {
        let line = line.min(self.line_starts.len().saturating_sub(1));
        let offset = self.line_starts[line].min(self.input.len());
        Position {
            line: line as u32,
            column: 0,
            offset: offset as u32,
        }
    }
    /// Span covering lines `[from, to)` (half-open).
    pub fn span(&self, from: usize, to: usize) -> Span {
        Span::new(self.position_at(from), self.position_at(to))
    }

    /// Skip blank lines. Returns `true` if at least one was consumed.
    pub fn skip_blank_lines(&mut self) -> bool {
        let start = self.pos;
        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                self.advance();
            } else {
                break;
            }
        }
        self.pos > start
    }
    /// Parse the current line as a block-level node.
    ///
    /// Returns `Some(Block)` if a block was successfully parsed, or `None`
    /// if no block construct matched.  Advances the parser position past
    /// the consumed lines.
    pub fn parse_block(&mut self) -> Option<Block> {
        let line = self.current()?;
        let start_line = self.pos;

        // Custom parser hooks — tried before every built-in matcher so an
        // extension can both introduce new syntax and override built-ins.
        if !self.hooks.is_empty() {
            if let Some(block) = self.try_hook_block() {
                return Some(block);
            }
        }

        // GFM table
        if self.opts.gfm && is_table_start(self.lines, self.pos) {
            return Some(self.parse_table(start_line));
        }

        // Thematic break
        if is_thematic_break(line) {
            self.advance();
            return Some(Block::ThematicBreak {
                pos: self.span(start_line, self.pos),
            });
        }

        // ATX heading
        if let Some(block) = self.try_heading() {
            return Some(block);
        }

        // Fenced code block
        if let Some(block) = self.try_fenced_code() {
            return Some(block);
        }

        // Indented code block (4+ leading spaces)
        if let Some(block) = self.try_indented_code() {
            return Some(block);
        }

        // Block quote
        if line.trim_start().starts_with('>') {
            return Some(self.parse_block_quote());
        }
        // List
        if is_list_marker(line) {
            return Some(self.parse_list());
        }

        // HTML block
        if is_html_block_start(line) {
            // HTML comment block
            if line.trim_start().starts_with("<!--") {
                return Some(self.parse_html_comment(start_line));
            }
            let html = self.collect_html_block();
            let attrs = self.try_trailing_attrs();
            return Some(Block::HtmlBlock {
                attrs,
                html,
                pos: self.span(start_line, self.pos),
            });
        }

        // Link reference definition: `[label]: url "title"`
        if let Some(block) = self.try_link_reference_definition() {
            return Some(block);
        }

        // Default: paragraph
        Some(self.parse_paragraph())
    }
    /// Try the registered custom block hooks at the current position.
    ///
    /// Returns the hook-parsed block with an accurate source span and any
    /// trailing Kramdown attribute block applied, or `None` when every
    /// hook declined (in which case the built-in matchers proceed).
    fn try_hook_block(&mut self) -> Option<Block> {
        let ctx = BlockParserContext {
            line: self.current()?.to_string(),
            line_index: self.pos as u32,
            lines: self.lines[self.pos..]
                .iter()
                .map(|l| l.to_string())
                .collect(),
        };
        let (block, consumed) = self.hooks.try_parse_block(&ctx)?;
        // A hook must consume at least one line; clamp defensively.
        let consumed = consumed.max(1);
        let end_line = (self.pos + consumed).min(self.lines.len());
        let pos = self.span(self.pos, end_line);
        self.pos = end_line;
        let attrs = self.try_trailing_attrs();
        Some(finalize_hook_block(block, pos, attrs))
    }
    /// After parsing a block, check if the next line is a Kramdown
    /// attribute block `{:...}` that applies to this block.  Blank lines
    /// between the block and the attribute marker are allowed.
    fn try_trailing_attrs(&mut self) -> Option<Attributes> {
        if !self.opts.kramdown {
            return None;
        }
        // Skip blank lines (but remember to consume them).
        let mut peek = self.pos;
        while peek < self.lines.len() && self.lines[peek].trim().is_empty() {
            peek += 1;
        }
        if peek < self.lines.len() {
            let trimmed = self.lines[peek].trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Some((attrs, _)) = parse_attrs(trimmed) {
                    // Consume the blank lines + the attribute line.
                    self.pos = peek + 1;
                    return Some(attrs);
                }
            }
        }
        None
    }
    // -----------------------------------------------------------------------
    // Tables (GFM)
    // -----------------------------------------------------------------------

    fn parse_table(&mut self, start_line: usize) -> Block {
        let header_line = self.current().unwrap();
        let delimiter_line = self.lines[self.pos + 1];
        let alignments = parse_delimiter_alignments(delimiter_line);

        self.advance(); // header
        self.advance(); // delimiter

        let mut body_lines: Vec<&str> = Vec::new();
        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                break;
            }
            if !line.contains('|') {
                break;
            }
            body_lines.push(line);
            self.advance();
        }

        let mut block = build_table(
            header_line,
            delimiter_line,
            alignments,
            &body_lines,
            Some(self.opts),
            self.hooks,
        );
        // Fix position + trailing attrs
        if let Block::Table { attrs, pos, .. } = &mut block {
            *pos = self.span(start_line, self.pos);
            *attrs = self.try_trailing_attrs();
        }
        block
    }
    // -----------------------------------------------------------------------
    // Link Reference Definitions
    // -----------------------------------------------------------------------

    /// Try to parse a link reference definition: `[label]: url "title"`
    fn try_link_reference_definition(&mut self) -> Option<Block> {
        let line = self.current()?;
        let trimmed = line.trim_start();

        // Must start with `[`
        if !trimmed.starts_with('[') {
            return None;
        }

        // Find the closing `]`
        let after_open = &trimmed[1..];
        let bracket_close = match after_open.find(']') {
            Some(idx) => 1 + idx, // index in trimmed
            None => return None,
        };

        // Must be followed by `:`
        let after_bracket = &trimmed[bracket_close + 1..];
        if !after_bracket.trim_start().starts_with(':') {
            return None;
        }

        // Parse the label
        let label_raw: String = trimmed[1..bracket_close].to_string();
        if label_raw.trim().is_empty() {
            return None;
        }
        let label = normalize_label(&label_raw);

        // Everything after `:`
        let after_colon = &after_bracket.trim_start()[1..]; // skip ':'
        let rest = after_colon.trim_start();

        // Parse URL — may be wrapped in `<...>` or bare
        let (url, after_url) = parse_link_url(rest);

        // Parse optional title
        let title_part = after_url.trim();
        let (title, _consumed) = parse_link_title(title_part);

        let start_line = self.pos;
        self.advance();

        Some(Block::LinkReferenceDefinition {
            label,
            url,
            title,
            pos: self.span(start_line, self.pos),
        })
    }

    // -----------------------------------------------------------------------
    // HTML Comment
    // -----------------------------------------------------------------------

    /// Parse an HTML comment block starting with `<!--`.
    fn parse_html_comment(&mut self, start_line: usize) -> Block {
        let mut content = String::new();
        let mut found_close = false;

        while let Some(line) = self.current() {
            if !found_close {
                content.push_str(line);
                content.push('\n');
                if let Some(_pos) = line.find("-->") {
                    found_close = true;
                    self.advance();
                    break;
                }
                self.advance();
            } else {
                break;
            }
        }

        // If we never found -->, keep consuming until blank line (HTML block condition)
        if !found_close {
            while let Some(line) = self.current() {
                if line.trim().is_empty() {
                    break;
                }
                content.push_str(line);
                content.push('\n');
                self.advance();
            }
        }

        // Strip the `<!--` prefix and `-->` suffix to extract the comment value
        let value = extract_comment_content(&content);

        Block::Comment {
            value,
            pos: self.span(start_line, self.pos),
        }
    }

    // -----------------------------------------------------------------------
    // Headings
    // -----------------------------------------------------------------------

    fn try_heading(&mut self) -> Option<Block> {
        let line = self.current()?;
        let start_line = self.pos;
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if hashes == 0 || hashes > 6 {
            return None;
        }
        let rest = &trimmed[hashes..];
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        let text = rest.trim();
        self.advance();

        // Kramdown: block attribute marker on the next line:
        //   # Title
        //   {: #id .cls key="val"}
        let heading_text = text.to_string();
        let attrs = self.try_trailing_attrs();

        let children =
            parse_inline_with_hooks(&heading_text, Some(self.opts), Some(self.refs), self.hooks);
        Some(Block::Heading {
            level: hashes as u8,
            children,
            attrs,
            pos: self.span(start_line, self.pos),
        })
    }
    // -----------------------------------------------------------------------
    // Fenced code
    // -----------------------------------------------------------------------

    fn try_fenced_code(&mut self) -> Option<Block> {
        let line = self.current()?;
        let trimmed = line.trim_start();
        let fence_char = trimmed.chars().next()?;
        if fence_char != '`' && fence_char != '~' {
            return None;
        }
        let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
        if fence_len < 3 {
            return None;
        }
        let info = trimmed[fence_len..].trim();
        let lang = if info.is_empty() {
            None
        } else {
            Some(info.to_string())
        };

        let start_line = self.pos;
        self.advance();
        let mut code = String::new();
        while let Some(l) = self.current() {
            let lt = l.trim_start();
            if lt.starts_with(fence_char)
                && lt.chars().take_while(|&c| c == fence_char).count() >= fence_len
            {
                self.advance();
                break;
            }
            code.push_str(l);
            code.push('\n');
            self.advance();
        }
        if code.ends_with('\n') {
            code.pop();
        }

        let attrs = self.try_trailing_attrs();
        Some(Block::CodeBlock {
            lang,
            code,
            attrs,
            pos: self.span(start_line, self.pos),
        })
    }
    // -----------------------------------------------------------------------
    // Indented code
    // -----------------------------------------------------------------------

    fn try_indented_code(&mut self) -> Option<Block> {
        let line = self.current()?;
        if !(line.starts_with("    ") || line.starts_with('\t')) {
            return None;
        }

        let start_line = self.pos;
        let mut code = String::new();
        while let Some(l) = self.current() {
            if l.trim().is_empty() {
                if self.pos + 1 < self.lines.len()
                    && (self.lines[self.pos + 1].starts_with("    ")
                        || self.lines[self.pos + 1].starts_with('\t'))
                {
                    code.push('\n');
                    self.advance();
                    continue;
                } else {
                    break;
                }
            }
            if l.starts_with("    ") {
                code.push_str(&l[4..]);
            } else if l.starts_with('\t') {
                code.push_str(&l[1..]);
            } else {
                break;
            }
            code.push('\n');
            self.advance();
        }
        if code.ends_with('\n') {
            code.pop();
        }

        let attrs = self.try_trailing_attrs();
        Some(Block::CodeBlock {
            lang: None,
            code,
            attrs,
            pos: self.span(start_line, self.pos),
        })
    }
    // -----------------------------------------------------------------------
    // Block quote
    // -----------------------------------------------------------------------

    fn parse_block_quote(&mut self) -> Block {
        let start_line = self.pos;
        let mut inner: Vec<&str> = Vec::new();
        while let Some(line) = self.current() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix('>') {
                inner.push(rest.strip_prefix(' ').unwrap_or(rest));
                self.advance();
            } else if line.trim().is_empty() {
                break;
            } else if self.opts.kramdown && is_kramdown_attr_line(line) {
                // Kramdown block attribute marker — stop the block quote;
                // the attrs apply to the quote itself.
                break;
            } else {
                inner.push(line);
                self.advance();
            }
        }
        let inner_doc = super::md_to_ast_with_hooks(
            &inner.join("\n"),
            self.opts,
            self.file_name.clone(),
            self.hooks,
        );
        let attrs = self.try_trailing_attrs();
        Block::BlockQuote {
            children: inner_doc.children,
            attrs,
            pos: self.span(start_line, self.pos),
        }
    }

    // -----------------------------------------------------------------------
    // Lists
    // -----------------------------------------------------------------------

    fn parse_list(&mut self) -> Block {
        let start_line = self.pos;
        let first = self.current().unwrap();
        let ordered = first.trim_start().chars().next().unwrap().is_ascii_digit();

        let mut items: Vec<ListItem> = Vec::new();

        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                let mut peek = self.pos + 1;
                while peek < self.lines.len() && self.lines[peek].trim().is_empty() {
                    peek += 1;
                }
                if peek < self.lines.len() && is_list_marker(self.lines[peek]) {
                    self.pos = peek;
                    continue;
                } else {
                    break;
                }
            }

            if is_list_marker(line) {
                let item_start = self.pos;
                let (content, consumed) = collect_list_item(self.lines, self.pos);
                self.pos += consumed;
                let item_end = self.pos;

                // GFM task list detection.  The `[ ]`/`[x]` marker is part
                // of the collected content, so strip it when present —
                // otherwise it would render as literal text alongside the
                // generated `<input type="checkbox">`.
                let task = if self.opts.gfm {
                    parse_task_marker(line).map(|(state, _)| state)
                } else {
                    None
                };
                let content = match task {
                    Some(_) => strip_task_marker(content),
                    None => content,
                };

                let inner = super::md_to_ast_with_hooks(
                    &content,
                    self.opts,
                    self.file_name.clone(),
                    self.hooks,
                );
                items.push(ListItem {
                    children: inner.children,
                    task,
                    pos: self.span(item_start, item_end),
                });
            } else {
                break;
            }
        }

        let attrs = self.try_trailing_attrs();
        Block::List {
            ordered,
            items,
            attrs,
            pos: self.span(start_line, self.pos),
        }
    }

    // -----------------------------------------------------------------------
    // HTML block
    // -----------------------------------------------------------------------

    fn collect_html_block(&mut self) -> String {
        let mut html = String::new();
        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                break;
            }
            html.push_str(line);
            html.push('\n');
            self.advance();
        }
        if html.ends_with('\n') {
            html.pop();
        }
        html
    }

    // -----------------------------------------------------------------------
    // Paragraph
    // -----------------------------------------------------------------------

    fn parse_paragraph(&mut self) -> Block {
        let start_line = self.pos;
        let mut text_lines: Vec<&str> = Vec::new();
        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                break;
            }
            // Stop if a new block construct begins.
            if is_thematic_break(line)
                || is_heading(line)
                || is_fence(line)
                || line.trim_start().starts_with('>')
                || is_list_marker(line)
                || is_html_block_start(line)
                || is_link_ref_def(line)
            {
                break;
            }
            // GFM table start
            if self.opts.gfm && is_table_start(self.lines, self.pos) {
                break;
            }
            // Kramdown block attribute marker `{:...}` on its own line
            // ends the paragraph (the attrs apply to it).
            if self.opts.kramdown && is_kramdown_attr_line(line) {
                break;
            }
            text_lines.push(line);
            self.advance();
        }

        let raw = text_lines.join("\n");
        let attrs = self.try_trailing_attrs();

        let children = parse_inline_with_hooks(&raw, Some(self.opts), Some(self.refs), self.hooks);
        Block::Paragraph {
            children,
            attrs,
            pos: self.span(start_line, self.pos),
        }
    }
}
// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

fn is_heading(line: &str) -> bool {
    let t = line.trim_start();
    let h = t.chars().take_while(|&c| c == '#').count();
    h >= 1 && h <= 6 && (t[h..].is_empty() || t[h..].starts_with(' '))
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    let c = match t.chars().next() {
        Some('`') | Some('~') => t.chars().next().unwrap(),
        _ => return false,
    };
    t.chars().take_while(|&ch| ch == c).count() >= 3
}

/// Returns `true` if the line is a Kramdown block attribute marker `{:...}`
/// or `{...}` on its own line.
fn is_kramdown_attr_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('{') && t.ends_with('}')
}

/// Returns `true` if the line is a thematic break (`---`, `***`, or `___`
/// with at least three identical characters, optionally spaced).
pub fn is_thematic_break(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return false;
    }
    let c = t.chars().next().unwrap();
    (c == '-' || c == '*' || c == '_') && t.chars().all(|ch| ch == c) && t.len() >= 3
}

/// Returns `true` if the line starts with an HTML block: `<`, `</`, `<!`,
/// or `<` followed by an ASCII letter.
fn is_html_block_start(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with('<') {
        return false;
    }
    let after = &t[1..];
    after.starts_with('/')
        || after.starts_with('!')
        || after
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphabetic())
}

/// Returns `true` if the line looks like a link reference definition:
/// `[label]: url ...`
///
/// This is a lightweight check — it only verifies the `[...]` `:` prefix.
/// Use [`parse_link_ref_def_line`] for full parsing.
pub fn is_link_ref_def(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('[') {
        return false;
    }
    // Find closing `]` followed by `:`
    let after_open = &trimmed[1..];
    let bracket_close = match after_open.find(']') {
        Some(idx) => idx + 1,
        None => return false,
    };
    let after_bracket = trimmed.get(bracket_close + 1..);
    match after_bracket {
        Some(s) => s.trim_start().starts_with(':'),
        None => false,
    }
}

/// Parse a single line as a link reference definition.
///
/// Returns a [`LinkReferenceDefinition`] with position information, or
/// `None` if the line is not a valid definition.
///
/// # Arguments
///
/// - `line` — the raw source line.
/// - `line_idx` — 0-based line index in the document.
/// - `line_starts` — byte offsets of each line start (from [`compute_line_starts`]).
/// - `input` — the full source text (for offset clamping).
pub fn parse_link_ref_def_line(
    line: &str,
    line_idx: usize,
    line_starts: &[usize],
    input: &str,
) -> Option<LinkReferenceDefinition> {
    let trimmed = line.trim_start();

    // Must start with `[`
    if !trimmed.starts_with('[') {
        return None;
    }

    // Find the closing `]`
    let after_open = &trimmed[1..];
    let bracket_close = match after_open.find(']') {
        Some(idx) => 1 + idx, // index in trimmed
        None => return None,
    };

    // Must be followed by `:`
    let after_bracket = &trimmed[bracket_close + 1..];
    if !after_bracket.trim_start().starts_with(':') {
        return None;
    }

    // Parse the label
    let label_raw: String = trimmed[1..bracket_close].to_string();
    if label_raw.trim().is_empty() {
        return None;
    }
    let label = normalize_label(&label_raw);

    // Everything after `:`
    let after_colon = &after_bracket.trim_start()[1..]; // skip ':'
    let rest = after_colon.trim_start();

    // Parse URL — may be wrapped in `<...>` or bare
    let (url, after_url) = parse_link_url(rest);

    // Parse optional title
    let title_part = after_url.trim();
    let (title, _consumed) = parse_link_title(title_part);

    // Compute position
    let start_offset = line_starts
        .get(line_idx)
        .copied()
        .unwrap_or(0)
        .min(input.len());
    let end_offset = (start_offset + line.len()).min(input.len());
    let pos = Span::new(
        Position {
            line: line_idx as u32,
            column: 0,
            offset: start_offset as u32,
        },
        Position {
            line: line_idx as u32,
            column: line.chars().count() as u32,
            offset: end_offset as u32,
        },
    );

    Some(LinkReferenceDefinition {
        label,
        url,
        title,
        pos,
    })
}

/// Returns `true` if the line is a list marker: `-`, `*`, `+` followed by
/// a space, or 1–9 digits followed by `.` or `)` and a space.
pub fn is_list_marker(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return true;
    }
    let digits_end = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_end > 0 && digits_end < 10 {
        let rest = &t[digits_end..];
        if (rest.starts_with(". ") || rest.starts_with(") ")) || (rest == "." || rest == ")") {
            return true;
        }
    }
    false
}

/// Collect a single list item starting at `start`.
fn collect_list_item(lines: &[&str], start: usize) -> (String, usize) {
    let first = lines[start];
    let t = first.trim_start();

    let marker_end = {
        let mut idx = first.len() - t.len();
        if t.starts_with(['-', '*', '+']) {
            idx += 1;
        } else {
            let dlen = t.chars().take_while(|c| c.is_ascii_digit()).count();
            idx += dlen + 1;
        }
        idx
    };

    let content_start = first.len() - first[marker_end..].trim_start().len();
    let first_content = &first[content_start..];

    let mut content = String::new();
    if first_content.is_empty() {
        content.push('\n');
    } else {
        content.push_str(first_content);
        content.push('\n');
    }

    let mut consumed = 1;
    let indent = content_start - (first.len() - t.len());

    for i in (start + 1)..lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            content.push('\n');
            consumed += 1;
            continue;
        }
        let leading: usize = line.chars().take_while(|c| *c == ' ').count();
        if leading >= indent {
            content.push_str(&line[indent.min(line.len())..]);
            content.push('\n');
            consumed += 1;
        } else if is_list_marker(line)
            || is_heading(line)
            || is_fence(line)
            || is_thematic_break(line)
            || line.trim_start().starts_with('>')
            || is_kramdown_attr_line(line)
        {
            break;
        } else {
            content.push_str(line);
            content.push('\n');
            consumed += 1;
        }
    }

    if content.ends_with('\n') {
        content.pop();
    }
    (content, consumed)
}

/// Parse a GFM task list marker: `[ ]`, `[x]`, `[X]`.
/// Returns `(TaskState, content_start_offset)`.
///
/// The offset counts from the start of the (trimmed) line and points just
/// past the closing `]` of the marker.
pub fn parse_task_marker(line: &str) -> Option<(TaskState, usize)> {
    let t = line.trim_start();
    // Must be after a list marker like `- `, `* `, `+ `, `1. `
    let marker_end = if t.starts_with(['-', '*', '+']) {
        1
    } else {
        let dlen = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if dlen == 0 {
            return None;
        }
        dlen + 1 // digits + '.' or ')'
    };

    let after_marker = &t[marker_end..];
    let after_marker_trimmed = after_marker.trim_start();

    if after_marker_trimmed.starts_with("[ ]") {
        Some((
            TaskState::Unchecked,
            marker_end + (after_marker.len() - after_marker_trimmed.len()) + 3,
        ))
    } else if after_marker_trimmed.starts_with("[x]") || after_marker_trimmed.starts_with("[X]") {
        Some((
            TaskState::Checked,
            marker_end + (after_marker.len() - after_marker_trimmed.len()) + 3,
        ))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Link reference definition helpers
// ---------------------------------------------------------------------------

/// Strip a leading GFM task-list marker (`[ ]`, `[x]`, or `[X]`, plus one
/// optional following space) from list-item content collected by
/// [`collect_list_item`].
///
/// Returns the content unchanged when it does not start with a marker.
fn strip_task_marker(content: String) -> String {
    for marker in ["[ ]", "[x]", "[X]"] {
        if let Some(rest) = content.strip_prefix(marker) {
            return rest.strip_prefix(' ').unwrap_or(rest).to_string();
        }
    }
    content
}

#[cfg(test)]
mod tests {
    use super::{parse_task_marker, strip_task_marker};
    use crate::markdown::ast::tokens::token::TaskState;

    #[test]
    fn strip_task_marker_removes_marker_and_one_space() {
        assert_eq!(strip_task_marker("[ ] todo".into()), "todo");
        assert_eq!(strip_task_marker("[x] done".into()), "done");
        assert_eq!(strip_task_marker("[X]no-space".into()), "no-space");
        // No marker → unchanged.
        assert_eq!(strip_task_marker("plain".into()), "plain");
        // Only the leading marker counts.
        assert_eq!(strip_task_marker("text [ ] stays".into()), "text [ ] stays");
    }

    #[test]
    fn parse_task_marker_reports_state_and_offset() {
        let (state, off) = parse_task_marker("- [ ] todo").unwrap();
        assert_eq!(state, TaskState::Unchecked);
        assert_eq!(off, 5); // just past the `]`
        let (state, _) = parse_task_marker("1. [x] done").unwrap();
        assert_eq!(state, TaskState::Checked);
        assert!(parse_task_marker("- plain").is_none());
    }
}

/// Normalise a link label per CommonMark: trim, collapse internal whitespace
/// to single spaces, lowercase.
pub fn normalize_label(label: &str) -> String {
    let trimmed = label.trim();
    let mut result = String::with_capacity(trimmed.len());
    let mut prev_ws = false;
    for c in trimmed.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                result.push(' ');
                prev_ws = true;
            }
        } else {
            result.push(c.to_ascii_lowercase());
            prev_ws = false;
        }
    }
    result
}

/// Parse the URL portion of a link reference definition.
/// Returns `(url, remaining_text)`.
/// The URL may be wrapped in `<...>` or be bare (up to the first whitespace).
pub fn parse_link_url(rest: &str) -> (String, &str) {
    let rest = rest.trim_start();
    if rest.starts_with('<') {
        if let Some(close) = rest.find('>') {
            return (rest[1..close].to_string(), &rest[close + 1..]);
        }
    }
    // Bare URL: read until whitespace
    let end = rest
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    (rest[..end].to_string(), &rest[end..])
}

/// Parse an optional link title: `"..."`, `'...'`, or `(...)` .
/// Returns `(Some(title), consumed_len)` or `(None, 0)`.
pub fn parse_link_title(rest: &str) -> (Option<String>, usize) {
    let rest = rest.trim_start();
    if rest.is_empty() {
        return (None, 0);
    }
    let open = rest.chars().next().unwrap();
    if open != '"' && open != '\'' && open != '(' {
        return (None, 0);
    }
    let close = match open {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return (None, 0),
    };
    // Find the closing delimiter
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 1;
    while i < chars.len() {
        if chars[i] == close {
            if open == '(' {
                return (Some(chars[1..i].iter().collect()), i + 1);
            }
            return (Some(chars[1..i].iter().collect()), i + 1);
        }
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        i += 1;
    }
    (None, 0)
}

/// Extract the comment content from `<!-- ... -->`, stripping the markers
/// and any leading/trailing whitespace.
fn extract_comment_content(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed.strip_prefix("<!--") {
        if let Some(inner) = inner.strip_suffix("-->") {
            return inner.trim().to_string();
        }
        // No closing --> — return everything after <!--
        return inner.trim().to_string();
    }
    trimmed.to_string()
}
