use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_COLUMN: usize = 80;
pub const DEFAULT_SYNTAX: &str = "on";
pub const DEFAULT_TABLE: &str = "on";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_syntax")]
    pub syntax: String,
    #[serde(default = "default_column")]
    pub column: usize,
    #[serde(default)]
    pub page: bool,
    #[serde(default = "default_table")]
    pub table: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            syntax: default_syntax(),
            column: default_column(),
            page: false,
            table: default_table(),
        }
    }
}

fn default_syntax() -> String {
    DEFAULT_SYNTAX.to_string()
}

fn default_column() -> usize {
    DEFAULT_COLUMN
}

fn default_table() -> String {
    DEFAULT_TABLE.to_string()
}

impl Config {
    pub fn path() -> io::Result<PathBuf> {
        config_dir().map(|dir| dir.join("v").join("v.conf"))
    }

    pub fn ensure() -> io::Result<(PathBuf, Config)> {
        let path = Self::path()?;
        if path.is_file() {
            return Self::load(&path).map(|config| (path, config));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config = Config::default();
        write(&path, &config)?;
        Ok((path, config))
    }

    pub fn load(path: &Path) -> io::Result<Config> {
        let contents = fs::read_to_string(path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to read config `{}`: {err}", path.display()),
            )
        })?;

        toml::from_str(&contents)
            .map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to parse config `{}`: {err}", path.display()),
                )
            })
            .and_then(|config| validate(&config).map(|_| config))
    }
}

fn validate(config: &Config) -> io::Result<()> {
    validate_on_off("syntax", &config.syntax)?;
    validate_on_off("table", &config.table)
}

fn validate_on_off(key: &str, value: &str) -> io::Result<()> {
    if value.eq_ignore_ascii_case("on") || value.eq_ignore_ascii_case("off") {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {key} value `{value}` in config: expected `on` or `off`"),
    ))
}

fn config_dir() -> io::Result<PathBuf> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME")
        && !xdg_config_home.is_empty()
    {
        return Ok(PathBuf::from(xdg_config_home));
    }

    let home = std::env::var("HOME").map_err(|err| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("HOME is not set and XDG_CONFIG_HOME is unavailable: {err}"),
        )
    })?;

    Ok(PathBuf::from(home).join(".config"))
}

fn write(path: &Path, config: &Config) -> io::Result<()> {
    let contents = toml::to_string_pretty(config).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize config: {err}"),
        )
    })?;

    fs::write(path, contents).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to write config `{}`: {err}", path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.syntax, "on");
        assert_eq!(config.column, DEFAULT_COLUMN);
        assert!(!config.page);
        assert_eq!(config.table, "on");
    }

    #[test]
    fn round_trips_toml() {
        let config = Config {
            syntax: "off".into(),
            column: 80,
            page: true,
            table: "off".into(),
        };
        let parsed: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn rejects_invalid_syntax_values() {
        let config: Config = toml::from_str("syntax = \"maybe\"\ncolumn = 80\npage = false").unwrap();
        let err = validate(&config).unwrap_err();
        assert!(err.to_string().contains("invalid syntax value"));
    }

    #[test]
    fn rejects_invalid_table_values() {
        let config: Config = toml::from_str("table = \"maybe\"").unwrap();
        let err = validate(&config).unwrap_err();
        assert!(err.to_string().contains("invalid table value"));
    }

    #[test]
    fn table_defaults_to_on_for_existing_config_files() {
        let config: Config = toml::from_str("syntax = \"on\"\ncolumn = 80\npage = false").unwrap();
        assert_eq!(config.table, "on");
    }
}
