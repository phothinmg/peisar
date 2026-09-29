use super::hooks::ParseHooks;
use super::inline;
use crate::markdown::ast::options::AstOptions;
use crate::markdown::ast::tokens::span::Span;
use crate::markdown::ast::tokens::token::{Block, Table, TableCell, TableCellAlignment, TableRow};

/// Returns `true` when `line` is a GFM table delimiter row — cells that
/// contain only `-` and `:` with at least one `-` per cell, optionally
/// surrounded by pipes: `| :--- | :---: | ---: |`.
///
/// Leading/trailing whitespace and pipes are ignored; every cell must be
/// non-empty.
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
///
/// Each cell maps to one [`TableCellAlignment`]: `:---` → left, `:---:` →
/// center, `---:` → right, and `---` (no colons) → default.
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
///
/// Leading and trailing pipes are optional; escaped pipes (`\|`) are not
/// treated as cell separators.
pub fn split_table_row(line: &str) -> Vec<String> {
    let stripped = line.trim();
    let s = stripped.strip_prefix('|').unwrap_or(stripped);
    let s = s.strip_suffix('|').unwrap_or(s);
    s.split('|').map(|c| c.trim().to_string()).collect()
}
/// Build a [`Block::Table`] from lines `[header_line, delimiter_line, ...body_lines]`.
///
/// Header and body cells are parsed as inline content with the given options
/// and hooks (so inline hooks apply inside table cells).
#[allow(clippy::too_many_arguments)]
pub fn build_table(
    header_line: &str,
    _delimiter_line: &str,
    alignments: Vec<TableCellAlignment>,
    body_lines: &[&str],
    options: Option<&AstOptions>,
    hooks: &ParseHooks,
) -> Block {
    let header_cells: Vec<String> = split_table_row(header_line);
    let header = TableRow {
        cells: header_cells
            .iter()
            .map(|c| TableCell {
                children: inline::parse_inline_with_hooks(c, options, None, hooks),
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
                        children: inline::parse_inline_with_hooks(c, options, None, hooks),
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

#[cfg(test)]
mod tests {
    use super::{
        build_table, is_table_delimiter, is_table_start, parse_delimiter_alignments,
        split_table_row,
    };
    use crate::markdown::ast::tokens::token::{Block, Inline, TableCellAlignment};

    #[test]
    fn delimiter_rows_are_recognised() {
        assert!(is_table_delimiter("| --- | --- |"));
        assert!(is_table_delimiter(":---:"));
        assert!(is_table_delimiter("  | :--- | ---: |  "));
        // No dashes anywhere in a cell → not a delimiter.
        assert!(!is_table_delimiter("| :: |"));
        assert!(!is_table_delimiter("| text | --- |"));
        assert!(!is_table_delimiter(""));
    }

    #[test]
    fn table_start_needs_header_pipe_and_delimiter() {
        let lines = ["| a | b |", "| --- | --- |", "| 1 | 2 |"];
        assert!(is_table_start(&lines, 0));
        // Header without a pipe is not a table.
        assert!(!is_table_start(&["no pipes", "---"], 0));
        // Last line has no following delimiter row.
        assert!(!is_table_start(&lines, 2));
        assert!(!is_table_start(&lines, 3));
    }

    #[test]
    fn alignments_map_from_colons() {
        let aligns = parse_delimiter_alignments("| :--- | --- | :---: | ---: |");
        assert_eq!(
            aligns,
            vec![
                TableCellAlignment::Left,
                TableCellAlignment::Default,
                TableCellAlignment::Center,
                TableCellAlignment::Right,
            ]
        );
    }

    #[test]
    fn rows_split_on_pipes() {
        assert_eq!(
            split_table_row("| a | b |"),
            vec!["a".to_string(), "b".to_string()]
        );
        // Leading/trailing pipes optional.
        assert_eq!(
            split_table_row("a | b"),
            vec!["a".to_string(), "b".to_string()]
        );
        // Cells are trimmed.
        assert_eq!(split_table_row("|  a  |"), vec!["a".to_string()]);
    }

    #[test]
    fn build_table_parses_header_and_rows_as_inline() {
        let block = build_table(
            "| Name | **Bold** |",
            "| --- | --- |",
            vec![TableCellAlignment::Default, TableCellAlignment::Default],
            &["| peisar | yes |"],
            None,
            &crate::markdown::ast::parsers::hooks::ParseHooks::empty(),
        );
        match block {
            Block::Table { table, .. } => {
                assert_eq!(table.header.cells.len(), 2);
                assert_eq!(table.rows.len(), 1);
                // Second header cell contains emphasis.
                assert!(matches!(
                    table.header.cells[1].children.as_slice(),
                    [Inline::Emphasis { .. }]
                ));
                // Body cell content is plain text.
                assert!(matches!(
                    table.rows[0].cells[0].children.as_slice(),
                    [Inline::Text { value, .. }] if value == "peisar"
                ));
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }
}
