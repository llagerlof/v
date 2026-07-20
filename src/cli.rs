use std::path::{Path, PathBuf};

use clap::{ArgAction, ArgMatches, CommandFactory, Parser};

use crate::config::Config;

/// View text files with syntax highlighting and word wrapping.
#[derive(Parser, Debug)]
#[command(name = "v", disable_help_flag = true, disable_version_flag = true)]
pub struct Cli {
    /// File to display.
    #[arg(required = false)]
    pub file: Option<PathBuf>,

    /// Enable or disable syntax highlighting (`on`, `off`, or `0`).
    #[arg(long, value_name = "on|off")]
    pub syntax: Option<String>,

    /// Wrap width in columns (`0` uses the terminal width).
    #[arg(long, value_name = "COLUMNS")]
    pub column: Option<usize>,

    /// Paginate output using `$PAGER` (defaults to `less -R`).
    #[arg(long = "page", action = ArgAction::SetTrue)]
    pub page: bool,

    /// Print help information.
    #[arg(short = 'h', long = "help", action = ArgAction::HelpLong)]
    help: bool,

    /// Print version information.
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: bool,
}

pub struct ResolvedCli {
    pub file: PathBuf,
    pub syntax: String,
    pub column: usize,
    pub page: bool,
}

pub fn build_command(config: &Config, config_path: &Path) -> clap::Command {
    Cli::command()
        .about("View text files with syntax highlighting and word wrapping.")
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(help_footer(config, config_path))
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

pub fn help_footer(config: &Config, config_path: &Path) -> String {
    format!(
        "Version: {}\nConfig: {}\n\nConfig defaults:\n  syntax = {}\n  column = {}\n  page = {}",
        env!("CARGO_PKG_VERSION"),
        config_path.display(),
        config.syntax,
        config.column,
        config.page,
    )
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
    fn help_footer_includes_version_and_config_path() {
        let config = Config::default();
        let footer = help_footer(&config, Path::new("/home/user/.config/v/v.conf"));
        assert!(footer.contains(env!("CARGO_PKG_VERSION")));
        assert!(footer.contains("/home/user/.config/v/v.conf"));
        assert!(footer.contains("column = 100"));
    }

    #[test]
    fn resolve_uses_config_when_flags_are_not_set() {
        let config = Config {
            syntax: "off".into(),
            column: 80,
            page: true,
        };
        let matches = build_command(&config, Path::new("/tmp/v.conf"))
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
        let matches = build_command(&config, Path::new("/tmp/v.conf"))
            .try_get_matches_from(["v", "--syntax=off", "--column=72", "--page", "file.txt"])
            .unwrap();

        let resolved = resolve(&matches, &config);
        assert_eq!(resolved.syntax, "off");
        assert_eq!(resolved.column, 72);
        assert!(resolved.page);
    }
}
