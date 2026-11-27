//! Block-level Markdown parser.
//!
//! This module implements the block-level parser that converts lines of
//! Markdown source text into [`Block`] nodes.  It supports:
//!
//! - ATX headings (`#` … `######`) and Setext headings
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

use super::attrs::parse_attrs;
use super::inline::{
    LinkRefMap, decode_char_references, parse_inline_with_hooks, unescape_backslash_punctuation,
};
use super::plugin::{BlockParserContext, ParserHooks, finalize_hook_block};
use super::table::{build_table, is_table_start, parse_delimiter_alignments};
use crate::ast::{
    AstOptions,
    span::{Position, Span},
    tokens::{Attributes, Block, LinkReferenceDefinition, ListItem, TaskState},
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

fn fence_opener(line: &str) -> Option<(char, usize)> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return None;
    }
    let trimmed = &line[indent..];
    let fence_char = trimmed.chars().next()?;
    if fence_char != '`' && fence_char != '~' {
        return None;
    }
    let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
    if fence_len < 3 {
        return None;
    }
    let info = trimmed[fence_len..].trim();
    if fence_char == '`' && info.contains('`') {
        return None;
    }
    Some((fence_char, fence_len))
}

fn is_fence_closer(line: &str, fence_char: char, fence_len: usize) -> bool {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return false;
    }
    let candidate = &line[indent..];
    let candidate_len = candidate.chars().take_while(|&c| c == fence_char).count();
    candidate_len >= fence_len && candidate[candidate_len..].trim().is_empty()
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
        if let Some(kind) = html_block_kind(self.lines, self.pos) {
            if matches!(kind, HtmlBlockKind::Comment) {
                return Some(self.parse_html_comment(start_line));
            }
            let html = self.collect_html_block(kind);
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
        let start_line = self.pos;
        let (definition, lines_consumed) =
            parse_link_ref_def_lines(self.lines, start_line, self.line_starts, self.input)?;
        for _ in 0..lines_consumed {
            self.advance();
        }

        Some(Block::LinkReferenceDefinition {
            label: definition.label,
            url: definition.url,
            title: definition.title,
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
            content.push_str(line);
            content.push('\n');
            if let Some(close) = line.find("-->") {
                found_close = true;
                let trailing = &line[close + 3..];
                let indented = self.pos == start_line && leading_indent_columns(line).0 > 0;
                self.advance();
                if indented || !trailing.is_empty() {
                    return Block::HtmlBlock {
                        html: content.trim_end_matches('\n').to_string(),
                        attrs: None,
                        pos: self.span(start_line, self.pos),
                    };
                }
                break;
            }
            self.advance();
        }

        if !found_close {
            return Block::HtmlBlock {
                html: content.trim_end_matches('\n').to_string(),
                attrs: None,
                pos: self.span(start_line, self.pos),
            };
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
        let (level, rest) = atx_heading(line)?;
        let text = atx_heading_text(rest);
        self.advance();

        // Kramdown: block attribute marker on the next line:
        //   # Title
        //   {: #id .cls key="val"}
        let attrs = self.try_trailing_attrs();

        let children = parse_inline_with_hooks(&text, Some(self.opts), Some(self.refs), self.hooks);
        Some(Block::Heading {
            level,
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
        let indent = line.bytes().take_while(|&b| b == b' ').count();
        if indent > 3 {
            return None;
        }
        let trimmed = &line[indent..];
        let fence_char = trimmed.chars().next()?;
        if fence_char != '`' && fence_char != '~' {
            return None;
        }
        let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
        if fence_len < 3 {
            return None;
        }
        let info = trimmed[fence_len..].trim();
        if fence_char == '`' && info.contains('`') {
            return None;
        }
        let info_word = info.split_whitespace().next().unwrap_or("");
        let lang = if info_word.is_empty() {
            None
        } else {
            Some(decode_char_references(&unescape_backslash_punctuation(
                info_word,
            )))
        };

        let start_line = self.pos;
        self.advance();
        let mut code = String::new();
        while let Some(l) = self.current() {
            let line_indent = l.bytes().take_while(|&b| b == b' ').count();
            let candidate = &l[line_indent.min(3)..];
            let candidate_fence_len = candidate.chars().take_while(|&c| c == fence_char).count();
            if line_indent <= 3
                && candidate_fence_len >= fence_len
                && candidate[candidate_fence_len..].trim().is_empty()
            {
                self.advance();
                break;
            }
            code.push_str(strip_fence_indent(l, indent));
            code.push('\n');
            self.advance();
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
        if leading_indent_columns(line).0 < 4 {
            return None;
        }

        let start_line = self.pos;
        let mut code = String::new();
        while let Some(l) = self.current() {
            if l.trim().is_empty() {
                let mut next = self.pos;
                while next < self.lines.len() && self.lines[next].trim().is_empty() {
                    next += 1;
                }
                if next < self.lines.len() && leading_indent_columns(self.lines[next]).0 >= 4 {
                    while self.pos < next {
                        code.push_str(&strip_indent_columns(self.lines[self.pos], 4));
                        code.push('\n');
                        self.advance();
                    }
                    continue;
                } else {
                    break;
                }
            }
            if leading_indent_columns(l).0 < 4 {
                break;
            }
            code.push_str(&strip_indent_columns(l, 4));
            code.push('\n');
            self.advance();
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
        let mut inner: Vec<String> = Vec::new();
        let mut fence: Option<(char, usize)> = None;
        while let Some(line) = self.current() {
            if let Some(content) = strip_block_quote_prefix(line) {
                if let Some((fence_char, fence_len)) = fence {
                    if is_fence_closer(&content, fence_char, fence_len) {
                        fence = None;
                    }
                } else if let Some(opening) = fence_opener(&content) {
                    fence = Some(opening);
                }
                inner.push(content);
                self.advance();
            } else {
                if line.trim().is_empty() {
                    break;
                }

                let indent = leading_indent_columns(line).0;
                let continuation = line.trim_start_matches(' ');
                let previous_is_code = inner
                    .last()
                    .is_some_and(|last| leading_indent_columns(last).0 >= 4);
                let can_lazy_continue = fence.is_none()
                    && !previous_is_code
                    && inner.last().is_some_and(|last| !last.trim().is_empty())
                    && (indent >= 4 || !is_list_marker(line))
                    && !is_thematic_break(line)
                    && !is_heading(line)
                    && !is_fence(line)
                    && !html_block_interrupts_paragraph(self.lines, self.pos)
                    && !line.trim_start().starts_with('>')
                    && !(self.opts.kramdown && is_kramdown_attr_line(line));
                if !can_lazy_continue {
                    break;
                }
                if setext_underline_level(continuation).is_some() {
                    inner.push(format!("\\{continuation}"));
                } else if can_interrupt_paragraph_with_list(continuation) {
                    inner.push(format!("\\{continuation}"));
                } else {
                    inner.push(continuation.to_string());
                }
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
        let first_marker = list_marker_parts(first).expect("list marker was checked");
        let ordered = matches!(first_marker.kind, ListMarkerKind::Ordered(_));
        let start = ordered.then_some(first_marker.number).filter(|&n| n != 1);
        let mut tight = true;

        let mut items: Vec<ListItem> = Vec::new();

        while let Some(line) = self.current() {
            if line.trim().is_empty() {
                let mut peek = self.pos + 1;
                while peek < self.lines.len() && self.lines[peek].trim().is_empty() {
                    peek += 1;
                }
                if peek < self.lines.len()
                    && list_marker_parts(self.lines[peek])
                        .is_some_and(|marker| marker.kind == first_marker.kind)
                {
                    self.pos = peek;
                    tight = false;
                    continue;
                } else {
                    break;
                }
            }

            if is_thematic_break(line) {
                break;
            }

            if list_marker_parts(line).is_some_and(|marker| marker.kind == first_marker.kind) {
                let item_start = self.pos;
                let (content, consumed, blank_before_next) =
                    collect_list_item(self.lines, self.pos, first_marker.kind);
                self.pos += consumed;
                let item_end = self.pos;
                if blank_before_next {
                    tight = false;
                }

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
                if blocks_have_blank_lines(&inner.children) {
                    tight = false;
                }
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
            start,
            tight,
            items,
            attrs,
            pos: self.span(start_line, self.pos),
        }
    }

    // -----------------------------------------------------------------------
    // HTML block
    // -----------------------------------------------------------------------

    fn collect_html_block(&mut self, kind: HtmlBlockKind) -> String {
        let mut html = String::new();
        while let Some(line) = self.current() {
            if matches!(kind, HtmlBlockKind::BlockTag | HtmlBlockKind::CompleteTag)
                && line.trim().is_empty()
            {
                break;
            }
            html.push_str(line);
            html.push('\n');
            self.advance();
            if html_block_terminator_reached(&kind, line) {
                break;
            }
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
            let indented_continuation =
                !text_lines.is_empty() && leading_indent_columns(line).0 >= 4;
            if !indented_continuation
                && !text_lines.is_empty()
                && let Some(level) = setext_underline_level(line)
            {
                self.advance();
                let heading_text = text_lines.join("\n");
                let heading_text = heading_text.trim();
                let attrs = self.try_trailing_attrs();
                let children = parse_inline_with_hooks(
                    heading_text,
                    Some(self.opts),
                    Some(self.refs),
                    self.hooks,
                );
                return Block::Heading {
                    level,
                    children,
                    attrs,
                    pos: self.span(start_line, self.pos),
                };
            }
            // Stop if a new block construct begins.
            if !text_lines.is_empty()
                && !indented_continuation
                && (is_thematic_break(line)
                    || is_heading(line)
                    || is_fence(line)
                    || line.trim_start().starts_with('>')
                    || can_interrupt_paragraph_with_list(line)
                    || html_block_interrupts_paragraph(self.lines, self.pos)
                    || is_link_ref_def(line))
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
            let line = line.trim_start_matches(' ');
            let line = if !text_lines.is_empty() {
                line.strip_prefix('\t').unwrap_or(line)
            } else {
                line
            };
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

fn leading_indent_columns(line: &str) -> (usize, usize) {
    let mut columns = 0;
    let mut byte_end = 0;
    for (index, c) in line.char_indices() {
        match c {
            ' ' => columns += 1,
            '\t' => columns += 4 - columns % 4,
            _ => break,
        }
        byte_end = index + c.len_utf8();
    }
    (columns, byte_end)
}

fn strip_fence_indent(line: &str, indent: usize) -> &str {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    &line[spaces.min(indent)..]
}

fn strip_indent_columns(line: &str, target: usize) -> String {
    let mut columns = 0;
    let mut output = String::new();
    for (index, c) in line.char_indices() {
        let next = match c {
            ' ' => columns + 1,
            '\t' => columns + 4 - columns % 4,
            _ => {
                output.push_str(&line[index..]);
                return output;
            }
        };
        if columns >= target {
            output.push_str(&" ".repeat(next - columns));
        } else if next > target {
            output.push_str(&" ".repeat(next - target));
        }
        columns = next;
    }
    output
}

fn expand_leading_whitespace(line: &str, start_column: usize) -> (String, usize, usize) {
    let mut expanded = String::new();
    let mut columns = start_column;
    let mut byte_end = 0;
    for (index, c) in line.char_indices() {
        let width = match c {
            ' ' => 1,
            '\t' => 4 - columns % 4,
            _ => break,
        };
        expanded.push_str(&" ".repeat(width));
        columns += width;
        byte_end = index + c.len_utf8();
    }
    (expanded, byte_end, columns - start_column)
}

fn strip_block_quote_prefix(line: &str) -> Option<String> {
    let (indent, start) = leading_indent_columns(line);
    if indent > 3 {
        return None;
    }
    let rest = line[start..].strip_prefix('>')?;
    let (_, whitespace_bytes, whitespace_columns) = expand_leading_whitespace(rest, indent + 1);
    if whitespace_bytes == 0 {
        return Some(rest.to_string());
    }

    let mut content = " ".repeat(whitespace_columns.saturating_sub(1));
    content.push_str(&rest[whitespace_bytes..]);
    Some(content)
}

fn is_heading(line: &str) -> bool {
    atx_heading(line).is_some()
}

fn atx_heading(line: &str) -> Option<(u8, &str)> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return None;
    }

    let content = &line[indent..];
    let hashes = content.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }

    let rest = &content[hashes..];
    if rest.chars().next().is_some_and(|c| c != ' ' && c != '\t') {
        return None;
    }
    Some((hashes as u8, rest))
}

fn atx_heading_text(rest: &str) -> String {
    let text = rest.trim();
    let closing_hashes = text.chars().rev().take_while(|&c| c == '#').count();
    if closing_hashes > 0 {
        let prefix = &text[..text.len() - closing_hashes];
        if matches!(prefix.chars().last(), Some(' ' | '\t'))
            || (prefix.is_empty() && matches!(rest.chars().next(), Some(' ' | '\t')))
        {
            return prefix.trim_end().to_string();
        }
    }
    text.to_string()
}

fn setext_underline_level(line: &str) -> Option<u8> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return None;
    }

    let underline = line[indent..].trim_end_matches([' ', '\t']);
    let marker = underline.chars().next()?;
    if (marker != '=' && marker != '-') || !underline.chars().all(|c| c == marker) {
        return None;
    }
    Some(if marker == '=' { 1 } else { 2 })
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
    let mut indent_columns = 0;
    let mut content_start = 0;
    for (index, c) in line.char_indices() {
        match c {
            ' ' => indent_columns += 1,
            '\t' => indent_columns += 4 - indent_columns % 4,
            _ => break,
        }
        if indent_columns > 3 {
            return false;
        }
        content_start = index + c.len_utf8();
    }

    let t: String = line[content_start..]
        .chars()
        .filter(|c| *c != ' ' && *c != '\t')
        .collect();
    if t.is_empty() {
        return false;
    }
    let c = t.chars().next().unwrap();
    (c == '-' || c == '*' || c == '_') && t.chars().all(|ch| ch == c) && t.len() >= 3
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HtmlBlockKind {
    Comment,
    ProcessingInstruction,
    Declaration,
    Cdata,
    RawTag(&'static str),
    BlockTag,
    CompleteTag,
}

struct HtmlTag {
    end_line: usize,
    end_column: usize,
}

fn html_block_kind(lines: &[&str], start: usize) -> Option<HtmlBlockKind> {
    let line = *lines.get(start)?;
    let (indent, byte_start) = leading_indent_columns(line);
    if indent > 3 {
        return None;
    }

    let text = &line[byte_start..];
    if text.starts_with("<!--") {
        return Some(HtmlBlockKind::Comment);
    }
    if text.starts_with("<?") {
        return Some(HtmlBlockKind::ProcessingInstruction);
    }
    if text.starts_with("<![CDATA[") {
        return Some(HtmlBlockKind::Cdata);
    }
    if text.starts_with("<!") && text.as_bytes().get(2).is_some_and(u8::is_ascii_uppercase) {
        return Some(HtmlBlockKind::Declaration);
    }

    if let Some((name, closing)) = html_tag_name(text) {
        let name = name.to_ascii_lowercase();
        if !closing && matches!(name.as_str(), "script" | "pre" | "style" | "textarea") {
            return Some(match name.as_str() {
                "script" => HtmlBlockKind::RawTag("script"),
                "pre" => HtmlBlockKind::RawTag("pre"),
                "style" => HtmlBlockKind::RawTag("style"),
                _ => HtmlBlockKind::RawTag("textarea"),
            });
        }
        if is_block_level_tag(&name) {
            return Some(HtmlBlockKind::BlockTag);
        }
    }

    let tag = scan_html_tag(lines, start, byte_start)?;
    if tag.end_line == start && lines[start][tag.end_column..].trim().is_empty() {
        return Some(HtmlBlockKind::CompleteTag);
    }
    None
}

fn html_block_interrupts_paragraph(lines: &[&str], start: usize) -> bool {
    html_block_kind(lines, start).is_some_and(|kind| kind != HtmlBlockKind::CompleteTag)
}

fn html_tag_name(text: &str) -> Option<(String, bool)> {
    let mut tag = text.strip_prefix('<')?;
    let closing = if let Some(rest) = tag.strip_prefix('/') {
        tag = rest;
        true
    } else {
        false
    };
    let name_end = tag
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .unwrap_or(tag.len());
    if name_end == 0 || !tag.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    if !matches!(
        tag[name_end..].chars().next(),
        None | Some(' ' | '\t' | '/' | '>')
    ) {
        return None;
    }
    Some((tag[..name_end].to_string(), closing))
}

fn scan_html_tag(lines: &[&str], start: usize, first_column: usize) -> Option<HtmlTag> {
    let mut source = lines.get(start)?.get(first_column..)?.to_string();
    for line in &lines[start + 1..] {
        source.push('\n');
        source.push_str(line);
    }
    let chars: Vec<char> = source.chars().collect();
    let end = super::inline::match_inline_html(&chars, 0)?;
    let end_byte = chars
        .get(end)
        .and_then(|_| source.char_indices().nth(end).map(|(byte, _)| byte))
        .unwrap_or(source.len());
    let prefix = &source[..end_byte];
    let line_offset = prefix.bytes().filter(|&byte| byte == b'\n').count();
    if line_offset > 0
        && lines[start + 1..=start + line_offset]
            .iter()
            .any(|line| line.trim().is_empty())
    {
        return None;
    }
    let end_column = prefix
        .rfind('\n')
        .map_or(first_column + end_byte, |newline| end_byte - newline - 1);
    Some(HtmlTag {
        end_line: start + line_offset,
        end_column,
    })
}

fn is_block_level_tag(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "address"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "blockquote"
            | "body"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hr"
            | "html"
            | "iframe"
            | "legend"
            | "li"
            | "link"
            | "main"
            | "menu"
            | "menuitem"
            | "meta"
            | "nav"
            | "noframes"
            | "ol"
            | "optgroup"
            | "option"
            | "p"
            | "param"
            | "search"
            | "section"
            | "source"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
    )
}

fn html_block_terminator_reached(kind: &HtmlBlockKind, line: &str) -> bool {
    match kind {
        HtmlBlockKind::Comment => line.contains("-->"),
        HtmlBlockKind::ProcessingInstruction => line.contains("?>"),
        HtmlBlockKind::Declaration => line.contains('>'),
        HtmlBlockKind::Cdata => line.contains("]]>"),
        HtmlBlockKind::RawTag(tag) => {
            let lower = line.to_ascii_lowercase();
            let closing = format!("</{tag}");
            lower.match_indices(&closing).any(|(start, _)| {
                matches!(
                    lower[start + closing.len()..].chars().next(),
                    Some('>' | ' ' | '\t' | '/')
                )
            })
        }
        HtmlBlockKind::BlockTag | HtmlBlockKind::CompleteTag => false,
    }
}

/// Returns `true` if the line looks like a link reference definition:
/// `[label]: url ...`
///
/// This is a lightweight check — it only verifies the `[...]` `:` prefix.
/// Use [`parse_link_ref_def_line`] for full parsing.
pub fn is_link_ref_def(line: &str) -> bool {
    link_ref_source_lines(&[line], 0).is_some()
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
    let start_offset = line_starts.get(line_idx).copied().unwrap_or(0);
    let (mut definition, _) = parse_link_ref_def_lines(&[line], 0, &[start_offset], input)?;
    definition.pos.start.line = line_idx as u32;
    definition.pos.end.line = line_idx as u32;
    Some(definition)
}

/// Parse a link reference definition that may have a multiline label.
pub fn parse_link_ref_def_lines(
    lines: &[&str],
    line_idx: usize,
    line_starts: &[usize],
    input: &str,
) -> Option<(LinkReferenceDefinition, usize)> {
    let (source, lines_consumed) = link_ref_source_lines(lines, line_idx)?;
    let trimmed = source.trim_start();
    let bracket_close = link_label_close(trimmed)?;
    let label_raw = &trimmed[1..bracket_close];
    if label_raw.trim().is_empty() || label_raw.chars().count() > 999 {
        return None;
    }

    let after_bracket = &trimmed[bracket_close + 1..];
    let after_colon = &after_bracket.trim_start()[1..];
    let (url, after_url) = parse_link_url(after_colon.trim_start());
    if url.is_empty() {
        return None;
    }
    let url = decode_char_references(&unescape_backslash_punctuation(&url));
    let title = parse_link_title(after_url.trim())
        .0
        .map(|value| decode_char_references(&unescape_backslash_punctuation(&value)));

    let start_offset = line_starts
        .get(line_idx)
        .copied()
        .unwrap_or(0)
        .min(input.len());
    let last_line_idx = line_idx + lines_consumed - 1;
    let end_offset = line_starts
        .get(last_line_idx)
        .copied()
        .unwrap_or(start_offset)
        .saturating_add(lines[last_line_idx].len())
        .min(input.len());
    let pos = Span::new(
        Position {
            line: line_idx as u32,
            column: 0,
            offset: start_offset as u32,
        },
        Position {
            line: last_line_idx as u32,
            column: lines[last_line_idx].chars().count() as u32,
            offset: end_offset as u32,
        },
    );
    Some((
        LinkReferenceDefinition {
            label: normalize_label(label_raw),
            url,
            title,
            pos,
        },
        lines_consumed,
    ))
}

fn link_ref_source_lines(lines: &[&str], start: usize) -> Option<(String, usize)> {
    let first = *lines.get(start)?;
    let mut source = first.trim_start().to_string();
    if !source.starts_with('[') {
        return None;
    }
    let mut consumed = 1;
    loop {
        if let Some(close) = link_label_close(&source) {
            if source[close + 1..].trim_start().starts_with(':') {
                return Some((source, consumed));
            }
            return None;
        }
        let next = *lines.get(start + consumed)?;
        if next.trim().is_empty() || leading_indent_columns(next).0 > 3 {
            return None;
        }
        source.push(' ');
        source.push_str(next.trim_start());
        consumed += 1;
    }
}

fn link_label_close(source: &str) -> Option<usize> {
    let mut chars = source.char_indices().peekable();
    if chars.next()?.1 != '[' {
        return None;
    }
    while let Some((index, c)) = chars.next() {
        if c == '\\'
            && chars
                .peek()
                .is_some_and(|(_, next)| next.is_ascii_punctuation())
        {
            chars.next();
        } else if c == '[' {
            return None;
        } else if c == ']' {
            return Some(index);
        }
    }
    None
}

/// Returns `true` if the line begins with a CommonMark list marker.
pub fn is_list_marker(line: &str) -> bool {
    list_marker_parts(line).is_some()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListMarkerKind {
    Bullet(char),
    Ordered(char),
}

#[derive(Clone, Copy)]
struct ListMarker {
    indent: usize,
    marker_end: usize,
    marker_width: usize,
    kind: ListMarkerKind,
    number: u32,
}

fn list_marker_parts(line: &str) -> Option<ListMarker> {
    let (indent, start) = leading_indent_columns(line);
    if indent > 3 {
        return None;
    }

    let rest = &line[start..];
    let first = *rest.as_bytes().first()?;
    let (marker_width, kind, number) = if matches!(first, b'-' | b'*' | b'+') {
        (1, ListMarkerKind::Bullet(first as char), 1)
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let delimiter = *rest.as_bytes().get(digits)?;
        if digits == 0 || digits > 9 || !matches!(delimiter, b'.' | b')') {
            return None;
        }
        let number = rest[..digits].parse().ok()?;
        (
            digits + 1,
            ListMarkerKind::Ordered(delimiter as char),
            number,
        )
    };

    let after_marker = rest.get(marker_width..)?;
    if after_marker
        .chars()
        .next()
        .is_some_and(|c| c != ' ' && c != '\t')
    {
        return None;
    }
    Some(ListMarker {
        indent,
        marker_end: start + marker_width,
        marker_width,
        kind,
        number,
    })
}

fn can_interrupt_paragraph_with_list(line: &str) -> bool {
    let Some(marker) = list_marker_parts(line) else {
        return false;
    };
    let rest = &line[marker.marker_end..];
    if rest.trim().is_empty() {
        return false;
    }
    match marker.kind {
        ListMarkerKind::Bullet(_) => true,
        ListMarkerKind::Ordered(_) => marker.number == 1,
    }
}

/// Collect a single list item starting at `start`.
fn collect_list_item(
    lines: &[&str],
    start: usize,
    list_kind: ListMarkerKind,
) -> (String, usize, bool) {
    let first = lines[start];
    let marker = list_marker_parts(first).expect("list marker was checked");
    let marker_column = marker.indent + marker.marker_width;
    let after_marker = &first[marker.marker_end..];
    let (expanded_whitespace, whitespace_bytes, whitespace_columns) =
        expand_leading_whitespace(after_marker, marker_column);
    let (content_indent, first_content) = if whitespace_bytes == 0 {
        (marker_column + 1, after_marker.to_string())
    } else if whitespace_columns <= 4 {
        (
            marker_column + whitespace_columns,
            after_marker[whitespace_bytes..].to_string(),
        )
    } else {
        let mut content = expanded_whitespace[1..].to_string();
        content.push_str(&after_marker[whitespace_bytes..]);
        (marker_column + 1, content)
    };

    let mut content = String::new();
    if first_content.is_empty() {
        content.push('\n');
    } else {
        content.push_str(&first_content);
        content.push('\n');
    }

    let mut consumed = 1;
    let mut pending_blank_lines = 0;

    for i in (start + 1)..lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            continue;
        }

        let leading = leading_indent_columns(line).0;
        if leading < content_indent
            && list_marker_parts(line).is_some_and(|next| next.kind == list_kind)
        {
            consumed += pending_blank_lines;
            return (
                content.trim_end_matches('\n').to_string(),
                consumed,
                pending_blank_lines > 0,
            );
        }

        if leading >= content_indent {
            if pending_blank_lines > 0 && content.trim().is_empty() {
                break;
            }
            for _ in 0..pending_blank_lines {
                content.push('\n');
            }
            consumed += pending_blank_lines;
            pending_blank_lines = 0;
            content.push_str(&strip_indent_columns(line, content_indent));
            content.push('\n');
            consumed += 1;
        } else if pending_blank_lines > 0
            || is_thematic_break(line)
            || is_list_marker(line)
            || is_heading(line)
            || is_fence(line)
            || html_block_interrupts_paragraph(lines, i)
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
    (content, consumed, false)
}

fn blocks_have_blank_lines(blocks: &[Block]) -> bool {
    blocks
        .windows(2)
        .any(|pair| block_end_line(&pair[0]) < block_start_line(&pair[1]))
}

fn block_start_line(block: &Block) -> u32 {
    match block {
        Block::Heading { pos, .. }
        | Block::Paragraph { pos, .. }
        | Block::CodeBlock { pos, .. }
        | Block::BlockQuote { pos, .. }
        | Block::List { pos, .. }
        | Block::ThematicBreak { pos }
        | Block::HtmlBlock { pos, .. }
        | Block::Table { pos, .. }
        | Block::LinkReferenceDefinition { pos, .. }
        | Block::Comment { pos, .. } => pos.start.line,
    }
}

fn block_end_line(block: &Block) -> u32 {
    match block {
        Block::Heading { pos, .. }
        | Block::Paragraph { pos, .. }
        | Block::CodeBlock { pos, .. }
        | Block::BlockQuote { pos, .. }
        | Block::List { pos, .. }
        | Block::ThematicBreak { pos }
        | Block::HtmlBlock { pos, .. }
        | Block::Table { pos, .. }
        | Block::LinkReferenceDefinition { pos, .. }
        | Block::Comment { pos, .. } => pos.end.line,
    }
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
    use crate::ast::tokens::TaskState;

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
            match c {
                '\u{00DF}' | '\u{1E9E}' => result.push_str("ss"),
                '\u{03C2}' => result.push('\u{03C3}'),
                _ => result.extend(c.to_lowercase()),
            }
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
