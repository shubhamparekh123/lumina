use std::{fs, io, path::{Path, PathBuf}};

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{theme::Theme, utils::paths};

/// Automation strategy selected in the configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleMode {
    Time,
    Sunrise,
    Sunset,
    Battery,
    Weather,
    Rules,
}

impl std::fmt::Display for ScheduleMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Time => "Time",
            Self::Sunrise => "Sunrise",
            Self::Sunset => "Sunset",
            Self::Battery => "Battery",
            Self::Weather => "Weather",
            Self::Rules => "Rules",
        };
        formatter.write_str(name)
    }
}

/// Persistent Lumina user preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: Theme,
    #[serde(rename = "auto")]
    pub automation: bool,
    pub mode: ScheduleMode,
    pub light_time: String,
    pub dark_time: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            automation: true,
            mode: ScheduleMode::Time,
            light_time: "07:00".to_owned(),
            dark_time: "18:30".to_owned(),
        }
    }
}

impl AppConfig {
    /// Parses and validates a TOML configuration document.
    pub fn parse(contents: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Ensures configured clock times are valid and non-ambiguous.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let light = self.light_time()?;
        let dark = self.dark_time()?;
        if light == dark {
            return Err(ConfigError::EqualScheduleTimes);
        }
        Ok(())
    }

    pub fn light_time(&self) -> Result<NaiveTime, ConfigError> {
        parse_time("light_time", &self.light_time)
    }

    pub fn dark_time(&self) -> Result<NaiveTime, ConfigError> {
        parse_time("dark_time", &self.dark_time)
    }
}

fn parse_time(field: &'static str, value: &str) -> Result<NaiveTime, ConfigError> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| ConfigError::InvalidTime {
        field,
        value: value.to_owned(),
    })
}

/// Reads and writes the Lumina configuration file.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// Uses `~/.config/lumina/config.toml` on Linux.
    pub fn standard() -> Result<Self, ConfigError> {
        Ok(Self::at(paths::config_file()?))
    }

    /// Creates a store at an explicit path, primarily for tests and embedding.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the config, atomically creating a default file if it is missing.
    pub fn load_or_create(&self) -> Result<AppConfig, ConfigError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => AppConfig::parse(&contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let config = AppConfig::default();
                self.save(&config)?;
                Ok(config)
            }
            Err(source) => Err(ConfigError::Read {
                path: self.path.clone(),
                source,
            }),
        }
    }

    /// Persists validated configuration using a temporary file and rename.
    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        config.validate()?;
        let parent = self.path.parent().ok_or_else(|| ConfigError::InvalidPath(self.path.clone()))?;
        fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
        let contents = toml::to_string_pretty(config)?;
        let temporary = self.path.with_extension("toml.tmp");
        fs::write(&temporary, contents).map_err(|source| ConfigError::Write {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &self.path).map_err(|source| ConfigError::Write {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not locate the user configuration directory")]
    MissingConfigDirectory,
    #[error("invalid configuration path: {0}")]
    InvalidPath(PathBuf),
    #[error("could not read configuration {path}: {source}")]
    Read { path: PathBuf, #[source] source: io::Error },
    #[error("could not write configuration {path}: {source}")]
    Write { path: PathBuf, #[source] source: io::Error },
    #[error("invalid TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not serialize configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("{field} must use 24-hour HH:MM format, got {value:?}")]
    InvalidTime { field: &'static str, value: String },
    #[error("light_time and dark_time must be different")]
    EqualScheduleTimes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_configuration() {
        let config = AppConfig::parse(
            "theme = \"dark\"\nauto = true\nmode = \"time\"\nlight_time = \"07:00\"\ndark_time = \"18:30\"\n",
        ).unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn fills_future_compatible_missing_fields_with_defaults() {
        let config = AppConfig::parse("auto = false\n").unwrap();
        assert!(!config.automation);
        assert_eq!(config.light_time, "07:00");
    }

    #[test]
    fn rejects_invalid_time() {
        assert!(matches!(
            AppConfig::parse("light_time = \"25:00\""),
            Err(ConfigError::InvalidTime { field: "light_time", .. })
        ));
    }

    #[test]
    fn creates_default_config_when_missing() {
        let directory = tempfile::tempdir().unwrap();
        let store = ConfigStore::at(directory.path().join("lumina/config.toml"));
        assert_eq!(store.load_or_create().unwrap(), AppConfig::default());
        assert!(store.path().exists());
    }
}

