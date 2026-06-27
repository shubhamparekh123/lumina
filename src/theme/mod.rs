use std::fmt;

use serde::{Deserialize, Serialize};

use crate::backends::BackendError;

/// A desktop color theme supported by Lumina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

impl fmt::Display for Theme {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Light => formatter.write_str("Light"),
            Self::Dark => formatter.write_str("Dark"),
        }
    }
}

/// Desktop-independent theme operations.
pub trait ThemeBackend: Send + Sync {
    /// Switches the desktop to dark mode.
    fn set_dark(&self) -> Result<(), BackendError>;

    /// Switches the desktop to light mode.
    fn set_light(&self) -> Result<(), BackendError>;

    /// Returns the currently active color theme.
    fn current_theme(&self) -> Result<Theme, BackendError>;

    /// Switches to the opposite of the current theme.
    fn toggle(&self) -> Result<Theme, BackendError> {
        let target = match self.current_theme()? {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        self.set_theme(target)?;
        Ok(target)
    }

    /// Applies a specific theme.
    fn set_theme(&self, theme: Theme) -> Result<(), BackendError> {
        match theme {
            Theme::Light => self.set_light(),
            Theme::Dark => self.set_dark(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct MemoryBackend(Mutex<Theme>);

    impl ThemeBackend for MemoryBackend {
        fn set_dark(&self) -> Result<(), BackendError> {
            *self.0.lock().expect("test mutex poisoned") = Theme::Dark;
            Ok(())
        }

        fn set_light(&self) -> Result<(), BackendError> {
            *self.0.lock().expect("test mutex poisoned") = Theme::Light;
            Ok(())
        }

        fn current_theme(&self) -> Result<Theme, BackendError> {
            Ok(*self.0.lock().expect("test mutex poisoned"))
        }
    }

    #[test]
    fn default_toggle_uses_only_trait_operations() {
        let backend = MemoryBackend(Mutex::new(Theme::Light));
        assert_eq!(backend.toggle().unwrap(), Theme::Dark);
        assert_eq!(backend.current_theme().unwrap(), Theme::Dark);
    }
}

