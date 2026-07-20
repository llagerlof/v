/// Compute the effective wrap width from a requested column count and terminal width.
pub fn effective_wrap_width(requested: usize, terminal_cols: usize) -> usize {
    let terminal_cols = terminal_cols.max(1);
    if requested == 0 {
        return terminal_cols;
    }
    requested.max(terminal_cols)
}

/// Return the terminal column count, falling back to 80 when unavailable.
pub fn terminal_columns() -> usize {
    terminal_size::terminal_size()
        .map(|(width, _)| width.0 as usize)
        .unwrap_or(80)
        .max(1)
}

/// Wrap text by word at the given visible column width, preserving ANSI escape sequences.
pub fn wrap_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }

    let mut output = String::with_capacity(text.len() + text.len() / 8);

    for line in split_lines_preserving_endings(text) {
        let has_newline = line.ends_with('\n');
        let content = line.trim_end_matches('\n');
        let wrapped = wrap_ansi_line(content, width);

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

fn wrap_ansi_line(line: &str, width: usize) -> Vec<String> {
    if visible_width(line) <= width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut line_buf = String::new();
    let mut line_visible = 0;
    let mut word_buf = String::new();
    let mut word_visible = 0;
    let mut style_prefix = String::new();

    let mut index = 0;
    while index < line.len() {
        if line.as_bytes()[index] == b'\x1b' {
            let escape = read_escape_sequence(&line[index..]);
            if word_visible > 0 {
                word_buf.push_str(&escape);
            } else {
                style_prefix.push_str(&escape);
                line_buf.push_str(&escape);
            }
            index += escape.len();
            continue;
        }

        let ch = line[index..].chars().next().expect("valid utf-8");
        let ch_len = ch.len_utf8();

        if ch.is_whitespace() {
            append_word(
                &mut lines,
                &mut line_buf,
                &mut line_visible,
                &mut word_buf,
                &mut word_visible,
                &style_prefix,
                width,
            );

            if ch == '\n' {
                lines.push(trim_line_end(&line_buf));
                line_buf.clear();
                line_visible = 0;
                style_prefix.clear();
            } else if line_visible > 0 {
                if line_visible + 1 > width {
                    lines.push(trim_line_end(&line_buf));
                    line_buf = style_prefix.clone();
                    line_visible = 0;
                } else {
                    line_buf.push(ch);
                    line_visible += 1;
                }
            }

            index += ch_len;
            continue;
        }

        if word_visible == 0 {
            word_buf.push_str(&style_prefix);
        }

        word_buf.push(ch);
        word_visible += 1;

        if word_visible > width {
            if line_visible > 0 {
                lines.push(trim_line_end(&line_buf));
                line_buf = style_prefix.clone();
                line_visible = 0;
            }

            while word_visible > width {
                let (chunk, rest) = split_visible_prefix(&word_buf, width);
                lines.push(trim_line_end(&chunk));
                word_buf = rest;
                word_visible = visible_width(&word_buf);
                if word_visible > 0 && !word_buf.starts_with('\x1b') {
                    word_buf = format!("{style_prefix}{word_buf}");
                }
            }
        } else if line_visible + word_visible > width && line_visible > 0 {
            lines.push(trim_line_end(&line_buf));
            line_buf = style_prefix.clone();
            line_visible = 0;
        }

        index += ch_len;
    }

    append_word(
        &mut lines,
        &mut line_buf,
        &mut line_visible,
        &mut word_buf,
        &mut word_visible,
        &style_prefix,
        width,
    );

    if !line_buf.is_empty() {
        lines.push(trim_line_end(&line_buf));
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn append_word(
    lines: &mut Vec<String>,
    line_buf: &mut String,
    line_visible: &mut usize,
    word_buf: &mut String,
    word_visible: &mut usize,
    style_prefix: &str,
    width: usize,
) {
    if *word_visible == 0 {
        return;
    }

    if *line_visible + *word_visible > width && *line_visible > 0 {
        lines.push(trim_line_end(line_buf));
        *line_buf = style_prefix.to_owned();
        *line_visible = 0;
    }

    line_buf.push_str(word_buf);
    *line_visible += *word_visible;
    word_buf.clear();
    *word_visible = 0;
}

fn trim_line_end(line: &str) -> String {
    line.trim_end_matches([' ', '\t']).to_string()
}

fn read_escape_sequence(input: &str) -> String {
    let mut index = 1;
    let bytes = input.as_bytes();

    while index < bytes.len() {
        let ch = bytes[index] as char;
        index += 1;
        if ch == 'm' {
            break;
        }
    }

    input[..index.min(input.len())].to_string()
}

fn visible_width(text: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;

    for ch in text.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() || ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        if ch == '\x1b' {
            in_escape = true;
            continue;
        }
        width += 1;
    }

    width
}

fn split_visible_prefix(text: &str, width: usize) -> (String, String) {
    let mut visible = 0;
    let mut index = 0;
    let mut in_escape = false;

    for (byte_index, ch) in text.char_indices() {
        if in_escape {
            if ch.is_ascii_alphabetic() || ch == 'm' {
                in_escape = false;
            }
            index = byte_index + ch.len_utf8();
            continue;
        }

        if ch == '\x1b' {
            in_escape = true;
            index = byte_index + ch.len_utf8();
            continue;
        }

        if visible == width {
            break;
        }

        visible += 1;
        index = byte_index + ch.len_utf8();
    }

    if index == 0 {
        return (String::new(), text.to_string());
    }

    (text[..index].to_string(), text[index..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_width_uses_terminal_when_requested_is_smaller() {
        assert_eq!(effective_wrap_width(80, 120), 120);
    }

    #[test]
    fn effective_width_honors_larger_requested_value() {
        assert_eq!(effective_wrap_width(120, 80), 120);
    }

    #[test]
    fn effective_width_zero_means_terminal() {
        assert_eq!(effective_wrap_width(0, 100), 100);
    }

    #[test]
    fn wraps_plain_text_by_words() {
        let wrapped = wrap_text("one two three four", 8);
        assert_eq!(wrapped, "one two\nthree\nfour");
    }

    #[test]
    fn preserves_ansi_sequences_when_wrapping() {
        let line = format!("\x1b[31mhello\x1b[0m world");
        let wrapped = wrap_ansi_line(&line, 12);
        assert_eq!(wrapped.len(), 1);
        assert!(wrapped[0].contains("\x1b[31m"));
    }

    #[test]
    fn breaks_overlong_words() {
        let wrapped = wrap_text("abcdefghij", 4);
        assert_eq!(wrapped, "abcd\nefgh\nij");
    }
}
