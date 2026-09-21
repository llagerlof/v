use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::cli::ResolvedCli;
use crate::emphasis;
use crate::highlight;
use crate::markdown;
use crate::pager;
use crate::wrap;

pub fn run(cli: &ResolvedCli) -> io::Result<()> {
    let content = read_text_file(&cli.file)?;

    let wrap_width = wrap::effective_wrap_width(cli.column, wrap::terminal_columns());
    let rendered = render(
        &cli.file,
        &content,
        cli.syntax_enabled(),
        cli.table_enabled(),
        wrap_width,
    )?;

    if cli.page {
        pager::page_output(&rendered)
    } else {
        let mut stdout = io::stdout();
        stdout.write_all(rendered.as_bytes())?;
        if !rendered.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
        Ok(())
    }
}

fn read_text_file(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to read `{}`: {err}", path.display()),
        )
    })?;

    if bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` appears to be a binary file", path.display()),
        ));
    }

    String::from_utf8(bytes).map_err(|_err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` is not valid UTF-8 text", path.display()),
        )
    })
}

fn render(
    path: &Path,
    content: &str,
    syntax_enabled: bool,
    table_enabled: bool,
    wrap_width: usize,
) -> io::Result<String> {
    // Tables are laid out to fit `wrap_width`, so word wrapping leaves them intact.
    let content = if table_enabled && markdown::is_markdown_path(path) {
        markdown::format_tables(content, wrap_width)
    } else {
        content.to_string()
    };

    let wrapped = wrap::wrap_plain_text(&content, wrap_width);

    let styled = if syntax_enabled {
        highlight::highlight_file(path, &wrapped)?
    } else {
        wrapped
    };

    // Strong emphasis is styled last, so it survives highlighting and adds only
    // zero-width escapes to already wrapped lines.
    if markdown::is_markdown_path(path) {
        Ok(emphasis::style_strong(&styled))
    } else {
        Ok(styled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_content_without_highlighting() {
        let rendered = render(
            Path::new("example.txt"),
            "hello world",
            false,
            true,
            120,
        )
        .unwrap();

        assert_eq!(rendered, "hello world");
    }

    #[test]
    fn formats_markdown_tables_before_wrapping() {
        let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let rendered = render(Path::new("doc.md"), source, false, true, 80).unwrap();

        assert_eq!(
            rendered,
            "+---+---+\n| a | b |\n+---+---+\n| 1 | 2 |\n+---+---+\n"
        );
    }

    #[test]
    fn leaves_markdown_tables_alone_when_disabled() {
        let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let rendered = render(Path::new("doc.md"), source, false, false, 80).unwrap();

        assert_eq!(rendered, source);
    }

    #[test]
    fn does_not_format_tables_in_non_markdown_files() {
        let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let rendered = render(Path::new("notes.txt"), source, false, true, 80).unwrap();

        assert_eq!(rendered, source);
    }

    #[test]
    fn rejects_binary_files() {
        let path = std::env::temp_dir().join("v-binary-test.bin");
        fs::write(&path, b"text\x00binary").unwrap();
        let err = read_text_file(&path).unwrap_err();
        assert!(err.to_string().contains("binary file"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_utf8_text() {
        let path = std::env::temp_dir().join("v-invalid-utf8-test.txt");
        fs::write(&path, &[0xFF, 0xFE, b'a', b'b']).unwrap();
        let err = read_text_file(&path).unwrap_err();
        assert!(err.to_string().contains("not valid UTF-8 text"));
        let _ = fs::remove_file(path);
    }
}
