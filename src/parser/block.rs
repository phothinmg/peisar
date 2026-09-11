//! Block-level Markdown parser.
//!
//! Handles: headings (with Kramdown attrs), fenced/indented code blocks,
//! block quotes, lists (with GFM task lists), GFM tables, thematic breaks,
//! HTML blocks, and paragraphs.

use super::gfm;
use super::kramdown;
use crate::ast::{Block, KramdownAttributes, ListItem, Position, Span, TaskState};
use crate::config::ParseOptions;

/// Byte offset of the start of each line (line 0 starts at offset 0).
pub fn compute_line_starts(input: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in input.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

pub struct ParserState<'a> {
    pub input: &'a str,
    pub lines: &'a [&'a str],
    /// Byte offset of the start of each line.
    pub line_starts: &'a [usize],
    pub pos: usize,
    pub opts: &'a ParseOptions,
}

impl<'a> ParserState<'a> {
    pub fn new(
        input: &'a str,
        lines: &'a [&'a str],
        line_starts: &'a [usize],
        opts: &'a ParseOptions,
    ) -> Self {
        Self {
            input,
            lines,
            line_starts,
            pos: 0,
            opts,
        }
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.lines.len()
    }

    pub fn current(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    pub fn advance(&mut self) {
        self.pos += 1;
    }

    /// The [`Position`] at the start of logical line `line` (0-based).
    pub fn position_at(&self, line: usize) -> Position {
        let line = line.min(self.line_starts.len().saturating_sub(1));
        let offset = self.line_starts[line].min(self.input.len());
        Position {
            line,
            column: 0,
            offset,
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

    pub fn parse_block(&mut self) -> Option<Block> {
        let line = self.current()?;
        let start_line = self.pos;

        // GFM table
        if self.opts.gfm && gfm::is_table_start(self.lines, self.pos) {
            return Some(self.parse_table(start_line));
        }

        // Thematic break
        if is_thematic_break(line) {
            self.advance();
            let attrs = self.try_trailing_attrs();
            return Some(Block::ThematicBreak {
                attrs,
                position: self.span(start_line, self.pos),
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
            let html = self.collect_html_block();
            return Some(Block::HtmlBlock {
                html,
                position: self.span(start_line, self.pos),
            });
        }

        // Default: paragraph
        Some(self.parse_paragraph())
    }

    /// After parsing a block, check if the next line is a Kramdown
    /// attribute block `{:...}` that applies to this block.  Blank lines
    /// between the block and the attribute marker are allowed.
    fn try_trailing_attrs(&mut self) -> Option<KramdownAttributes> {
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
                if let Some((attrs, _)) = kramdown::parse_kramdown_attrs(trimmed) {
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
        let alignments = gfm::parse_delimiter_alignments(delimiter_line);

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

        let mut block = gfm::build_table(header_line, delimiter_line, alignments, &body_lines);
        // Fix position + trailing attrs
        if let Block::Table {
            attrs, position, ..
        } = &mut block
        {
            *position = self.span(start_line, self.pos);
            *attrs = self.try_trailing_attrs();
        }
        block
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

        let children = super::inline::parse_inline_with(&heading_text, self.opts);
        Some(Block::Heading {
            level: hashes as u8,
            children,
            attrs,
            position: self.span(start_line, self.pos),
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
            position: self.span(start_line, self.pos),
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
            position: self.span(start_line, self.pos),
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
        let inner_doc = super::parse_with(&inner.join("\n"), self.opts);
        let attrs = self.try_trailing_attrs();
        Block::BlockQuote {
            children: inner_doc.children,
            attrs,
            position: self.span(start_line, self.pos),
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

                // GFM task list detection
                let task = if self.opts.gfm {
                    gfm::parse_task_marker(line).map(|(state, _)| state)
                } else {
                    None
                };

                let inner = super::parse_with(&content, self.opts);
                items.push(ListItem {
                    children: inner.children,
                    task,
                    position: self.span(item_start, item_end),
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
            position: self.span(start_line, self.pos),
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
            {
                break;
            }
            // GFM table start
            if self.opts.gfm && gfm::is_table_start(self.lines, self.pos) {
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

        let children = super::inline::parse_inline_with(&raw, self.opts);
        Block::Paragraph {
            children,
            attrs,
            position: self.span(start_line, self.pos),
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

pub fn is_thematic_break(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return false;
    }
    let c = t.chars().next().unwrap();
    (c == '-' || c == '*' || c == '_') && t.chars().all(|ch| ch == c) && t.len() >= 3
}

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

// Suppress unused warning for TaskState re-export used in parse_list
#[allow(unused_imports)]
use TaskState as _TaskState;
