//! Terminal styling for markdown emphasis.
//!
//! Runs last in the render pipeline, on text that may already carry syntax
//! highlighting escapes: `*text*` is shown italic and `**text**` bold (`***`
//! being both), while the asterisks themselves stay unstyled and dimmed.

/// Bold on.
const BOLD_ON: &str = "\x1b[1m";
/// Italic on.
const ITALIC_ON: &str = "\x1b[3m";
/// Normal intensity: clears bold, leaving colors untouched.
const NORMAL_INTENSITY: &str = "\x1b[22m";
/// Italic off, leaving colors and intensity untouched.
const ITALIC_OFF: &str = "\x1b[23m";
/// Dim gray for the delimiters: a fixed color rather than the syntax color of
/// the moment, so every delimiter fades by the same amount, and rather than
/// faint (`\x1b[2m`), which terminals dim by wildly different amounts.
const MARKER_COLOR: &str = "\x1b[38;2;96;96;96m";
/// Default foreground, restored after a delimiter when no color was active.
const DEFAULT_COLOR: &str = "\x1b[39m";
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

/// Render `*italic*`, `**bold**` and `***both***` spans, with dimmed asterisks.
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
        style_chunk(chunk, output, Emphasis::default(), "");
        chunk.clear();
    }
}

/// Style one run of consecutive text lines, nested inside `active` emphasis and
/// `color` (the foreground already in effect). Returns the foreground left in
/// effect at the end, so delimiters can restore it.
fn style_chunk<'a>(
    chunk: &'a str,
    output: &mut String,
    active: Emphasis,
    color: &'a str,
) -> &'a str {
    let bytes = chunk.as_bytes();
    let mut color = color;
    let mut index = 0;

    while index < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, index) {
            let sequence = &chunk[index..end];
            output.push_str(sequence);
            color = foreground_after(sequence, color);
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
                let span = &chunk[index..end];
                output.push_str(span);
                color = trailing_foreground(span, color);
                index = end;
            }
            b'*' => {
                let run = asterisk_run(bytes, index);
                match span_close(chunk, index, run) {
                    Some(close) => {
                        color = push_span(&chunk[index + run..close], run, active, color, output);
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

    color
}

/// Emit one emphasized span, its delimiters dimmed and unstyled, and return the
/// foreground left in effect.
fn push_span<'a>(
    content: &'a str,
    run: usize,
    active: Emphasis,
    color: &'a str,
    output: &mut String,
) -> &'a str {
    let marker = &MARKERS[..run];
    let inner = active.merge(Emphasis::from_run(run));

    push_marker(marker, active, inner, color, output);
    let trailing = style_chunk(content, output, inner, color);
    push_marker(marker, inner, active, trailing, output);

    trailing
}

/// Emit a delimiter dimmed and unstyled: drop the emphasis in effect `before`
/// it, then restore `color` and the emphasis that applies `after`.
fn push_marker(marker: &str, before: Emphasis, after: Emphasis, color: &str, output: &mut String) {
    if before.bold {
        output.push_str(NORMAL_INTENSITY);
    }
    if before.italic {
        output.push_str(ITALIC_OFF);
    }

    output.push_str(MARKER_COLOR);
    output.push_str(marker);
    output.push_str(if color.is_empty() { DEFAULT_COLOR } else { color });

    if after.bold {
        output.push_str(BOLD_ON);
    }
    if after.italic {
        output.push_str(ITALIC_ON);
    }
}

/// The foreground left in effect by an escape sequence: the sequence itself
/// when it sets a color, an empty string when it resets to the default, and the
/// unchanged `color` otherwise.
fn foreground_after<'a>(sequence: &'a str, color: &'a str) -> &'a str {
    let Some(params) = sequence
        .strip_prefix("\x1b[")
        .and_then(|rest| rest.strip_suffix('m'))
    else {
        return color;
    };

    if params.starts_with("38;") {
        return sequence;
    }

    let mut result = color;
    for param in params.split(';') {
        match param.parse::<u16>() {
            Ok(0) | Ok(39) => result = "",
            Ok(30..=37) | Ok(90..=97) => result = sequence,
            Err(_) if param.is_empty() => result = "",
            _ => {}
        }
    }

    result
}

/// The foreground left in effect by every escape sequence in `text`.
fn trailing_foreground<'a>(text: &'a str, color: &'a str) -> &'a str {
    let bytes = text.as_bytes();
    let mut color = color;
    let mut index = 0;

    while index < bytes.len() {
        match ansi_escape_end(bytes, index) {
            Some(end) => {
                color = foreground_after(&text[index..end], color);
                index = end;
            }
            None => index += char_len(text, index),
        }
    }

    color
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
    const ITALIC: &str = "\x1b[3m";
    const OFF: &str = "\x1b[22m";
    const ITALIC_END: &str = "\x1b[23m";
    /// Dim gray a delimiter is painted in.
    const DIM: &str = "\x1b[38;2;96;96;96m";
    /// Foreground restored after a delimiter, when no color was in effect.
    const UNDIM: &str = "\x1b[39m";

    #[test]
    fn bolds_content_and_fades_the_asterisks() {
        assert_eq!(
            style_emphasis("a **bold** b"),
            format!("a {DIM}**{UNDIM}{BOLD}bold{OFF}{DIM}**{UNDIM} b")
        );
    }

    #[test]
    fn italicizes_content_and_fades_the_asterisks() {
        assert_eq!(
            style_emphasis("a *slanted* b"),
            format!("a {DIM}*{UNDIM}{ITALIC}slanted{ITALIC_END}{DIM}*{UNDIM} b")
        );
    }

    #[test]
    fn treats_three_asterisks_as_bold_italic() {
        assert_eq!(
            style_emphasis("***both***"),
            format!("{DIM}***{UNDIM}{BOLD}{ITALIC}both{OFF}{ITALIC_END}{DIM}***{UNDIM}")
        );
    }

    #[test]
    fn styles_emphasis_nested_inside_bold() {
        let styled = style_emphasis("**bold *and italic* again**");
        // The inner delimiters drop both attributes, then restore the bold.
        assert!(
            styled.contains(&format!("{OFF}{DIM}*{UNDIM}{BOLD}{ITALIC}and")),
            "{styled:?}"
        );
        assert!(
            styled.contains(&format!("italic{OFF}{ITALIC_END}{DIM}*{UNDIM}{BOLD} again")),
            "{styled:?}"
        );
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
        assert_eq!(styled.matches(DIM).count(), 4);
        assert_eq!(styled.matches("**").count(), 4);
    }

    #[test]
    fn spans_may_cross_a_wrapped_line() {
        let styled = style_emphasis("**bold text\nkeeps going** after\n");
        assert_eq!(
            styled,
            format!("{DIM}**{UNDIM}{BOLD}bold text\nkeeps going{OFF}{DIM}**{UNDIM} after\n")
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
        assert!(styled.contains(&format!("{DIM}**")), "{styled:?}");
        assert!(styled.contains(&format!("{BOLD}\x1b[38;2;4;5;6mbold")), "{styled:?}");
    }

    #[test]
    fn dims_every_delimiter_by_the_same_amount() {
        // syntect colors `*` and `**` differently; the delimiters must not.
        let bold = style_emphasis("\x1b[38;2;255;255;208m**bold**");
        let italic = style_emphasis("\x1b[38;2;255;213;255m*italic*");
        assert!(bold.contains(DIM), "{bold:?}");
        assert!(italic.contains(DIM), "{italic:?}");
    }

    #[test]
    fn restores_the_active_color_after_a_delimiter() {
        let color = "\x1b[38;2;1;2;3m";
        let styled = style_emphasis(&format!("{color}**bold** tail"));
        assert_eq!(
            styled,
            format!("{color}{DIM}**{color}{BOLD}bold{OFF}{DIM}**{color} tail")
        );
    }

    #[test]
    fn tracks_the_foreground_across_escape_sequences() {
        let color = "\x1b[38;2;1;2;3m";
        assert_eq!(foreground_after(color, ""), color);
        assert_eq!(foreground_after("\x1b[31m", ""), "\x1b[31m");
        assert_eq!(foreground_after("\x1b[0m", color), "");
        assert_eq!(foreground_after("\x1b[39m", color), "");
        // Not a foreground change: keep what was already in effect.
        assert_eq!(foreground_after("\x1b[1m", color), color);
        assert_eq!(foreground_after("\x1b[48;2;1;2;3m", color), color);
        assert_eq!(trailing_foreground(&format!("a{color}b\x1b[0mc"), "x"), "");
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
