use clap::Parser;
use std::path::PathBuf;

/// View text files with syntax highlighting and word wrapping.
#[derive(Parser, Debug)]
#[command(name = "v", version, about)]
pub struct Cli {
    /// File to display.
    pub file: PathBuf,

    /// Enable or disable syntax highlighting (`on`, `off`, or `0`).
    #[arg(long, default_value = "on", value_name = "on|off")]
    pub syntax: String,

    /// Wrap width in columns (`0` uses the terminal width).
    #[arg(long, default_value = "120", value_name = "COLUMNS")]
    pub column: usize,

    /// Paginate output using `$PAGER` (defaults to `less -R`).
    #[arg(long = "page")]
    pub page: bool,
}

impl Cli {
    pub fn syntax_enabled(&self) -> bool {
        !matches!(
            self.syntax.to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_flag_parses_off_values() {
        let cli = Cli {
            file: PathBuf::from("test.rs"),
            syntax: "off".into(),
            column: 120,
            page: false,
        };
        assert!(!cli.syntax_enabled());

        let cli = Cli {
            syntax: "0".into(),
            ..cli
        };
        assert!(!cli.syntax_enabled());
    }

    #[test]
    fn syntax_flag_defaults_on() {
        let cli = Cli {
            file: PathBuf::from("test.rs"),
            syntax: "on".into(),
            column: 120,
            page: false,
        };
        assert!(cli.syntax_enabled());
    }
}
