use std::{process::Command, sync::Arc};

use crate::{
    backends::BackendError,
    theme::{Theme, ThemeBackend},
};

const SCHEMA: &str = "org.gnome.desktop.interface";
const KEY: &str = "color-scheme";

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, BackendError>;
}

#[derive(Debug, Default)]
struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, BackendError> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|source| BackendError::Command {
                program: program.to_owned(),
                source,
            })?;

        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// GNOME implementation backed by the `gsettings` command.
#[derive(Clone)]
pub struct GnomeBackend {
    runner: Arc<dyn CommandRunner>,
}

impl Default for GnomeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GnomeBackend {
    /// Creates a backend that invokes the system `gsettings` executable.
    pub fn new() -> Self {
        Self {
            runner: Arc::new(SystemCommandRunner),
        }
    }

    fn set_color_scheme(&self, value: &str) -> Result<(), BackendError> {
        let output = self.runner.run("gsettings", &["set", SCHEMA, KEY, value])?;
        if output.success {
            Ok(())
        } else {
            Err(BackendError::CommandFailed {
                program: "gsettings".to_owned(),
                message: output.stderr.trim().to_owned(),
            })
        }
    }

    fn parse_theme(value: &str) -> Result<Theme, BackendError> {
        match value.trim().trim_matches('\'') {
            "prefer-dark" => Ok(Theme::Dark),
            "default" => Ok(Theme::Light),
            other => Err(BackendError::UnsupportedTheme(other.to_owned())),
        }
    }
}

impl ThemeBackend for GnomeBackend {
    fn set_dark(&self) -> Result<(), BackendError> {
        self.set_color_scheme("prefer-dark")
    }

    fn set_light(&self) -> Result<(), BackendError> {
        self.set_color_scheme("default")
    }

    fn current_theme(&self) -> Result<Theme, BackendError> {
        let output = self.runner.run("gsettings", &["get", SCHEMA, KEY])?;
        if !output.success {
            return Err(BackendError::CommandFailed {
                program: "gsettings".to_owned(),
                message: output.stderr.trim().to_owned(),
            });
        }
        Self::parse_theme(&output.stdout)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use super::*;

    struct FakeRunner(Mutex<VecDeque<CommandOutput>>);

    impl CommandRunner for FakeRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<CommandOutput, BackendError> {
            Ok(self.0.lock().unwrap().pop_front().unwrap())
        }
    }

    fn backend_with(stdout: &str) -> GnomeBackend {
        GnomeBackend {
            runner: Arc::new(FakeRunner(Mutex::new(VecDeque::from([CommandOutput {
                success: true,
                stdout: stdout.to_owned(),
                stderr: String::new(),
            }])))),
        }
    }

    #[test]
    fn detects_dark_theme_from_gsettings_output() {
        assert_eq!(
            backend_with("'prefer-dark'\n").current_theme().unwrap(),
            Theme::Dark
        );
    }

    #[test]
    fn detects_light_theme_from_gsettings_output() {
        assert_eq!(
            backend_with("'default'\n").current_theme().unwrap(),
            Theme::Light
        );
    }

    #[test]
    fn rejects_unknown_gsettings_value() {
        assert!(matches!(
            backend_with("'unexpected'").current_theme(),
            Err(BackendError::UnsupportedTheme(_))
        ));
    }
}
