//! Terminal styling for markdown emphasis and headings.
//!
//! Runs last in the render pipeline, on text that may already carry syntax
//! highlighting escapes: `*text*` is shown italic and `**text**` bold (`***`
//! being both), and `# Heading` text bold, while the asterisks and hashes
//! themselves stay unstyled and dimmed. List item markers are bold, so bullets
//! stand out from the text they introduce.

use unicode_width::UnicodeWidthStr;

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
/// Deepest ATX heading level.
const MAX_HEADING_LEVEL: usize = 6;
/// Deepest indent an ATX heading may carry.
const MAX_HEADING_INDENT: usize = 3;
/// Markers a thematic break (`---`, `* * *`) needs at least.
const MIN_THEMATIC_BREAK: usize = 3;

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

/// Render `*italic*`, `**bold**` and `***both***` spans and `# Heading` text,
/// with dimmed asterisks and hashes, and bold `-` and `*` list item markers.
///
/// Spans may cross wrapped lines but not blank lines; fenced and indented code
/// blocks, and inline code spans, are left as written. `width` is the wrap
/// width the text was laid out at, used to follow a heading that wrapped onto
/// the next line.
pub fn style_emphasis(content: &str, width: usize) -> String {
    if !content.contains('*') && !content.contains('#') && !content.contains('-') {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let mut output = String::with_capacity(content.len() + content.len() / 8);
    let mut chunk = String::new();
    let mut in_fence = false;
    let mut in_indented_code = false;
    let mut in_heading = false;
    let mut prev_space = true;

    for (index, line) in lines.iter().enumerate() {
        let plain = strip_ansi(line);
        let trimmed = plain.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut chunk, &mut output);
            in_fence = !in_fence;
            in_indented_code = false;
            in_heading = false;
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
            in_heading = false;
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
            in_heading = false;
            output.push_str(line);
            continue;
        }

        // The rest of a heading that word wrapping pushed onto its own line.
        if in_heading {
            push_heading_text(line, "", &mut output);
            in_heading = wraps_onto_next(&plain, lines.get(index + 1), width);
            continue;
        }

        if let Some(marks) = heading_marks(line) {
            flush(&mut chunk, &mut output);
            push_heading(line, marks, &mut output);
            in_heading = wraps_onto_next(&plain, lines.get(index + 1), width);
            continue;
        }

        chunk.push_str(line);
    }

    flush(&mut chunk, &mut output);
    output
}

/// Byte range of the leading `#` run of an ATX heading, or `None` when the line
/// is not one.
fn heading_marks(line: &str) -> Option<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut index = skip_ansi(bytes, 0);

    for _ in 0..=MAX_HEADING_INDENT {
        if bytes.get(index) != Some(&b' ') {
            break;
        }
        index = skip_ansi(bytes, index + 1);
    }

    let start = index;
    while bytes.get(index) == Some(&b'#') {
        index += 1;
    }
    let level = index - start;
    if level == 0 || level > MAX_HEADING_LEVEL {
        return None;
    }

    // `#hashtag` is not a heading; `#` alone on its line is.
    match bytes.get(skip_ansi(bytes, index)) {
        None | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => Some((start, index)),
        _ => None,
    }
}

/// Emit a heading line: its `#` run dimmed, its text bold.
fn push_heading(line: &str, marks: (usize, usize), output: &mut String) {
    let (start, end) = marks;

    output.push_str(&line[..start]);
    let color = trailing_foreground(&line[..start], "");

    output.push_str(MARKER_COLOR);
    output.push_str(&line[start..end]);
    output.push_str(if color.is_empty() {
        DEFAULT_COLOR
    } else {
        color
    });

    push_heading_text(&line[end..], color, output);
}

/// Emit heading text in bold, keeping any emphasis inside it.
fn push_heading_text<'a>(text: &'a str, color: &'a str, output: &mut String) {
    let (text, ending) = split_line_ending(text);

    output.push_str(BOLD_ON);
    style_chunk(
        text,
        output,
        Emphasis {
            bold: true,
            italic: false,
        },
        color,
    );
    output.push_str(NORMAL_INTENSITY);
    output.push_str(ending);
}

fn split_line_ending(line: &str) -> (&str, &str) {
    match line.strip_suffix('\n') {
        Some(rest) => line.split_at(rest.strip_suffix('\r').unwrap_or(rest).len()),
        None => (line, ""),
    }
}

/// Whether word wrapping at `width` would have pushed the start of `next` onto
/// its own line, meaning it continues the current one.
fn wraps_onto_next(plain: &str, next: Option<&&str>, width: usize) -> bool {
    if width == 0 {
        return false;
    }

    let Some(next) = next else {
        return false;
    };
    let next = strip_ansi(next);
    let Some(word) = next.split_whitespace().next() else {
        return false;
    };

    let used = UnicodeWidthStr::width(plain.trim_end());
    used + 1 + UnicodeWidthStr::width(word) > width
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
        // A list item marker, when this is the start of a line and the text is
        // not bold already.
        if !active.bold
            && (index == 0 || bytes[index - 1] == b'\n')
            && let Some((start, end)) = bullet_marker(chunk, index)
        {
            let indent = &chunk[index..start];
            output.push_str(indent);
            color = trailing_foreground(indent, color);
            output.push_str(BOLD_ON);
            output.push_str(&chunk[start..end]);
            output.push_str(NORMAL_INTENSITY);
            index = end;
            continue;
        }

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
    output.push_str(if color.is_empty() {
        DEFAULT_COLOR
    } else {
        color
    });

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

/// Byte range of the `-` or `*` that marks a list item on the line starting at
/// `index`, or `None` when the line does not start one.
fn bullet_marker(chunk: &str, index: usize) -> Option<(usize, usize)> {
    let bytes = chunk.as_bytes();
    let mut start = skip_ansi(bytes, index);
    while matches!(bytes.get(start), Some(b' ' | b'\t')) {
        start = skip_ansi(bytes, start + 1);
    }

    let end = match bytes.get(start) {
        Some(b'-') => start + 1,
        // One asterisk only: `**bold**` opening a line is not a bullet.
        Some(b'*') if asterisk_run(bytes, start) == 1 => start + 1,
        _ => return None,
    };

    // The marker must be followed by the item's text, or end the line, and a
    // run of markers on a line of its own is a thematic break, not a bullet.
    match bytes.get(skip_ansi(bytes, end)) {
        None | Some(b' ' | b'\t' | b'\n' | b'\r') if !is_thematic_break(chunk, start) => {
            Some((start, end))
        }
        _ => None,
    }
}

/// Whether the line starting at `index` holds nothing but three or more of the
/// same marker and spaces, as `---` and `* * *` do.
fn is_thematic_break(chunk: &str, index: usize) -> bool {
    let bytes = chunk.as_bytes();
    let marker = bytes[index];
    let mut count = 0;
    let mut cursor = index;

    while cursor < bytes.len() {
        if let Some(end) = ansi_escape_end(bytes, cursor) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            byte if byte == marker => count += 1,
            b' ' | b'\t' => {}
            b'\n' | b'\r' => break,
            _ => return false,
        }
        cursor += 1;
    }

    count >= MIN_THEMATIC_BREAK
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

    /// Style at the default wrap width.
    fn style(content: &str) -> String {
        style_emphasis(content, 80)
    }

    #[test]
    fn bolds_content_and_fades_the_asterisks() {
        assert_eq!(
            style("a **bold** b"),
            format!("a {DIM}**{UNDIM}{BOLD}bold{OFF}{DIM}**{UNDIM} b")
        );
    }

    #[test]
    fn italicizes_content_and_fades_the_asterisks() {
        assert_eq!(
            style("a *slanted* b"),
            format!("a {DIM}*{UNDIM}{ITALIC}slanted{ITALIC_END}{DIM}*{UNDIM} b")
        );
    }

    #[test]
    fn treats_three_asterisks_as_bold_italic() {
        assert_eq!(
            style("***both***"),
            format!("{DIM}***{UNDIM}{BOLD}{ITALIC}both{OFF}{ITALIC_END}{DIM}***{UNDIM}")
        );
    }

    #[test]
    fn styles_emphasis_nested_inside_bold() {
        let styled = style("**bold *and italic* again**");
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
        let input = "Title\n\nplain text\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn leaves_lone_asterisks_alone() {
        // The bullets turn bold, but none of them opens an emphasis span.
        let input = "* a list item\n* 2 * 3 = 6\n* globs *.rs and *.md\n";
        let styled = style(input);
        assert_eq!(strip_ansi(&styled), input);
        assert!(!styled.contains(ITALIC), "{styled:?}");
        assert_eq!(styled.matches(BOLD).count(), 3);
    }

    #[test]
    fn bolds_list_item_markers() {
        assert_eq!(
            style("- first\n* second\n"),
            format!("{BOLD}-{OFF} first\n{BOLD}*{OFF} second\n")
        );
    }

    #[test]
    fn bolds_nested_list_markers_and_keeps_the_indent() {
        assert_eq!(
            style("- top\n  - nested\n\t- tabbed\n"),
            format!("{BOLD}-{OFF} top\n  {BOLD}-{OFF} nested\n\t{BOLD}-{OFF} tabbed\n")
        );
    }

    #[test]
    fn bolds_an_empty_list_item_marker() {
        assert_eq!(style("- \n-\n"), format!("{BOLD}-{OFF} \n{BOLD}-{OFF}\n"));
    }

    #[test]
    fn keeps_emphasis_inside_a_list_item() {
        assert_eq!(
            style("- a **bold** item\n"),
            format!("{BOLD}-{OFF} a {DIM}**{UNDIM}{BOLD}bold{OFF}{DIM}**{UNDIM} item\n")
        );
    }

    #[test]
    fn restores_the_active_color_after_a_list_marker() {
        let color = "\x1b[38;2;1;2;3m";
        assert_eq!(
            style(&format!("{color}- item **bold**\n")),
            format!("{color}{BOLD}-{OFF} item {DIM}**{color}{BOLD}bold{OFF}{DIM}**{color}\n")
        );
    }

    #[test]
    fn leaves_hyphens_that_do_not_start_a_list_alone() {
        let input = "well-known text\n- - -\n---\n***\nem -- dash\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn leaves_list_markers_inside_code_blocks_alone() {
        let fenced = "```sh\n- not a list\n```\n";
        assert_eq!(style(fenced), fenced);
        let indented = "text\n\n    - not a list\n\nmore\n";
        assert_eq!(style(indented), indented);
    }

    #[test]
    fn leaves_table_rows_alone() {
        let input = "| - | b |\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn styles_several_spans_on_one_line() {
        let styled = style("**one** and **two**");
        assert_eq!(styled.matches(BOLD).count(), 2);
        assert_eq!(styled.matches(DIM).count(), 4);
        assert_eq!(styled.matches("**").count(), 4);
    }

    #[test]
    fn spans_may_cross_a_wrapped_line() {
        let styled = style("**bold text\nkeeps going** after\n");
        assert_eq!(
            styled,
            format!("{DIM}**{UNDIM}{BOLD}bold text\nkeeps going{OFF}{DIM}**{UNDIM} after\n")
        );
    }

    #[test]
    fn spans_do_not_cross_a_blank_line() {
        let input = "**open\n\nclose** text\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_unmatched_and_whitespace_padded_markers() {
        assert_eq!(style("2 ** 3 and **unclosed"), "2 ** 3 and **unclosed");
    }

    #[test]
    fn requires_the_closing_run_to_match_the_opening_one() {
        let input = "**bold* leftovers\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_markers_inside_fenced_code() {
        let input = "```python\ndef f(**kwargs):\n    pass\n```\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_markers_inside_indented_code() {
        let input = "text\n\n    def f(**kwargs):\n        return **kwargs\n\nmore\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_markers_inside_inline_code() {
        let input = "use `**kwargs` and `a * b` here\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_escaped_markers() {
        let input = "a \\*\\*not bold\\*\\* b, \\*plain\\* c\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn styles_markers_split_by_highlighting_escapes() {
        let styled = style("\x1b[38;2;1;2;3m**\x1b[38;2;4;5;6mbold\x1b[38;2;1;2;3m**");
        assert!(styled.contains(&format!("{DIM}**")), "{styled:?}");
        assert!(
            styled.contains(&format!("{BOLD}\x1b[38;2;4;5;6mbold")),
            "{styled:?}"
        );
    }

    #[test]
    fn dims_every_delimiter_by_the_same_amount() {
        // syntect colors `*` and `**` differently; the delimiters must not.
        let bold = style("\x1b[38;2;255;255;208m**bold**");
        let italic = style("\x1b[38;2;255;213;255m*italic*");
        assert!(bold.contains(DIM), "{bold:?}");
        assert!(italic.contains(DIM), "{italic:?}");
    }

    #[test]
    fn restores_the_active_color_after_a_delimiter() {
        let color = "\x1b[38;2;1;2;3m";
        let styled = style(&format!("{color}**bold** tail"));
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
        let styled = style("| **a** | b |\n");
        assert_eq!(strip_ansi(&styled), "| **a** | b |\n");
    }

    #[test]
    fn strips_escape_sequences_for_line_checks() {
        assert_eq!(strip_ansi("\x1b[38;2;1;2;3m```rust\x1b[0m"), "```rust");
    }

    #[test]
    fn bolds_heading_text_and_dims_the_hashes() {
        assert_eq!(
            style("## Section\n"),
            format!("{DIM}##{UNDIM}{BOLD} Section{OFF}\n")
        );
    }

    #[test]
    fn styles_every_heading_level() {
        for level in 1..=MAX_HEADING_LEVEL {
            let hashes = "#".repeat(level);
            let styled = style(&format!("{hashes} Title\n"));
            assert_eq!(
                styled,
                format!("{DIM}{hashes}{UNDIM}{BOLD} Title{OFF}\n"),
                "level {level}"
            );
        }
    }

    #[test]
    fn keeps_emphasis_inside_a_heading() {
        let styled = style("# A *slanted* title\n");
        assert!(
            styled.contains(&format!("{OFF}{DIM}*{UNDIM}{BOLD}{ITALIC}slanted")),
            "{styled:?}"
        );
        assert_eq!(strip_ansi(&styled), "# A *slanted* title\n");
    }

    #[test]
    fn follows_a_heading_that_wrapped_onto_the_next_line() {
        // As wrapped at 20 columns: the second line continues the heading.
        let styled = style_emphasis("# a title that is\nlong\n\nplain\n", 20);
        assert_eq!(
            styled,
            format!("{DIM}#{UNDIM}{BOLD} a title that is{OFF}\n{BOLD}long{OFF}\n\nplain\n")
        );
    }

    #[test]
    fn stops_following_a_heading_that_fits() {
        let styled = style_emphasis("# short\nplain text\n", 20);
        assert_eq!(
            styled,
            format!("{DIM}#{UNDIM}{BOLD} short{OFF}\nplain text\n")
        );
    }

    #[test]
    fn ignores_hashes_that_do_not_start_a_heading() {
        let input = "#hashtag, C# and a # in prose\n     # over-indented\n####### too deep\n";
        assert_eq!(style(input), input);
    }

    #[test]
    fn ignores_hashes_inside_code_blocks() {
        let fenced = "```bash\n# a comment\necho hi\n```\n";
        assert_eq!(style(fenced), fenced);
        let indented = "text\n\n    # a comment\n    echo hi\n\nmore\n";
        assert_eq!(style(indented), indented);
    }

    #[test]
    fn dims_hashes_in_the_syntax_color_of_the_delimiter() {
        let color = "\x1b[38;2;1;2;3m";
        assert_eq!(
            style(&format!("{color}# Title\n")),
            format!("{color}{DIM}#{color}{BOLD} Title{OFF}\n")
        );
    }

    #[test]
    fn keeps_heading_indentation_and_line_endings() {
        assert_eq!(
            style("  # Title\r\n"),
            format!("  {DIM}#{UNDIM}{BOLD} Title{OFF}\r\n")
        );
    }
}
