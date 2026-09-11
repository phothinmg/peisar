//! GFM-specific parsing helpers.
//!
//! Handles:
//! - **Tables** — pipe-delimited tables with an alignment row.
//! - **Task lists** — `- [x] item` / `- [ ] item` checkboxes.
//! - **Autolinks** — bare URL detection.
//! - **Strikethrough** — `~~text~~`.

use crate::ast::{Block, Inline, Table, TableCell, TableCellAlignment, TableRow, TaskState};

/// Check whether a line could be the start of a GFM table.
/// A table requires at least one pipe `|` on the current line and a
/// delimiter row (consisting of `-`, `:`, `|`, and whitespace) on the
/// next line.
pub fn is_table_start(lines: &[&str], pos: usize) -> bool {
    if pos + 1 >= lines.len() {
        return false;
    }
    let header = lines[pos];
    let delimiter = lines[pos + 1];
    header.contains('|') && is_table_delimiter(delimiter)
}

/// A table delimiter row looks like `| :--- | :--: | ---: |` —
/// cells containing only `-`, `:`, and whitespace, with at least one `-`.
pub fn is_table_delimiter(line: &str) -> bool {
    let stripped = line.trim();
    if stripped.is_empty() {
        return false;
    }
    // remove leading/trailing pipe
    let s = stripped.strip_prefix('|').unwrap_or(stripped);
    let s = s.strip_suffix('|').unwrap_or(s);

    let cells: Vec<&str> = s.split('|').collect();
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|c| {
        let t = c.trim();
        !t.is_empty() && t.chars().all(|ch| ch == '-' || ch == ':') && t.contains('-')
    })
}

/// Parse a delimiter row into column alignments.
pub fn parse_delimiter_alignments(line: &str) -> Vec<TableCellAlignment> {
    let stripped = line.trim();
    let s = stripped.strip_prefix('|').unwrap_or(stripped);
    let s = s.strip_suffix('|').unwrap_or(s);

    s.split('|')
        .map(|cell| {
            let t = cell.trim();
            let left = t.starts_with(':');
            let right = t.ends_with(':');
            match (left, right) {
                (true, true) => TableCellAlignment::Center,
                (true, false) => TableCellAlignment::Left,
                (false, true) => TableCellAlignment::Right,
                (false, false) => TableCellAlignment::Default,
            }
        })
        .collect()
}

/// Split a table row into raw cell strings (pipes removed, leading/trailing
/// whitespace trimmed).
pub fn split_table_row(line: &str) -> Vec<String> {
    let stripped = line.trim();
    let s = stripped.strip_prefix('|').unwrap_or(stripped);
    let s = s.strip_suffix('|').unwrap_or(s);
    s.split('|').map(|c| c.trim().to_string()).collect()
}

/// Build a [`Block::Table`] from lines `[header_line, delimiter_line, ...body_lines]`.
pub fn build_table(
    header_line: &str,
    _delimiter_line: &str,
    alignments: Vec<TableCellAlignment>,
    body_lines: &[&str],
) -> Block {
    let header_cells: Vec<String> = split_table_row(header_line);
    let header = TableRow {
        cells: header_cells
            .iter()
            .map(|c| TableCell {
                children: crate::parser::inline::parse_inline(c),
            })
            .collect(),
    };

    let rows: Vec<TableRow> = body_lines
        .iter()
        .map(|line| {
            let cells: Vec<String> = split_table_row(line);
            TableRow {
                cells: cells
                    .iter()
                    .map(|c| TableCell {
                        children: crate::parser::inline::parse_inline(c),
                    })
                    .collect(),
            }
        })
        .collect();

    Block::Table {
        table: Table {
            header,
            rows,
            alignments,
        },
        attrs: None,
        position: crate::ast::Span::default(),
    }
}

/// Parse a GFM task list marker: `[ ]`, `[x]`, `[X]`.
/// Returns `(TaskState, content_start_offset)`.
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

/// Check if a line starts with a task list marker.
pub fn has_task_marker(line: &str) -> bool {
    parse_task_marker(line).is_some()
}

/// Detect a bare URL for GFM autolink (www.example.com or https://example.com).
/// Returns `(url, end_index)` relative to `chars`.
pub fn match_autolink(chars: &[char], start: usize) -> Option<(String, usize)> {
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

/// Match GFM strikethrough `~~text~~`. Returns `(children, end_index)`.
pub fn match_strikethrough(
    chars: &[char],
    start: usize,
    ctx: &crate::parser::inline::InlineCtx,
) -> Option<(Inline, usize)> {
    if chars[start] != '~' || start + 1 >= chars.len() || chars[start + 1] != '~' {
        return None;
    }

    let after = &chars[start + 2..];
    let after_str: String = after.iter().collect();
    let close = after_str.find("~~")?;
    let inner: String = after_str[..close].to_string();
    let children = crate::parser::inline::parse_inline(&inner);
    let end = start + 2 + close + 2;

    Some((
        Inline::Strikethrough {
            children,
            position: crate::ast::Span::new(
                ctx.position(start, chars.len()),
                ctx.position(end, chars.len()),
            ),
        },
        end,
    ))
}
