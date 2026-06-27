//! Desktop-environment backend implementations.

pub mod gnome;

use std::io;

use thiserror::Error;

pub use gnome::GnomeBackend;

/// Failures reported by desktop theme backends.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("could not run {program}: {source}")]
    Command {
        program: String,
        #[source]
        source: io::Error,
    },
    #[error("{program} failed: {message}")]
    CommandFailed { program: String, message: String },
    #[error("desktop returned an unsupported color scheme: {0}")]
    UnsupportedTheme(String),
}

