use std::io;
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

const DEFAULT_THEME: &str = "base16-ocean.dark";

/// Highlight file contents based on the file extension.
pub fn highlight_file(path: &Path, content: &str) -> io::Result<String> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let syntax = match syntax_set.find_syntax_by_extension(extension) {
        Some(syntax) => syntax,
        None => return Ok(content.to_string()),
    };

    let theme = theme_set
        .themes
        .get(DEFAULT_THEME)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "default theme not found"))?;

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut output = String::with_capacity(content.len() + content.len() / 4);

    for line in LinesWithEndings::from(content) {
        let ranges = highlighter
            .highlight_line(line, &syntax_set)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        output.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn highlights_rust_source() {
        let source = "fn main() {\n    println!(\"hi\");\n}\n";
        let highlighted = highlight_file(Path::new("main.rs"), source).unwrap();
        assert!(highlighted.contains('\x1b'));
        assert!(highlighted.contains("fn"));
        assert!(highlighted.contains("main"));
    }

    #[test]
    fn unknown_extension_returns_plain_text() {
        let source = "plain text\n";
        let highlighted = highlight_file(Path::new("file.unknownext"), source).unwrap();
        assert_eq!(highlighted, source);
    }
}
