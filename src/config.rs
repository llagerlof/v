use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_COLUMN: usize = 100;
pub const DEFAULT_SYNTAX: &str = "on";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_syntax")]
    pub syntax: String,
    #[serde(default = "default_column")]
    pub column: usize,
    #[serde(default)]
    pub page: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            syntax: default_syntax(),
            column: default_column(),
            page: false,
        }
    }
}

fn default_syntax() -> String {
    DEFAULT_SYNTAX.to_string()
}

fn default_column() -> usize {
    DEFAULT_COLUMN
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

        toml::from_str(&contents).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse config `{}`: {err}", path.display()),
            )
        })
    }
}

fn config_dir() -> io::Result<PathBuf> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.is_empty() {
            return Ok(PathBuf::from(xdg_config_home));
        }
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
    }

    #[test]
    fn round_trips_toml() {
        let config = Config {
            syntax: "off".into(),
            column: 80,
            page: true,
        };
        let parsed: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(parsed, config);
    }
}
