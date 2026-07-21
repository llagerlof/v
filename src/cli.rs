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

    /// Enable or disable syntax highlighting (default from config).
    #[arg(
        short = 's',
        long,
        value_name = "on|off",
        value_parser = ["on", "off"],
        default_missing_value = "on",
        num_args = 0..=1
    )]
    pub syntax: Option<String>,

    /// Enable or disable pagination using `$PAGER` (default from config).
    #[arg(
        short = 'p',
        long = "page",
        value_name = "on|off",
        value_parser = ["on", "off"],
        default_missing_value = "on",
        num_args = 0..=1
    )]
    pub page: Option<String>,

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

fn is_on_off_value(arg: &str) -> bool {
    arg.eq_ignore_ascii_case("on") || arg.eq_ignore_ascii_case("off")
}

fn is_optional_on_off_flag(arg: &str) -> bool {
    matches!(arg, "-p" | "--page" | "-s" | "--syntax")
}

/// Inserts a default `on` value after bare `-p` / `--page` / `-s` / `--syntax`
/// when not followed by `on` or `off`.
pub fn normalize_optional_value_args(args: &[String]) -> Vec<String> {
    if args.len() <= 1 {
        return args.to_vec();
    }

    let mut normalized = Vec::with_capacity(args.len() + 1);
    normalized.push(args[0].clone());

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        normalized.push(arg.clone());

        if is_optional_on_off_flag(arg) {
            let insert_on = match args.get(i + 1) {
                None => true,
                Some(next) if next.starts_with('-') => true,
                Some(next) if is_on_off_value(next) => false,
                Some(_) => true,
            };

            if insert_on {
                normalized.push("on".into());
            }
        }

        i += 1;
    }

    normalized
}

pub fn parse_matches() -> Result<ArgMatches, clap::Error> {
    let args: Vec<String> = std::env::args().collect();
    parse_matches_from(args)
}

pub fn parse_matches_from<I, T>(args: I) -> Result<ArgMatches, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    build_command().try_get_matches_from(normalize_optional_value_args(&args))
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
    let page = match matches.get_one::<String>("page") {
        Some(value) => value.eq_ignore_ascii_case("on"),
        None => config.page,
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
        !self.syntax.eq_ignore_ascii_case("off")
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
                         Wrap width in columns (default 80; 0 uses the terminal width)
  -s, --syntax <on|off>  Enable or disable syntax highlighting (default from config; bare `-s` is `on`)
  -p, --page <on|off>    Enable or disable pagination using `$PAGER` (default from config; bare `-p` is `on`)
  -h, --help             Print help information
  -v, --version          Print version information

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
    fn syntax_flag_parses_off_value() {
        let cli = ResolvedCli {
            file: PathBuf::from("test.rs"),
            syntax: "off".into(),
            column: DEFAULT_COLUMN,
            page: false,
        };
        assert!(!cli.syntax_enabled());
    }

    #[test]
    fn syntax_flag_rejects_invalid_values() {
        let err = build_command()
            .try_get_matches_from(["v", "-s", "0", "file.txt"])
            .unwrap_err();
        assert!(err.to_string().contains("invalid value"));
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
        let matches = parse_matches_from(["v", "-s", "off", "-w", "72", "-p", "on", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.syntax, "off");
        assert_eq!(resolved.column, 72);
        assert!(resolved.page);
    }

    #[test]
    fn page_off_overrides_config() {
        let config = Config {
            page: true,
            ..Config::default()
        };
        let matches = parse_matches_from(["v", "-p", "off", "file.txt"]).unwrap();

        let resolved = resolve(&matches, &config);
        assert!(!resolved.page);
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
    fn page_flag_accepts_on_and_off() {
        let config = Config::default();
        let matches = parse_matches_from(["v", "-p", "on", "file.txt"]).unwrap();
        assert!(resolve(&matches, &config).page);

        let matches = parse_matches_from(["v", "--page=off", "file.txt"]).unwrap();
        assert!(!resolve(&matches, &config).page);
    }

    #[test]
    fn page_flag_defaults_to_on_without_argument() {
        let config = Config::default();
        let matches = parse_matches_from(["v", "-p", "file.txt"]).unwrap();

        assert!(resolve(&matches, &config).page);

        let matches = parse_matches_from(["v", "--page", "file.txt"]).unwrap();
        assert!(resolve(&matches, &config).page);
    }

    #[test]
    fn normalize_optional_value_args_inserts_on_before_file() {
        assert_eq!(
            normalize_optional_value_args(&["v".into(), "-p".into(), "file.txt".into()]),
            vec!["v", "-p", "on", "file.txt"]
        );
        assert_eq!(
            normalize_optional_value_args(&["v".into(), "-s".into(), "file.txt".into()]),
            vec!["v", "-s", "on", "file.txt"]
        );
    }

    #[test]
    fn syntax_flag_defaults_to_on_without_argument() {
        let config = Config {
            syntax: "off".into(),
            ..Config::default()
        };
        let matches = parse_matches_from(["v", "-s", "file.txt"]).unwrap();

        assert!(resolve(&matches, &config).syntax_enabled());

        let matches = parse_matches_from(["v", "--syntax", "file.txt"]).unwrap();
        assert!(resolve(&matches, &config).syntax_enabled());
    }
}
