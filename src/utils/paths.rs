use std::path::PathBuf;

use crate::config::ConfigError;

pub fn config_file() -> Result<PathBuf, ConfigError> {
    dirs::config_dir()
        .map(|path| path.join("lumina").join("config.toml"))
        .ok_or(ConfigError::MissingConfigDirectory)
}

pub fn state_dir() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|path| path.join("lumina"))
}

