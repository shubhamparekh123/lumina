use notify_rust::Notification;
use thiserror::Error;

use crate::theme::Theme;

/// Desktop-independent notification port.
pub trait NotificationService: Send + Sync {
    fn theme_changed(&self, theme: Theme) -> Result<(), NotificationError>;
}

/// Freedesktop notification implementation (libnotify-compatible).
#[derive(Debug, Default, Clone, Copy)]
pub struct LibnotifyService;

impl NotificationService for LibnotifyService {
    fn theme_changed(&self, theme: Theme) -> Result<(), NotificationError> {
        Notification::new()
            .summary("Lumina")
            .body(&format!("Switched to {theme} mode"))
            .icon("preferences-desktop-theme")
            .show()
            .map(|_| ())
            .map_err(NotificationError::Show)
    }
}

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("could not show desktop notification: {0}")]
    Show(#[source] notify_rust::error::Error),
}
