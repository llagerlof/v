//! Terminal styling for markdown strong emphasis.
//!
//! Runs last in the render pipeline, on text that may already carry syntax
//! highlighting escapes: `**text**` is shown bold, while the asterisks
//! themselves stay unbold and faint.

/// Bold on.
const BOLD_ON: &str = "\x1b[1m";
/// Faint (dim) on.
const FAINT_ON: &str = "\x1b[2m";
/// Normal intensity: clears both bold and faint, leaving colors untouched.
const NORMAL_INTENSITY: &str = "\x1b[22m";
/// Strong emphasis delimiter.
const MARKER: &str = "**";

/// Render `**text**` spans bold, with faint asterisks.
///
/// Spans may cross wrapped lines but not blank lines; fenced and indented code
/// blocks, and inline code spans, are left as written.
pub fn style_strong(content: &str) -> String {
    if !content.contains(MARKER) {
        return content.to_string();
    }

    let mut output = String::with_capacity(content.len() + content.len() / 8);
    let mut chunk = String::new();
    let mut in_fence = false;
    let mut in_indented_code = false;
    let mut prev_blank = true;

    for line in content.split_inclusive('\n') {
        let plain = strip_ansi(line);
        let trimmed = plain.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut chunk, &mut output);
            in_fence = !in_fence;
            in_indented_code = false;
            prev_blank = false;
            output.push_str(line);
            continue;
        }

        if in_fence {
            flush(&mut chunk, &mut output);
            output.push_str(line);
            continue;
        }

        if trimmed.is_empty() {
            flush(&mut chunk, &mut output);
            output.push_str(line);
            prev_blank = true;
            continue;
        }

        let indented = plain.starts_with("    ") || plain.starts_with('\t');
        if in_indented_code && !indented {
            in_indented_code = false;
        } else if prev_blank && indented {
            in_indented_code = true;
        }
        prev_blank = false;

        if in_indented_code {
            flush(&mut chunk, &mut output);
            output.push_str(line);
            continue;
        }

        chunk.push_str(line);
    }

    flush(&mut chunk, &mut output);
    output
}

fn flush(chunk: &mut String, output: &mut String) {
    if !chunk.is_empty() {
        style_chunk(chunk, output);
        chunk.clear();
    }
}

/// Style one run of consecutive text lines.
fn style_chunk(chunk: &str, output: &mut String) {
    let bytes = chunk.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, index) {
            output.push_str(&chunk[index..end]);
            index = end;
            continue;
        }

        match bytes[index] {
            b'\\' if index + 1 < bytes.len() => {
                let end = index + 1 + char_len(chunk, index + 1);
                output.push_str(&chunk[index..end]);
                index = end;
            }
            b'`' => {
                let end = code_span_end(chunk, index);
                output.push_str(&chunk[index..end]);
                index = end;
            }
            b'*' if bytes.get(index + 1) == Some(&b'*') => {
                match span_content_end(chunk, index) {
                    Some(close) => {
                        push_strong(&chunk[index + MARKER.len()..close], output);
                        index = close + MARKER.len();
                    }
                    None => {
                        output.push_str(MARKER);
                        index += MARKER.len();
                    }
                }
            }
            _ => {
                let end = index + char_len(chunk, index);
                output.push_str(&chunk[index..end]);
                index = end;
            }
        }
    }
}

fn push_strong(content: &str, output: &mut String) {
    output.push_str(FAINT_ON);
    output.push_str(MARKER);
    output.push_str(NORMAL_INTENSITY);

    output.push_str(BOLD_ON);
    output.push_str(content);
    output.push_str(NORMAL_INTENSITY);

    output.push_str(FAINT_ON);
    output.push_str(MARKER);
    output.push_str(NORMAL_INTENSITY);
}

/// Byte index of the closing `**` for an opener at `open`, or `None` when the
/// delimiters do not form a span.
fn span_content_end(chunk: &str, open: usize) -> Option<usize> {
    let bytes = chunk.as_bytes();
    let start = open + MARKER.len();

    // An opener must be followed by content, not by whitespace.
    let after_open = skip_ansi(bytes, start);
    let next = chunk[after_open..].chars().next()?;
    if next.is_whitespace() {
        return None;
    }

    let mut index = start;
    let mut prev_blank = true;

    while index < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, index) {
            index = end;
            continue;
        }

        if bytes[index] == b'\\' && index + 1 < bytes.len() {
            index += 1 + char_len(chunk, index + 1);
            prev_blank = false;
            continue;
        }

        if bytes[index] == b'`' {
            index = code_span_end(chunk, index);
            prev_blank = false;
            continue;
        }

        if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'*') {
            // A closer must follow content, not whitespace.
            if index > start && !prev_blank {
                return Some(index);
            }
            index += MARKER.len();
            prev_blank = false;
            continue;
        }

        let ch = chunk[index..].chars().next()?;
        prev_blank = ch.is_whitespace();
        index += ch.len_utf8();
    }

    None
}

/// End of the inline code span starting at `start`, or just past its opening
/// backticks when it is never closed.
fn code_span_end(chunk: &str, start: usize) -> usize {
    let bytes = chunk.as_bytes();
    let mut open = start;
    while bytes.get(open) == Some(&b'`') {
        open += 1;
    }
    let fence = open - start;

    let mut index = open;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let mut close = index;
        while bytes.get(close) == Some(&b'`') {
            close += 1;
        }
        if close - index == fence {
            return close;
        }
        index = close;
    }

    open
}

/// Length of the UTF-8 character at `index`.
fn char_len(text: &str, index: usize) -> usize {
    text[index..].chars().next().map_or(1, char::len_utf8)
}

/// End of the ANSI escape sequence starting at `index`, or `None` when there is
/// no sequence there.
fn ansi_escape_end(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&0x1b) || bytes.get(index + 1) != Some(&b'[') {
        return None;
    }

    let mut end = index + 2;
    while let Some(byte) = bytes.get(end) {
        end += 1;
        if (0x40..=0x7e).contains(byte) {
            return Some(end);
        }
    }

    Some(end)
}

fn skip_ansi(bytes: &[u8], mut index: usize) -> usize {
    while let Some(end) = ansi_escape_end(bytes, index) {
        index = end;
    }
    index
}

/// Drop ANSI escape sequences, so line-level markdown checks see plain text.
fn strip_ansi(line: &str) -> String {
    if !line.contains('\x1b') {
        return line.to_string();
    }

    let bytes = line.as_bytes();
    let mut plain = String::with_capacity(line.len());
    let mut index = 0;

    while index < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, index) {
            index = end;
            continue;
        }
        let end = index + char_len(line, index);
        plain.push_str(&line[index..end]);
        index = end;
    }

    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOLD: &str = "\x1b[1m";
    const FAINT: &str = "\x1b[2m";
    const OFF: &str = "\x1b[22m";

    #[test]
    fn bolds_content_and_fades_the_asterisks() {
        assert_eq!(
            style_strong("a **bold** b"),
            format!("a {FAINT}**{OFF}{BOLD}bold{OFF}{FAINT}**{OFF} b")
        );
    }

    #[test]
    fn leaves_text_without_markers_untouched() {
        let input = "# Title\n\nplain *italic* text\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn styles_several_spans_on_one_line() {
        let styled = style_strong("**one** and **two**");
        assert_eq!(styled.matches(BOLD).count(), 2);
        assert_eq!(styled.matches("**").count(), 4);
    }

    #[test]
    fn spans_may_cross_a_wrapped_line() {
        let styled = style_strong("**bold text\nkeeps going** after\n");
        assert_eq!(
            styled,
            format!("{FAINT}**{OFF}{BOLD}bold text\nkeeps going{OFF}{FAINT}**{OFF} after\n")
        );
    }

    #[test]
    fn spans_do_not_cross_a_blank_line() {
        let input = "**open\n\nclose** text\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn ignores_unmatched_and_whitespace_padded_markers() {
        assert_eq!(style_strong("2 ** 3 and **unclosed"), "2 ** 3 and **unclosed");
    }

    #[test]
    fn ignores_markers_inside_fenced_code() {
        let input = "```python\ndef f(**kwargs):\n    pass\n```\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn ignores_markers_inside_indented_code() {
        let input = "text\n\n    def f(**kwargs):\n        return **kwargs\n\nmore\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn ignores_markers_inside_inline_code() {
        let input = "use `**kwargs` here\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn ignores_escaped_markers() {
        let input = "a \\**not bold\\** b\n";
        assert_eq!(style_strong(input), input);
    }

    #[test]
    fn styles_markers_split_by_highlighting_escapes() {
        let styled = style_strong("\x1b[38;2;1;2;3m**\x1b[38;2;4;5;6mbold\x1b[38;2;1;2;3m**");
        assert!(styled.contains(&format!("{FAINT}**{OFF}")), "{styled:?}");
        assert!(styled.contains(&format!("{BOLD}\x1b[38;2;4;5;6mbold")), "{styled:?}");
    }

    #[test]
    fn keeps_table_cell_padding_intact() {
        let styled = style_strong("| **a** | b |\n");
        assert_eq!(strip_ansi(&styled), "| **a** | b |\n");
    }

    #[test]
    fn strips_escape_sequences_for_line_checks() {
        assert_eq!(strip_ansi("\x1b[38;2;1;2;3m```rust\x1b[0m"), "```rust");
    }
}
