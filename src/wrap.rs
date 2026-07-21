use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Compute the effective wrap width from a requested column count and terminal width.
pub fn effective_wrap_width(requested: usize, terminal_cols: usize) -> usize {
    let terminal_cols = terminal_cols.max(1);
    if requested == 0 {
        return terminal_cols;
    }
    requested
}

/// Return the terminal column count, falling back to 80 when unavailable.
pub fn terminal_columns() -> usize {
    terminal_size::terminal_size()
        .map(|(width, _)| width.0 as usize)
        .unwrap_or(80)
        .max(1)
}

/// Wrap plain text by word at the given column width.
pub fn wrap_plain_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }

    let mut output = String::with_capacity(text.len() + text.len() / 8);

    for line in split_lines_preserving_endings(text) {
        let has_newline = line.ends_with('\n');
        let content = line.trim_end_matches('\n');
        let wrapped = wrap_plain_line(content, width);

        for (index, part) in wrapped.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            output.push_str(part);
        }

        if has_newline {
            output.push('\n');
        }
    }

    output
}

fn split_lines_preserving_endings(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;

    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            let end = index + 1;
            lines.push(&text[start..end]);
            start = end;
        }
    }

    if start < text.len() {
        lines.push(&text[start..]);
    }

    if lines.is_empty() {
        lines.push("");
    }

    lines
}

fn split_leading_whitespace(line: &str) -> (&str, &str) {
    let content_start = line
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(line.len());
    line.split_at(content_start)
}

fn line_width(text: &str) -> usize {
    text.width()
}

fn wrap_plain_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }

    if line_width(line) <= width {
        return vec![line.to_string()];
    }

    let (prefix, content) = split_leading_whitespace(line);
    if content.is_empty() {
        return vec![line.to_string()];
    }

    let prefix_width = line_width(prefix);
    let content_width = width.saturating_sub(prefix_width).max(1);
    let wrapped_content = wrap_line_content(content, content_width);

    wrapped_content
        .into_iter()
        .map(|part| format!("{prefix}{part}"))
        .collect()
}

fn wrap_line_content(content: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;

    for word in split_words(content) {
        let word_len = line_width(word);

        if word_len > width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
                current_len = 0;
            }
            for chunk in break_long_word(word, width) {
                lines.push(chunk);
            }
            continue;
        }

        let needed = if current.is_empty() {
            word_len
        } else {
            word_len + 1
        };

        if current_len + needed > width {
            lines.push(current);
            current = word.to_string();
            current_len = word_len;
        } else if current.is_empty() {
            current = word.to_string();
            current_len = word_len;
        } else {
            current.push(' ');
            current.push_str(word);
            current_len += needed;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn split_words(content: &str) -> impl Iterator<Item = &str> {
    content.split_whitespace()
}

fn break_long_word(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chunk = String::new();
    let mut chunk_width = 0;

    for ch in word.chars() {
        let ch_width = char_display_width(ch);
        if chunk_width + ch_width > width && !chunk.is_empty() {
            chunks.push(chunk);
            chunk = String::new();
            chunk_width = 0;
        }
        chunk.push(ch);
        chunk_width += ch_width;
    }

    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    chunks
}

fn char_display_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_width_uses_requested_column() {
        assert_eq!(effective_wrap_width(120, 200), 120);
    }

    #[test]
    fn effective_width_honors_requested_value_on_narrow_terminal() {
        assert_eq!(effective_wrap_width(120, 80), 120);
    }

    #[test]
    fn effective_width_zero_means_terminal() {
        assert_eq!(effective_wrap_width(0, 80), 80);
    }

    #[test]
    fn wraps_plain_text_by_words() {
        let wrapped = wrap_plain_text("one two three four", 8);
        assert_eq!(wrapped, "one two\nthree\nfour");
    }

    #[test]
    fn wraps_at_requested_width() {
        let line = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima";
        let wrapped = wrap_plain_text(line, 30);
        for part in wrapped.split('\n') {
            assert!(
                line_width(part) <= 30,
                "line exceeded width: {part:?} ({})",
                line_width(part)
            );
        }
        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn breaks_overlong_words() {
        let wrapped = wrap_plain_text("abcdefghij", 4);
        assert_eq!(wrapped, "abcd\nefgh\nij");
    }

    #[test]
    fn preserves_existing_line_breaks() {
        let wrapped = wrap_plain_text("short\nanother line here", 80);
        assert_eq!(wrapped, "short\nanother line here");
    }

    #[test]
    fn preserves_leading_indent_on_wrapped_lines() {
        let line = "    fn process_items(items: &[Item]) -> Result<Vec<ProcessedItem>, Error> {";
        let wrapped = wrap_plain_text(line, 40);
        assert_eq!(
            wrapped,
            "    fn process_items(items: &[Item]) ->\n    Result<Vec<ProcessedItem>, Error> {"
        );
    }

    #[test]
    fn wraps_by_terminal_display_width() {
        let wrapped = wrap_plain_text("你好世界", 4);
        assert_eq!(wrapped, "你好\n世界");
    }

    #[test]
    fn preserves_indent_on_wrapped_markdown() {
        let line = "    - This is a long bullet point that should stay indented when it wraps";
        let wrapped = wrap_plain_text(line, 50);
        for part in wrapped.split('\n') {
            assert!(
                part.starts_with("    "),
                "continuation line lost indent: {part:?}"
            );
        }
    }
}
