use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::cli::ResolvedCli;
use crate::highlight;
use crate::pager;
use crate::wrap;

pub fn run(cli: &ResolvedCli) -> io::Result<()> {
    let content = read_text_file(&cli.file)?;

    let wrap_width = wrap::effective_wrap_width(cli.column, wrap::terminal_columns());
    let rendered = render(&cli.file, &content, cli.syntax_enabled(), wrap_width)?;

    if cli.page {
        pager::page_output(&rendered)
    } else {
        io::stdout().write_all(rendered.as_bytes())
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

fn render(path: &Path, content: &str, syntax_enabled: bool, wrap_width: usize) -> io::Result<String> {
    let wrapped = wrap::wrap_plain_text(content, wrap_width);

    if syntax_enabled {
        highlight::highlight_file(path, &wrapped)
    } else {
        Ok(wrapped)
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
            120,
        )
        .unwrap();

        assert_eq!(rendered, "hello world");
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
