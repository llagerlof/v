//! Terminal styling for markdown emphasis.
//!
//! Runs last in the render pipeline, on text that may already carry syntax
//! highlighting escapes: `*text*` is shown italic and `**text**` bold (`***`
//! being both), while the asterisks themselves stay unstyled and faint.

/// Bold on.
const BOLD_ON: &str = "\x1b[1m";
/// Faint (dim) on.
const FAINT_ON: &str = "\x1b[2m";
/// Italic on.
const ITALIC_ON: &str = "\x1b[3m";
/// Normal intensity: clears both bold and faint, leaving colors untouched.
const NORMAL_INTENSITY: &str = "\x1b[22m";
/// Italic off, leaving colors and intensity untouched.
const ITALIC_OFF: &str = "\x1b[23m";
/// Longest run of asterisks that opens a span; also the delimiter source.
const MARKERS: &str = "***";

/// The emphasis a run of asterisks turns on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Emphasis {
    bold: bool,
    italic: bool,
}

impl Emphasis {
    /// Emphasis added by a run of `len` asterisks: one italic, two bold, three both.
    fn from_run(len: usize) -> Self {
        Emphasis {
            bold: len != 1,
            italic: len != 2,
        }
    }

    fn merge(self, other: Self) -> Self {
        Emphasis {
            bold: self.bold || other.bold,
            italic: self.italic || other.italic,
        }
    }
}

/// Render `*italic*`, `**bold**` and `***both***` spans, with faint asterisks.
///
/// Spans may cross wrapped lines but not blank lines; fenced and indented code
/// blocks, and inline code spans, are left as written.
pub fn style_emphasis(content: &str) -> String {
    if !content.contains('*') {
        return content.to_string();
    }

    let mut output = String::with_capacity(content.len() + content.len() / 8);
    let mut chunk = String::new();
    let mut in_fence = false;
    let mut in_indented_code = false;
    let mut prev_space = true;

    for line in content.split_inclusive('\n') {
        let plain = strip_ansi(line);
        let trimmed = plain.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut chunk, &mut output);
            in_fence = !in_fence;
            in_indented_code = false;
            prev_space = false;
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
            prev_space = true;
            continue;
        }

        let indented = plain.starts_with("    ") || plain.starts_with('\t');
        if in_indented_code && !indented {
            in_indented_code = false;
        } else if prev_space && indented {
            in_indented_code = true;
        }
        prev_space = false;

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
        style_chunk(chunk, output, Emphasis::default());
        chunk.clear();
    }
}

/// Style one run of consecutive text lines, nested inside `active` emphasis.
fn style_chunk(chunk: &str, output: &mut String, active: Emphasis) {
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
            b'*' => {
                let run = asterisk_run(bytes, index);
                match span_close(chunk, index, run) {
                    Some(close) => {
                        push_span(&chunk[index + run..close], run, active, output);
                        index = close + run;
                    }
                    None => {
                        output.push_str(&chunk[index..index + run]);
                        index += run;
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

/// Emit one emphasized span, its delimiters faint and unstyled.
fn push_span(content: &str, run: usize, active: Emphasis, output: &mut String) {
    let marker = &MARKERS[..run];
    let inner = active.merge(Emphasis::from_run(run));

    push_marker(marker, active, inner, output);
    style_chunk(content, output, inner);
    push_marker(marker, inner, active, output);
}

/// Emit a delimiter faint and unstyled, dropping the emphasis in effect
/// `before` it and restoring the one that applies `after`.
fn push_marker(marker: &str, before: Emphasis, after: Emphasis, output: &mut String) {
    if before.bold {
        output.push_str(NORMAL_INTENSITY);
    }
    if before.italic {
        output.push_str(ITALIC_OFF);
    }

    output.push_str(FAINT_ON);
    output.push_str(marker);
    // Also clears the faint above, so the delimiter alone is dimmed.
    output.push_str(NORMAL_INTENSITY);

    if after.bold {
        output.push_str(BOLD_ON);
    }
    if after.italic {
        output.push_str(ITALIC_ON);
    }
}

/// Length of the run of asterisks starting at `index`.
fn asterisk_run(bytes: &[u8], index: usize) -> usize {
    let mut end = index;
    while bytes.get(end) == Some(&b'*') {
        end += 1;
    }
    end - index
}

/// Byte index of the delimiter closing the `run` asterisks at `open`, or `None`
/// when they do not open a span.
fn span_close(chunk: &str, open: usize, run: usize) -> Option<usize> {
    if run > MARKERS.len() {
        return None;
    }

    let bytes = chunk.as_bytes();
    let start = open + run;

    // An opener must be followed by content, not by whitespace.
    let after_open = skip_ansi(bytes, start);
    let next = chunk[after_open..].chars().next()?;
    if next.is_whitespace() {
        return None;
    }

    let mut index = start;
    let mut prev_space = true;

    while index < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, index) {
            index = end;
            continue;
        }

        if bytes[index] == b'\\' && index + 1 < bytes.len() {
            index += 1 + char_len(chunk, index + 1);
            prev_space = false;
            continue;
        }

        if bytes[index] == b'`' {
            index = code_span_end(chunk, index);
            prev_space = false;
            continue;
        }

        if bytes[index] == b'*' {
            let closing = asterisk_run(bytes, index);
            // A closer must match the opener and follow content, not whitespace.
            if closing == run && index > start && !prev_space {
                return Some(index);
            }
            index += closing;
            prev_space = false;
            continue;
        }

        let ch = chunk[index..].chars().next()?;
        prev_space = ch.is_whitespace();
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
    const ITALIC: &str = "\x1b[3m";
    const OFF: &str = "\x1b[22m";
    const ITALIC_END: &str = "\x1b[23m";

    #[test]
    fn bolds_content_and_fades_the_asterisks() {
        assert_eq!(
            style_emphasis("a **bold** b"),
            format!("a {FAINT}**{OFF}{BOLD}bold{OFF}{FAINT}**{OFF} b")
        );
    }

    #[test]
    fn italicizes_content_and_fades_the_asterisks() {
        assert_eq!(
            style_emphasis("a *slanted* b"),
            format!("a {FAINT}*{OFF}{ITALIC}slanted{ITALIC_END}{FAINT}*{OFF} b")
        );
    }

    #[test]
    fn treats_three_asterisks_as_bold_italic() {
        assert_eq!(
            style_emphasis("***both***"),
            format!("{FAINT}***{OFF}{BOLD}{ITALIC}both{OFF}{ITALIC_END}{FAINT}***{OFF}")
        );
    }

    #[test]
    fn styles_emphasis_nested_inside_bold() {
        let styled = style_emphasis("**bold *and italic* again**");
        // The inner delimiters drop both attributes, then restore the bold.
        assert!(styled.contains(&format!("{OFF}{FAINT}*{OFF}{BOLD}{ITALIC}and")), "{styled:?}");
        assert!(styled.contains(&format!("italic{OFF}{ITALIC_END}{FAINT}*{OFF}{BOLD} again")), "{styled:?}");
        assert_eq!(strip_ansi(&styled), "**bold *and italic* again**");
    }

    #[test]
    fn leaves_text_without_markers_untouched() {
        let input = "# Title\n\nplain text\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn leaves_lone_asterisks_alone() {
        let input = "* a list item\n* 2 * 3 = 6\n* globs *.rs and *.md\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn styles_several_spans_on_one_line() {
        let styled = style_emphasis("**one** and **two**");
        assert_eq!(styled.matches(BOLD).count(), 2);
        assert_eq!(styled.matches("**").count(), 4);
    }

    #[test]
    fn spans_may_cross_a_wrapped_line() {
        let styled = style_emphasis("**bold text\nkeeps going** after\n");
        assert_eq!(
            styled,
            format!("{FAINT}**{OFF}{BOLD}bold text\nkeeps going{OFF}{FAINT}**{OFF} after\n")
        );
    }

    #[test]
    fn spans_do_not_cross_a_blank_line() {
        let input = "**open\n\nclose** text\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn ignores_unmatched_and_whitespace_padded_markers() {
        assert_eq!(style_emphasis("2 ** 3 and **unclosed"), "2 ** 3 and **unclosed");
    }

    #[test]
    fn requires_the_closing_run_to_match_the_opening_one() {
        let input = "**bold* leftovers\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn ignores_markers_inside_fenced_code() {
        let input = "```python\ndef f(**kwargs):\n    pass\n```\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn ignores_markers_inside_indented_code() {
        let input = "text\n\n    def f(**kwargs):\n        return **kwargs\n\nmore\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn ignores_markers_inside_inline_code() {
        let input = "use `**kwargs` and `a * b` here\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn ignores_escaped_markers() {
        let input = "a \\*\\*not bold\\*\\* b, \\*plain\\* c\n";
        assert_eq!(style_emphasis(input), input);
    }

    #[test]
    fn styles_markers_split_by_highlighting_escapes() {
        let styled = style_emphasis("\x1b[38;2;1;2;3m**\x1b[38;2;4;5;6mbold\x1b[38;2;1;2;3m**");
        assert!(styled.contains(&format!("{FAINT}**{OFF}")), "{styled:?}");
        assert!(styled.contains(&format!("{BOLD}\x1b[38;2;4;5;6mbold")), "{styled:?}");
    }

    #[test]
    fn keeps_table_cell_padding_intact() {
        let styled = style_emphasis("| **a** | b |\n");
        assert_eq!(strip_ansi(&styled), "| **a** | b |\n");
    }

    #[test]
    fn strips_escape_sequences_for_line_checks() {
        assert_eq!(strip_ansi("\x1b[38;2;1;2;3m```rust\x1b[0m"), "```rust");
    }
}
