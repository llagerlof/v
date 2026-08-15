use std::path::Path;

use unicode_width::UnicodeWidthStr;

use crate::wrap;

/// Smallest content width a column is allowed to shrink to when a table is
/// wider than the available space.
const MIN_COLUMN_WIDTH: usize = 3;

/// Extensions rendered as markdown.
const MARKDOWN_EXTENSIONS: [&str; 5] = ["md", "markdown", "mdown", "mkd", "mdx"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug)]
struct Table {
    indent: String,
    aligns: Vec<Alignment>,
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    /// Original markdown lines, kept as a fallback when the table cannot fit.
    source: Vec<String>,
    /// Index of the first line after the table.
    end: usize,
}

/// Return true when the path looks like a markdown document.
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .is_some_and(|ext| MARKDOWN_EXTENSIONS.contains(&ext.as_str()))
}

/// Rewrite GitHub-flavored markdown tables as ASCII grid tables that fit `width`.
pub fn format_tables(content: &str, width: usize) -> String {
    if !content.contains('|') {
        return content.to_string();
    }

    let ends_with_newline = content.ends_with('\n');
    let mut lines: Vec<&str> = content.split('\n').collect();
    if ends_with_newline {
        lines.pop();
    }

    let mut output: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    let mut index = 0;

    while index < lines.len() {
        let raw = lines[index];

        if is_fence(strip_cr(raw)) {
            in_fence = !in_fence;
            output.push(raw.to_string());
            index += 1;
            continue;
        }

        if !in_fence
            && let Some(table) = parse_table(&lines, index)
        {
            index = table.end;
            output.extend(render_table(&table, width));
            continue;
        }

        output.push(raw.to_string());
        index += 1;
    }

    let mut result = output.join("\n");
    if ends_with_newline {
        result.push('\n');
    }
    result
}

fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Leading indent of a table line, or `None` when the line is indented enough
/// to be a code block.
fn leading_indent(line: &str) -> Option<&str> {
    let end = line
        .char_indices()
        .find(|(_, ch)| *ch != ' ')
        .map(|(index, _)| index)
        .unwrap_or(line.len());
    if end > 3 { None } else { Some(&line[..end]) }
}

/// Split a table row into trimmed cells, honoring `\|` escapes.
fn split_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }

    let mut inner = trimmed;
    if let Some(rest) = inner.strip_prefix('|') {
        inner = rest;
    }
    if inner.ends_with('|') && !inner.ends_with("\\|") {
        inner = &inner[..inner.len() - 1];
    }

    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                current.push('|');
            }
            '|' => {
                cells.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    cells.push(current.trim().to_string());

    Some(cells)
}

/// Parse a delimiter row (`| --- | :---: |`) into per-column alignments.
fn parse_alignments(line: &str) -> Option<Vec<Alignment>> {
    let cells = split_row(line)?;
    let mut aligns = Vec::with_capacity(cells.len());

    for cell in &cells {
        let left = cell.starts_with(':');
        let right = cell.len() > 1 && cell.ends_with(':');
        let body = cell.trim_matches(':');
        if body.is_empty() || !body.chars().all(|ch| ch == '-') {
            return None;
        }
        aligns.push(match (left, right) {
            (true, true) => Alignment::Center,
            (false, true) => Alignment::Right,
            _ => Alignment::Left,
        });
    }

    Some(aligns)
}

fn parse_table(lines: &[&str], start: usize) -> Option<Table> {
    let header_line = strip_cr(lines.get(start)?);
    let delimiter_line = strip_cr(lines.get(start + 1)?);

    let indent = leading_indent(header_line)?.to_string();
    let header = split_row(header_line)?;
    let aligns = parse_alignments(delimiter_line)?;
    if aligns.len() != header.len() {
        return None;
    }

    let mut rows = Vec::new();
    let mut index = start + 2;

    while let Some(raw) = lines.get(index) {
        let line = strip_cr(raw);
        if line.trim().is_empty() || is_fence(line) {
            break;
        }
        match split_row(line) {
            Some(cells) => rows.push(cells),
            None => break,
        }
        index += 1;
    }

    Some(Table {
        indent,
        aligns,
        header,
        rows,
        source: lines[start..index].iter().map(|l| l.to_string()).collect(),
        end: index,
    })
}

/// Content width each column needs before any shrinking.
fn natural_widths(table: &Table) -> Vec<usize> {
    let mut widths: Vec<usize> = table
        .header
        .iter()
        .map(|cell| UnicodeWidthStr::width(cell.as_str()).max(1))
        .collect();

    for row in &table.rows {
        for (index, cell) in row.iter().take(widths.len()).enumerate() {
            widths[index] = widths[index].max(UnicodeWidthStr::width(cell.as_str()));
        }
    }

    widths
}

/// Shrink the widest columns until the table fits into `budget` content columns.
/// Returns `None` when the table cannot fit even at the minimum column width.
fn fit_widths(mut widths: Vec<usize>, budget: usize) -> Option<Vec<usize>> {
    let mut total: usize = widths.iter().sum();

    while total > budget {
        let widest = widths.iter().copied().max().unwrap_or(0);
        if widest <= MIN_COLUMN_WIDTH {
            return None;
        }
        let index = widths
            .iter()
            .position(|width| *width == widest)
            .expect("widest column exists");
        widths[index] -= 1;
        total -= 1;
    }

    Some(widths)
}

fn column_widths(table: &Table, width: usize) -> Option<Vec<usize>> {
    let columns = table.header.len();
    let overhead = 3 * columns + 1 + UnicodeWidthStr::width(table.indent.as_str());
    let budget = width.saturating_sub(overhead);
    let widths = natural_widths(table);

    if widths.iter().sum::<usize>() <= budget {
        return Some(widths);
    }

    fit_widths(widths, budget)
}

fn pad(text: &str, width: usize, align: Alignment) -> String {
    let space = width.saturating_sub(UnicodeWidthStr::width(text));
    match align {
        Alignment::Left => format!("{text}{}", " ".repeat(space)),
        Alignment::Right => format!("{}{text}", " ".repeat(space)),
        Alignment::Center => {
            let left = space / 2;
            format!("{}{text}{}", " ".repeat(left), " ".repeat(space - left))
        }
    }
}

fn border(widths: &[usize], indent: &str) -> String {
    let mut line = String::from(indent);
    line.push('+');
    for width in widths {
        line.push_str(&"-".repeat(width + 2));
        line.push('+');
    }
    line
}

/// Render one logical row, which may span several lines when cells wrap.
fn render_row(cells: &[String], widths: &[usize], aligns: &[Alignment], indent: &str) -> Vec<String> {
    let wrapped: Vec<Vec<String>> = widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let text = cells.get(index).map(String::as_str).unwrap_or("");
            wrap::wrap_text_to_lines(text, *width)
        })
        .collect();

    let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    let mut lines = Vec::with_capacity(height);

    for row in 0..height {
        let mut line = String::from(indent);
        line.push('|');
        for (index, width) in widths.iter().enumerate() {
            let text = wrapped[index].get(row).map(String::as_str).unwrap_or("");
            line.push(' ');
            line.push_str(&pad(text, *width, aligns[index]));
            line.push_str(" |");
        }
        lines.push(line);
    }

    lines
}

/// Lay out a table, or fall back to the original markdown when it cannot be
/// made to fit `width` — a table wider than the wrap width would be mangled by
/// word wrapping anyway.
fn render_table(table: &Table, width: usize) -> Vec<String> {
    let Some(widths) = column_widths(table, width) else {
        return table.source.clone();
    };
    let indent = table.indent.as_str();

    let header = render_row(&table.header, &widths, &table.aligns, indent);
    let rows: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|cells| render_row(cells, &widths, &table.aligns, indent))
        .collect();

    // Separate every row when at least one of them wraps, so multi-line cells
    // stay visually grouped.
    let separate_rows = rows.iter().any(|row| row.len() > 1);
    let rule = border(&widths, indent);

    let mut output = Vec::new();
    output.push(rule.clone());
    output.extend(header);
    output.push(rule.clone());

    for (index, row) in rows.iter().enumerate() {
        if separate_rows && index > 0 {
            output.push(rule.clone());
        }
        output.extend(row.iter().cloned());
    }

    if !rows.is_empty() {
        output.push(rule);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(content: &str) -> String {
        format_tables(content, 80)
    }

    #[test]
    fn detects_markdown_paths() {
        assert!(is_markdown_path(Path::new("README.md")));
        assert!(is_markdown_path(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown_path(Path::new("main.rs")));
        assert!(!is_markdown_path(Path::new("plain")));
    }

    #[test]
    fn formats_a_simple_table() {
        let input = "| Crate | Purpose |\n| --- | --- |\n| clap | CLI parsing |\n";
        assert_eq!(
            format(input),
            "\
+-------+-------------+
| Crate | Purpose     |
+-------+-------------+
| clap  | CLI parsing |
+-------+-------------+
"
        );
    }

    #[test]
    fn honors_column_alignment() {
        let input = "| left | center | right |\n| :--- | :----: | ----: |\n| 1 | 2 | 3 |\n";
        let formatted = format(input);
        assert!(formatted.contains("| 1    |   2    |     3 |"), "{formatted}");
    }

    #[test]
    fn pads_short_rows_and_ignores_extra_cells() {
        let input = "| a | b |\n| --- | --- |\n| only |\n";
        let formatted = format(input);
        assert!(formatted.contains("| only |   |"), "{formatted}");
    }

    #[test]
    fn keeps_surrounding_text_untouched() {
        let input = "# Title\n\n| a |\n| --- |\n| 1 |\n\nafter\n";
        let formatted = format(input);
        assert!(formatted.starts_with("# Title\n\n+"));
        assert!(formatted.ends_with("+\n\nafter\n"));
    }

    #[test]
    fn leaves_tables_inside_code_fences_alone() {
        let input = "```\n| a | b |\n| --- | --- |\n| 1 | 2 |\n```\n";
        assert_eq!(format(input), input);
    }

    #[test]
    fn ignores_tables_without_a_delimiter_row() {
        let input = "| a | b |\n| 1 | 2 |\n";
        assert_eq!(format(input), input);
    }

    #[test]
    fn unescapes_pipes_inside_cells() {
        let input = "| a | b |\n| --- | --- |\n| x \\| y | z |\n";
        let formatted = format(input);
        assert!(formatted.contains("| x | y "), "{formatted}");
    }

    #[test]
    fn wraps_cells_to_fit_the_requested_width() {
        let input = concat!(
            "| Name | Description |\n",
            "| --- | --- |\n",
            "| alpha | one two three four five six seven eight nine ten |\n"
        );
        let formatted = format_tables(input, 40);
        for line in formatted.lines() {
            assert!(
                UnicodeWidthStr::width(line) <= 40,
                "line too wide: {line:?}"
            );
        }
        assert!(formatted.matches("| alpha").count() == 1);
    }

    #[test]
    fn separates_rows_when_cells_wrap() {
        let input = concat!(
            "| a | b |\n",
            "| --- | --- |\n",
            "| one two three four five six | x |\n",
            "| short | y |\n"
        );
        let formatted = format_tables(input, 30);
        // header rule, plus a rule between the two data rows, plus the closing rule
        assert!(formatted.matches("+---").count() >= 4, "{formatted}");
    }

    #[test]
    fn preserves_table_indentation() {
        let input = "  | a | b |\n  | --- | --- |\n  | 1 | 2 |\n";
        let formatted = format(input);
        for line in formatted.lines() {
            assert!(line.starts_with("  "), "lost indent: {line:?}");
        }
    }

    #[test]
    fn skips_indented_code_blocks() {
        let input = "    | a | b |\n    | --- | --- |\n";
        assert_eq!(format(input), input);
    }

    #[test]
    fn preserves_content_without_tables() {
        let input = "# Title\n\nJust a paragraph.\n";
        assert_eq!(format(input), input);
    }

    #[test]
    fn handles_missing_trailing_newline() {
        let input = "| a |\n| --- |\n| 1 |";
        let formatted = format(input);
        assert!(!formatted.ends_with('\n'));
        assert!(formatted.ends_with("+"));
    }

    #[test]
    fn shrinks_columns_down_to_the_minimum() {
        assert_eq!(fit_widths(vec![20, 20, 20], 9), Some(vec![3, 3, 3]));
        assert_eq!(fit_widths(vec![20, 4, 4], 12), Some(vec![4, 4, 4]));
    }

    #[test]
    fn gives_up_when_the_table_cannot_fit() {
        assert_eq!(fit_widths(vec![20, 20, 20], 5), None);
    }

    #[test]
    fn keeps_original_markdown_when_the_table_cannot_fit() {
        let input = "| a | b | c |\n| --- | --- | --- |\n| 1 | 2 | 3 |\n";
        assert_eq!(format_tables(input, 12), input);
    }

    #[test]
    fn renders_a_header_only_table_without_a_doubled_rule() {
        let input = "| Header only |\n| --- |\n";
        assert_eq!(
            format(input),
            "+-------------+\n| Header only |\n+-------------+\n"
        );
    }
}
