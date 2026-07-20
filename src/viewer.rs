use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::cli::Cli;
use crate::highlight;
use crate::pager;
use crate::wrap;

pub fn run(cli: &Cli) -> io::Result<()> {
    let content = fs::read_to_string(&cli.file).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to read `{}`: {err}", cli.file.display()),
        )
    })?;

    let wrap_width = wrap::effective_wrap_width(cli.column, wrap::terminal_columns());
    let rendered = render(&cli.file, &content, cli.syntax_enabled(), wrap_width)?;

    if cli.page {
        pager::page_output(&rendered)
    } else {
        io::stdout().write_all(rendered.as_bytes())
    }
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
}
