use super::inline;
use crate::ast::options::AstOptions;
use crate::ast::tokens::span::Span;
use crate::ast::tokens::token::{Block, Table, TableCell, TableCellAlignment, TableRow};
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
    options: Option<&AstOptions>,
) -> Block {
    let header_cells: Vec<String> = split_table_row(header_line);
    let header = TableRow {
        cells: header_cells
            .iter()
            .map(|c| TableCell {
                children: inline::parse_inline(c, options),
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
                        children: inline::parse_inline(c, options),
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
        pos: Span::default(),
    }
}
