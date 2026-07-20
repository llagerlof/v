use std::path::{Path, PathBuf};

use clap::{ArgAction, ArgMatches, CommandFactory, Parser};

use crate::config::Config;

#[derive(Parser, Debug)]
#[command(name = "v", disable_help_flag = true, disable_version_flag = true)]
pub struct Cli {
    /// File to display.
    #[arg(required = false)]
    pub file: Option<PathBuf>,

    /// Wrap width in columns (0 uses the terminal width).
    #[arg(
        short = 'c',
        visible_short_alias = 'w',
        long = "column",
        visible_alias = "width",
        value_name = "COLUMNS"
    )]
    pub column: Option<usize>,

    /// Enable or disable syntax highlighting (on, off or 0 - default on).
    #[arg(short = 's', long, value_name = "on|off")]
    pub syntax: Option<String>,

    /// Paginate output using `$PAGER` (defaults to `less -R`).
    #[arg(short = 'p', long = "page", action = ArgAction::SetTrue)]
    pub page: bool,

    /// Print help information.
    #[arg(short = 'h', long = "help", action = ArgAction::SetTrue)]
    pub help: bool,

    /// Print version information.
    #[arg(
        short = 'v',
        visible_short_alias = 'V',
        long = "version",
        action = ArgAction::SetTrue
    )]
    pub version: bool,
}

pub struct ResolvedCli {
    pub file: PathBuf,
    pub syntax: String,
    pub column: usize,
    pub page: bool,
}

pub fn build_command() -> clap::Command {
    Cli::command()
}

pub fn resolve(matches: &ArgMatches, config: &Config) -> ResolvedCli {
    let syntax = matches
        .get_one::<String>("syntax")
        .cloned()
        .unwrap_or_else(|| config.syntax.clone());
    let column = matches
        .get_one::<usize>("column")
        .copied()
        .unwrap_or(config.column);
    let page = if matches.get_flag("page") {
        true
    } else {
        config.page
    };

    ResolvedCli {
        file: matches
            .get_one::<PathBuf>("file")
            .expect("file is required for resolved CLI")
            .clone(),
        syntax,
        column,
        page,
    }
}

impl ResolvedCli {
    pub fn syntax_enabled(&self) -> bool {
        !matches!(
            self.syntax.to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        )
    }
}

pub fn format_config_path(config_path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(relative) = config_path.strip_prefix(home_path) {
            return format!("~/{}", relative.display());
        }
    }

    config_path.display().to_string()
}

pub fn format_help(config_path: &Path) -> String {
    format!(
        "\
v v{version}
A cli tool to view text files with word wrapping and syntax highlighting.

Usage example:
$ v history.md

Usage syntax:
v [OPTIONS] [FILE]

Arguments:
  [FILE]  File to display

Options:
  -c, -w, --column, --width <COLUMNS>
                        Wrap width in columns (0 uses the terminal width)
  -s, --syntax <on|off>   Enable or disable syntax highlighting (on, off or 0 - default on)
  -p, --page              Paginate output using `$PAGER` (defaults to `less -R`)
  -h, --help              Print help information
  -v, --version           Print version information

Configuration file: {config_path}

    \"Enjoy the read!\"
        ",
        version = env!("CARGO_PKG_VERSION"),
        config_path = format_config_path(config_path),
    )
}

pub fn format_version() -> String {
    format!("v v{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DEFAULT_COLUMN, DEFAULT_SYNTAX};

    #[test]
    fn syntax_flag_parses_off_values() {
        let cli = ResolvedCli {
            file: PathBuf::from("test.rs"),
            syntax: "off".into(),
            column: DEFAULT_COLUMN,
            page: false,
        };
        assert!(!cli.syntax_enabled());

        let cli = ResolvedCli {
            syntax: "0".into(),
            ..cli
        };
        assert!(!cli.syntax_enabled());
    }

    #[test]
    fn syntax_flag_defaults_on() {
        let cli = ResolvedCli {
            file: PathBuf::from("test.rs"),
            syntax: DEFAULT_SYNTAX.into(),
            column: DEFAULT_COLUMN,
            page: false,
        };
        assert!(cli.syntax_enabled());
    }

    #[test]
    fn help_includes_version_and_config_path() {
        let help = format_help(Path::new("/home/user/.config/v/v.conf"));
        assert!(help.starts_with(&format!("v v{}", env!("CARGO_PKG_VERSION"))));
        assert!(help.contains("Configuration file:"));
        assert!(help.contains("Enjoy the read!"));
        assert!(help.contains("Usage example:"));
        assert!(help.contains("-v, --version"));
        assert!(help.contains("-p, --page"));
        assert!(help.contains("-s, --syntax"));
        assert!(help.contains("-c, -w, --column, --width"));
    }

    #[test]
    fn format_config_path_expands_home_directory() {
        let home = std::env::var("HOME").expect("HOME must be set for this test");
        let config_path = PathBuf::from(&home).join(".config/v/v.conf");
        assert_eq!(format_config_path(&config_path), "~/.config/v/v.conf");
    }

    #[test]
    fn format_config_path_leaves_non_home_paths_unchanged() {
        let path = format_config_path(Path::new("/var/custom/v/v.conf"));
        assert_eq!(path, "/var/custom/v/v.conf");
    }

    #[test]
    fn version_flag_accepts_lowercase_and_uppercase_short_aliases() {
        let matches = build_command()
            .try_get_matches_from(["v", "-v"])
            .unwrap();
        assert!(matches.get_flag("version"));

        let matches = build_command()
            .try_get_matches_from(["v", "-V"])
            .unwrap();
        assert!(matches.get_flag("version"));
    }

    #[test]
    fn resolve_uses_config_when_flags_are_not_set() {
        let config = Config {
            syntax: "off".into(),
            column: 80,
            page: true,
        };
        let matches = build_command()
            .try_get_matches_from(["v", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.syntax, "off");
        assert_eq!(resolved.column, 80);
        assert!(resolved.page);
    }

    #[test]
    fn resolve_prefers_command_line_over_config() {
        let config = Config::default();
        let matches = build_command()
            .try_get_matches_from(["v", "-s", "off", "-w", "72", "-p", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.syntax, "off");
        assert_eq!(resolved.column, 72);
        assert!(resolved.page);
    }

    #[test]
    fn width_alias_matches_column_flag() {
        let config = Config::default();
        let matches = build_command()
            .try_get_matches_from(["v", "--width=42", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.column, 42);
    }

    #[test]
    fn short_width_alias_matches_column_flag() {
        let config = Config::default();
        let matches = build_command()
            .try_get_matches_from(["v", "-w", "42", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.column, 42);
    }

    #[test]
    fn page_flag_accepts_short_form() {
        let matches = build_command()
            .try_get_matches_from(["v", "-p", "file.txt"])
            .unwrap();

        assert!(matches.get_flag("page"));
    }
}
