use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
    sync::Mutex,
};

use tracing_subscriber::EnvFilter;

/// Initializes human-readable stderr logging for interactive commands.
pub fn init_console() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(default_filter())
        .with_target(false)
        .try_init();
}

/// Initializes append-only daemon logging at the given path.
pub fn init_file(path: &Path) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file: File = OpenOptions::new().create(true).append(true).open(path)?;
    tracing_subscriber::fmt()
        .with_env_filter(default_filter())
        .with_ansi(false)
        .with_target(false)
        .with_writer(Mutex::new(file))
        .try_init()
        .map_err(io::Error::other)
}

fn default_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("lumina=info"))
}
